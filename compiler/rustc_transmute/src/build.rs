//! Build NFA describing a given type
#![allow(dead_code)]
use core::alloc::{Layout, LayoutError};

use crate::debug::DebugEntry;
use crate::prog::*;

use rustc_middle::ty::TyCtxt;
use rustc_middle::ty::{subst::SubstsRef, AdtDef, Ty, VariantDef};

type Result<'tcx, T = ()> = core::result::Result<T, BuilderError<'tcx>>;

fn layout_of<'tcx>(ctx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Result<'tcx, Layout> {
    use rustc_middle::ty::{ParamEnv, ParamEnvAnd};
    use rustc_target::abi::TyAndLayout;

    let param_env = ParamEnv::reveal_all();
    let param_env_and_type = ParamEnvAnd { param_env, value: ty };
    let TyAndLayout { layout, .. } = ctx.layout_of(param_env_and_type)
        .map_err(|_| BuilderError::LayoutOverflow)?;
    let layout = Layout::from_size_align(
        layout.size().bytes_usize(),
        layout.align().abi.bytes().try_into().unwrap(),
    )?;
    Ok(layout)
}

pub enum BuilderError<'tcx> {
    NonReprC(Ty<'tcx>),
    TypeNotSupported(Ty<'tcx>),
    LayoutOverflow,
    NfaTooLarge,
}

impl<'a> core::convert::From<LayoutError> for BuilderError<'a> {
    fn from(_err: LayoutError) -> Self {
        BuilderError::LayoutOverflow
    }
}

const MAX_NFA_SIZE: usize = u32::max_value() as usize;

pub struct NfaBuilder<'tcx, R: Clone> {
    pub layout: Layout,
    pub insts: Vec<Inst<R>>,
    pub priv_depth: usize,
    pub debug: Vec<DebugEntry<Ty<'tcx>>>,
    pub debug_parent: usize,
    pub tcx: TyCtxt<'tcx>,
    pub scope: Ty<'tcx>,
}

impl<'tcx> NfaBuilder<'tcx, Ty<'tcx>> {
    pub fn new(tcx: TyCtxt<'tcx>, scope: Ty<'tcx>) -> Self {
        Self {
            layout: Layout::from_size_align(0, 1)
                .expect("This layout should always succeed"),
            insts: Vec::new(),
            priv_depth: 0,
            debug: Vec::new(),
            debug_parent: 0,
            tcx,
            scope,
        }
    }

    pub fn build_ty(
        tcx: TyCtxt<'tcx>,
        scope: Ty<'tcx>,
        ty: Ty<'tcx>,
    ) -> Result<'tcx, Program<Ty<'tcx>>> {
        let mut builder = Self::new(tcx, scope);

        builder.debug.push(DebugEntry::Root { ip: 0, ty });
        builder.debug_parent = 0;

        builder.extend_from_ty(ty)?;
        builder.push(Inst::Accept)?;
        Ok(Program::new(builder.insts, builder.debug, builder.layout.size()))
    }

    fn debug_enter<F>(&mut self, f: F)
    where
        F: Fn(InstPtr, usize) -> DebugEntry<Ty<'tcx>>,
    {
        let ip = self.insts.len() as InstPtr;
        let parent = self.debug_parent;
        self.debug_parent = self.debug.len();
        self.debug.push(f(ip, parent));
    }

    fn debug_exit(&mut self) {
        let parent = self.debug[self.debug_parent].parent_id()
            .expect("debug_exit on root is invalid");
        self.debug_parent = parent;
    }

    fn extend_from_ty(&mut self, ty: Ty<'tcx>) -> Result<'tcx> {
        use rustc_middle::ty::FloatTy::*;
        use rustc_middle::ty::IntTy::*;
        use rustc_middle::ty::ParamEnv;
        use rustc_middle::ty::TyKind::*;
        use rustc_middle::ty::UintTy::*;
        use rustc_target::abi::HasDataLayout;

        let tcx = self.tcx;
        let target = tcx.data_layout();
        let layout = layout_of(self.tcx, ty)?;
        self.layout = self.layout.align_to(layout.align())?;
        self.pad_to_align(layout.align())?;

        match ty.kind() {
            Bool => {
                self.repeat_byte(1, (0..=1).into())?;
                self.layout = self.layout.extend(layout)?.0;
                Ok(())
            }
            Int(I8) | Uint(U8) => self.number(1, layout),
            Int(I16) | Uint(U16) => self.number(2, layout),
            Int(I32) | Uint(U32) | Float(F32) => self.number(4, layout),
            Int(I64) | Uint(U64) | Float(F64) => self.number(8, layout),
            Int(I128) | Uint(U128) => self.number(16, layout),
            Int(Isize) | Uint(Usize) => self.number(target.pointer_size.bytes_usize() as _, layout),
            &Array(ty, size) => {
                self.debug_enter(|ip, parent| DebugEntry::EnterArray { ip, parent, ty });

                for _index in 0..size.eval_usize(tcx, ParamEnv::reveal_all()) {
                    self.extend_from_ty(ty)?;
                }

                self.debug_exit();
                Ok(())
            }
            Adt(adt_def, substs_ref) => {
                use rustc_middle::ty::AdtKind::*;
                match adt_def.adt_kind() {
                    Struct => self.extend_struct(ty, *adt_def, substs_ref),
                    Enum => self.extend_enum(ty, *adt_def, substs_ref),
                    Union => Err(BuilderError::TypeNotSupported(ty)),
                }
            }

            &RawPtr(ty_and_mut) => {
                let layout = layout_of(tcx, ty_and_mut.ty)?;
                self.debug_enter(|ip, parent| DebugEntry::EnterPtr {
                    ip,
                    parent,
                    ty: ty_and_mut.ty,
                    mutbl: ty_and_mut.mutbl,
                });
                self.push(Inst::Ref(InstRef {
                    is_ptr: true,
                    ref_kind: ty_and_mut.mutbl,
                    ty: ty_and_mut.ty,
                    data_size: layout.size() as u32,
                    data_align: layout.align() as u32,
                }))?;
                let tail_size = layout.size().checked_sub(1)
                    .expect("Pointer should be at least one byte long");
                self.repeat_with(tail_size as u32, || Inst::RefTail)?;
                Ok(())
            }

            &Ref(_region, rty, mu) => {
                let layout = layout_of(tcx, ty)?;
                self.debug_enter(|ip, parent| DebugEntry::EnterRef {
                    ip,
                    parent,
                    ty: rty,
                    mutbl: mu,
                });
                self.push(Inst::Ref(InstRef {
                    is_ptr: false,
                    ref_kind: mu,
                    ty: rty,
                    data_size: layout.size() as u32,
                    data_align: layout.align() as u32,
                }))?;
                let tail_size = layout.size().checked_sub(1)
                    .expect("Pointer should be at least one byte long");
                self.repeat_with(tail_size as u32, || Inst::RefTail)?;
                Ok(())
            }

            _ => Err(BuilderError::TypeNotSupported(ty)),
        }
    }

    fn extend_struct(
        &mut self,
        ty: Ty<'tcx>,
        adt_def: AdtDef<'tcx>,
        substs_ref: SubstsRef<'tcx>,
    ) -> Result<'tcx> {
        let tcx = self.tcx;
        let layout = layout_of(self.tcx, ty)?;
        let repr = adt_def.repr();
        // is the layout well-defined?
        if !repr.c() {
            return Err(BuilderError::NonReprC(ty));
        }

        self.debug_enter(|ip, parent| DebugEntry::EnterStruct { ip, parent, ty });
        for (index, field_def) in adt_def.all_fields().enumerate() {
            let field_ty = field_def.ty(tcx, substs_ref);

            self.debug_enter(|ip, parent| DebugEntry::EnterStructField {
                ip,
                parent,
                ty: field_ty,
                def_id: field_def.did,
                index,
            });

            let field_layout = layout_of(tcx, field_ty)?;
            let private = !field_def.vis.is_public();
            self.pad_to_align(field_layout.align())?;
            if private {
                self.priv_depth += 1;
            }
            self.extend_from_ty(field_ty)?;
            if private {
                self.priv_depth -= 1;
            }

            self.debug_exit();
        }

        self.pad_to_align(layout.align())?;
        self.debug_exit();
        Ok(())
    }

    fn extend_enum(
        &mut self,
        ty: Ty<'tcx>,
        adt_def: AdtDef<'tcx>,
        substs: SubstsRef<'tcx>,
    ) -> Result<'tcx> {
        use rustc_index::vec::Idx;
        use rustc_target::abi::VariantIdx;

        let tcx = self.tcx;
        let layout = layout_of(self.tcx, ty)?;
        let repr = adt_def.repr();
        if !repr.c() {
            return Err(BuilderError::NonReprC(ty));
        }
        self.debug_enter(|ip, parent| DebugEntry::EnterEnum { ip, parent, ty });

        let orig_layout = self.layout;

        let variants = adt_def.variants();
        let mut variant_it = variants.iter().enumerate();
        let (last_idx, last_variant) = variant_it.next_back()
            .expect("At least one variant is present");

        let mut patches = Vec::with_capacity(adt_def.variants().len());
        let mut prev_patch: Option<usize> = None;

        for (index, variant) in variant_it {
            self.debug_enter(|ip, parent| DebugEntry::EnterEnumVariant {
                ip,
                parent,
                def_id: variant.def_id,
                index,
            });
            let split = self.insts.len();
            if let Some(prev_split) = prev_patch {
                self.insts[prev_split].patch_split(split as InstPtr);
            }
            prev_patch = Some(split);
            self.insts.push(Inst::new_invalid_split());

            let discr = adt_def.discriminant_for_variant(tcx, VariantIdx::new(index));
            let tag_layout = layout_of(tcx, discr.ty)?;
            self.extend_enum_variant(layout, substs, tag_layout, discr.val, variant)?;

            patches.push(self.insts.len());
            self.insts.push(Inst::new_invalid_goto());
            self.layout = orig_layout;
            self.debug_exit();
        }

        if let Some(last_split) = prev_patch {
            let ip = self.insts.len() as InstPtr;
            self.insts[last_split].patch_split(ip);
        }

        let discr = adt_def.discriminant_for_variant(tcx, VariantIdx::new(last_idx));
        let tag_layout = layout_of(tcx, discr.ty)?;
        self.extend_enum_variant(layout, substs, tag_layout, discr.val, last_variant)?;

        let ip = self.insts.len() as InstPtr;

        for patch in patches {
            self.insts[patch].patch_goto(ip);
        }
        self.debug_exit();
        Ok(())
    }

    fn int_ty(&self, int_type: rustc_attr::IntType) -> Ty<'tcx> {
        use rustc_attr::IntType;
        use rustc_middle::ty::{int_ty, uint_ty};
        match int_type {
            IntType::SignedInt(si) => self.tcx.mk_mach_int(int_ty(si)),
            IntType::UnsignedInt(ui) => self.tcx.mk_mach_uint(uint_ty(ui)),
        }
    }

    fn extend_enum_variant(
        &mut self,
        layout: Layout,
        substs: SubstsRef<'tcx>,
        tag_layout: Layout,
        discr: u128,
        variant: &'tcx VariantDef,
    ) -> Result<'tcx> {
        use rustc_target::abi::HasDataLayout;
        let endian = self.tcx.data_layout().endian;
        let private = self.priv_depth > 0;
        let tag = InstByte::for_literal(endian, tag_layout.size(), discr, private);
        self.insts.extend(tag);
        self.layout = self.layout.extend(tag_layout)
            .map_err(|_| BuilderError::LayoutOverflow)?.0;
        self.pad_to_align(layout.align())?;
        for (index, field) in variant.fields.iter().enumerate() {
            let ty = field.ty(self.tcx, substs);
            self.debug_enter(|ip, parent| DebugEntry::EnterEnumVariantField {
                ip,
                parent,
                ty,
                def_id: field.did,
                index,
            });
            self.extend_from_ty(ty)?;
            self.debug_exit();
        }
        self.pad_to_align(layout.align())?;
        Ok(())
    }
    /*
    pub fn extend_from_ty_old(&mut self, ty: &Ty) {
        let layout = layout_of(ty);
        self.layout = self.layout.align_to(layout.align()).unwrap();
        self.pad_to_align(layout.align());

        match *ty {
            Ty::Void => {
                // let literal = InstBytes::for_literal(Endian::Little, 4, 0x13371337);
                // self.insts.extend(literal.map(Inst::Bytes));
            }
            Ty::Bool => {
                self.repeat_byte(1, (0..=1).into());
                self.layout = self.layout.extend(layout).unwrap().0;
            }
            Ty::Int(size) => {
                self.repeat_byte(size, (0..=255).into());
                self.layout = self.layout.extend(layout).unwrap().0;
            }
            Ty::Ptr(ref _ptr) => {
                unimplemented!();
            }
            Ty::Ref(ref _ptr) => {
                unimplemented!();
            }
            Ty::Array(ref array) => {
                for _ in 0..array.count {
                    self.extend_from_ty(&array.element);
                }
            }
            Ty::Struct(ref s_def) => {
                for field in s_def.fields.iter() {
                    let layout = layout_of(&field.ty);
                    self.pad_to_align(layout.align());
                    if field.private { self.priv_depth += 1; }
                    self.extend_from_ty(&field.ty);
                    if field.private { self.priv_depth -= 1; }
                }
                self.pad_to_align(layout.align());
            }
            Ty::Enum(ref e_def) => {
                assert!(!e_def.variants.is_empty(), "zero-variant enum isn't repr-c");
                let mut variants = e_def.variants.iter();
                let last_variant = variants.next_back()
                    .expect("at least one variant is present");
                let mut patches = Vec::with_capacity(e_def.variants.len());
                let mut prev_patch: Option<usize> = None;
                let orig_layout = self.layout;

                for variant in variants {
                    let split = self.insts.len();
                    if let Some(prev_split) = prev_patch {
                        self.insts[prev_split].patch_split(split as InstPtr);
                    }
                    prev_patch = Some(split);
                    self.insts.push(Inst::new_invalid_split());

                    self.extend_enum_variant(e_def, variant);

                    patches.push(self.insts.len());
                    self.insts.push(Inst::new_invalid_goto());
                    self.layout = orig_layout;
                }

                if let Some(last_split) = prev_patch {
                    let ip = self.insts.len() as InstPtr;
                    self.insts[last_split].patch_split(ip);
                }

                self.extend_enum_variant(e_def, last_variant);
                self.insts.push(Inst::Join);
                let ip = self.insts.len() as InstPtr;

                for patch in patches {
                    self.insts[patch].patch_goto(ip);
                }

            }
            Ty::Union(ref u_def) => {
                assert!(!u_def.variants.is_empty(), "zero-variant enum isn't repr-c");
                let mut variants = u_def.variants.iter();
                let last_variant = variants.next_back()
                    .expect("at least one variant is present");
                let mut patches = Vec::with_capacity(u_def.variants.len());
                let mut prev_patch: Option<usize> = None;
                let orig_layout = self.layout;

                for variant in variants {
                    let split = self.insts.len();
                    if let Some(prev_split) = prev_patch {
                        self.insts[prev_split].patch_split(split as InstPtr);
                    }
                    prev_patch = Some(split);
                    self.insts.push(Inst::new_invalid_split());

                    self.extend_union_variant(u_def, variant);
                    patches.push(self.insts.len());
                    self.insts.push(Inst::new_invalid_goto());
                    self.layout = orig_layout;
                }

                if let Some(last_split) = prev_patch {
                    let ip = self.insts.len() as InstPtr;
                    self.insts[last_split].patch_split(ip);
                }

                self.extend_union_variant(u_def, last_variant);
                self.insts.push(Inst::Join);
                let ip = self.insts.len() as InstPtr;

                for patch in patches {
                    self.insts[patch].patch_goto(ip);
                }
            }
        }
    }
    fn extend_union_variant(&mut self, u_def: &Union, variant: &UnionVariant) {
        self.pad_to_align(u_def.layout.align());
        self.priv_depth += variant.private as usize;
        self.extend_from_ty(&variant.ty);
        self.priv_depth -= variant.private as usize;
        let variant_layout = layout_of(&variant.ty);
        self.pad(u_def.layout.size() - variant_layout.size());
    }
    */

    fn push(&mut self, inst: Inst<Ty<'tcx>>) -> Result<'tcx> {
        if self.insts.len() >= MAX_NFA_SIZE {
            Err(BuilderError::NfaTooLarge)
        } else {
            self.insts.push(inst);
            Ok(())
        }
    }

    fn repeat_with<F>(&mut self, count: u32, f: F) -> Result<'tcx>
    where
        F: Fn() -> Inst<Ty<'tcx>>,
    {
        for _ in 0..count {
            self.push(f())?;
        }
        Ok(())
    }

    fn pad(&mut self, padding: usize) -> Result<'tcx> {
        let parent = self.debug_parent;
        self.debug.push(DebugEntry::Padding { ip: self.insts.len() as InstPtr, parent });

        // println!("i:{}, padding: {}, layout: {:?}", self.insts.len(), padding, self.layout);
        let padding_layout = Layout::from_size_align(padding, 1)
            .map_err(|_| BuilderError::LayoutOverflow)?;
        self.layout = self.layout.extend(padding_layout)
            .map_err(|_| BuilderError::LayoutOverflow)?.0;

        self.repeat_with(padding as u32, || Inst::Uninit)
    }

    fn pad_to_align(&mut self, align: usize) -> Result<'tcx> {
        let padding = self.layout.padding_needed_for(align);
        self.pad(padding)
    }

    fn number(&mut self, size: u32, layout: Layout) -> Result<'tcx> {
        self.repeat_byte(size, (0..=255).into())?;
        self.layout = self.layout.extend(layout)?.0;
        Ok(())
    }
    fn repeat_byte(&mut self, size: u32, byte_ranges: RangeInclusive) -> Result<'tcx> {
        let private = self.priv_depth > 0;
        self.repeat_with(size, || {
            Inst::ByteRange(InstByteRange { private, range: byte_ranges, alternate: None })
        })
    }
}
