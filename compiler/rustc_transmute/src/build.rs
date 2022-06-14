//! Build NFA describing a given type

use core::alloc::{Layout, LayoutError};

use crate::prog::*;
use rustc_middle::ty::Ty;
use rustc_middle::ty::TyCtxt;
use rustc_target::abi::Endian;

type Result<'tcx, T=()> = core::result::Result<T, BuilderError<'tcx>>;

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
    NonReprCStruct(Ty<'tcx>),
    TypeNotSupported(Ty<'tcx>),
    LayoutOverflow,
    NfaTooLarge,
}

impl<'a> core::convert::From<LayoutError> for BuilderError<'a> {
    fn from(err: LayoutError) -> Self {
        BuilderError::LayoutOverflow
    }
}

const MAX_NFA_SIZE: usize = u32::max_value() as usize;

pub struct NfaBuilder {
    pub endian: Endian,
    pub layout: Layout,
    pub insts: Vec<Inst>,
    pub priv_depth: usize,
}

impl NfaBuilder {
    pub fn new(endian: Endian) -> Self {
        Self {
            endian,
            layout: Layout::from_size_align(0, 1)
                .expect("This layout should always succeed"),
            insts: Vec::new(),
            priv_depth: 0,
        }
    }

    pub fn build_ty<'tcx>(ty: Ty<'tcx>, tcx: TyCtxt<'tcx>) -> Result<'tcx, Program> {
        use rustc_target::abi::HasDataLayout;
        let endian = tcx.data_layout().endian;
        let mut compiler = Self::new(endian)?;
        compiler.extend_from_ty(ty, tcx)?;
        compiler.push(Inst::Accept)?;
        Ok(Program::new(compiler.insts, "TODO"))
    }

    fn extend_from_ty<'tcx>(&mut self, ty: Ty<'tcx>, tcx: TyCtxt<'tcx>) -> Result<'tcx> {
        use rustc_middle::ty::FloatTy::*;
        use rustc_middle::ty::IntTy::*;
        use rustc_middle::ty::TyKind::*;
        use rustc_middle::ty::UintTy::*;
        use rustc_target::abi::{Align, Endian};
        use rustc_target::abi::HasDataLayout;
        use std::alloc::Layout;
        use std::iter;

        let target = tcx.data_layout();
        let layout = layout_of(tcx, ty)?;
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
            Int(Isize) | Uint(Usize) => {
                self.number(target.pointer_size.bytes_usize() as _, layout)
            }

            Adt(adt_def, substs_ref) => {
                use rustc_middle::ty::AdtKind::*;
                match adt_def.adt_kind() {
                    Struct => {
                        let repr = adt_def.repr();
                        // is the layout well-defined?
                        if !repr.c() {
                            return Err(BuilderError::NonReprCStruct(ty));
                        }

                        let size_and_align = layout_of(tcx, ty)?;

                        for field_def in adt_def.all_fields() {
                            let field_ty = field_def.ty(tcx, substs_ref);
                            let field_layout = layout_of(tcx, field_ty)?;
                            let private = !field_def.vis.is_public();
                            self.pad_to_align(field_layout.align())?;
                            if private {
                                self.priv_depth += 1;
                            }
                            self.extend_from_ty(field_ty, tcx)?;
                            if private {
                                self.priv_depth -= 1;
                            }
                        }
                        self.pad_to_align(layout.align())
                    }
                    Enum => Err(BuilderError::TypeNotSupported(ty)),
                    Union => Err(BuilderError::TypeNotSupported(ty)),
                }
            }

            _ => Err(BuilderError::TypeNotSupported(ty)),
        }
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
    fn extend_enum_variant(&mut self, e_def: &Enum, variant: &EnumVariant) {
        let endian = self.endian;
        let private = self.priv_depth > 0;
        let tag = InstByte::for_literal(
                endian, e_def.tag_layout.size(), variant.disc, private
            );
        self.insts.extend(tag);
        self.layout = self.layout.extend(e_def.tag_layout).unwrap().0;
        self.pad_to_align(e_def.payload_layout.align());
        self.extend_from_ty(&variant.payload);
        let variant_layout = layout_of(&variant.payload);
        self.pad(e_def.payload_layout.size() - variant_layout.size());
    }
    */
    fn push<'a>(&mut self, inst: Inst) -> Result<'a> {
        if self.insts.len() >= MAX_NFA_SIZE {
            Err(BuilderError::NfaTooLarge)
        } else {
            self.insts.push(inst);
            Ok(())
        }
    }

    fn repeat_with<'a, F>(&mut self, count: u32, f: F) -> Result<'a>
        where F: Fn() -> Inst
    {
        for _ in 0..count {
            self.push(f())?;
        }
        Ok(())
    }

    fn pad<'a>(&mut self, padding: usize) -> Result<'a> {
        // println!("i:{}, padding: {}, layout: {:?}", self.insts.len(), padding, self.layout);
        let padding_layout = Layout::from_size_align(padding, 1).unwrap();
        self.layout = self.layout.extend(padding_layout).unwrap().0;
        self.repeat_with(padding as u32, || Inst::Uninit)
    }

    fn pad_to_align<'a>(&mut self, align: usize) -> Result<'a> {
        let padding = self.layout.padding_needed_for(align);
        self.pad(padding)
    }

    fn number<'a>(&mut self, size: u32, layout: Layout) -> Result<'a> {
        self.repeat_byte(size, (0..=255).into())?;
        self.layout = self.layout.extend(layout)?.0;
        Ok(())
    }
    fn repeat_byte<'a>(&mut self, size: u32, byte_ranges: RangeInclusive) -> Result<'a> {
        let private = self.priv_depth > 0;
        self.repeat_with(size, || Inst::ByteRange(InstByteRange {
            private,
            range: byte_ranges,
            alternate: None,
        }))
    }
}