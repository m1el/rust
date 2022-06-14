//! Build NFA describing a given type

use core::alloc::Layout;
use core::iter::repeat_with;

use crate::debug::DebugEntry;
use crate::prog::*;
use crate::TransmuteError;

// use rustc_macros::TypeFoldable;
use rustc_middle::ty::TyCtxt;
use rustc_middle::ty::{subst::SubstsRef, AdtDef, FieldDef, Ty, VariantDef};
use rustc_span::def_id::DefId;

type TResult<'tcx, T = ()> = core::result::Result<T, TransmuteError<'tcx>>;

fn layout_of<'tcx>(ctx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> TResult<'tcx, Layout> {
    use rustc_middle::ty::{ParamEnv, ParamEnvAnd};
    use rustc_target::abi::TyAndLayout;

    let param_env = ParamEnv::reveal_all();
    let param_env_and_type = ParamEnvAnd { param_env, value: ty };
    let TyAndLayout { layout, .. } =
        ctx.layout_of(param_env_and_type).map_err(|_| TransmuteError::LayoutOverflow)?;
    let layout = Layout::from_size_align(
        layout.size().bytes_usize(),
        layout.align().abi.bytes().try_into().unwrap(),
    )?;
    Ok(layout)
}

const MAX_NFA_SIZE: usize = u32::max_value() as usize;

pub struct NfaBuilder<'tcx> {
    pub layout: Layout,
    pub insts: Vec<Inst<'tcx>>,
    pub priv_depth: usize,
    pub debug: Vec<DebugEntry<'tcx>>,
    pub debug_parent: usize,
    pub tcx: TyCtxt<'tcx>,
    pub module: DefId,
}

impl<'tcx> NfaBuilder<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, scope: Ty<'tcx>) -> TResult<'tcx, Self> {
        use rustc_type_ir::TyKind;
        let module = if let TyKind::Adt(adt_def, ..) = scope.kind() {
            use rustc_middle::ty::DefIdTree;
            tcx.parent(adt_def.did())
        } else {
            return Err(TransmuteError::ImproperContextParameter);
        };

        Ok(Self {
            layout: Layout::from_size_align(0, 1).expect("This layout should always succeed"),
            insts: Vec::new(),
            priv_depth: 0,
            debug: Vec::new(),
            debug_parent: 0,
            tcx,
            module,
        })
    }

    pub fn build_ty(
        tcx: TyCtxt<'tcx>,
        scope: Ty<'tcx>,
        ty: Ty<'tcx>,
    ) -> TResult<'tcx, Program<'tcx>> {
        let mut builder = Self::new(tcx, scope)?;

        builder.debug.push(DebugEntry::Root { ip: 0, ty });
        builder.debug_parent = 0;

        builder.extend_from_ty(ty)?;
        builder.push(Inst::Accept)?;
        Ok(Program::new(builder.insts, builder.debug, builder.layout.size()))
    }

    fn debug_enter<F>(&mut self, f: F)
    where
        F: Fn(InstPtr, usize) -> DebugEntry<'tcx>,
    {
        let ip = self.insts.len() as InstPtr;
        let parent = self.debug_parent;
        self.debug_parent = self.debug.len();
        self.debug.push(f(ip, parent));
    }

    fn debug_exit(&mut self) {
        let parent =
            self.debug[self.debug_parent].parent_id().expect("debug_exit on root is invalid");
        self.debug_parent = parent;
    }

    fn extend_from_ty(&mut self, ty: Ty<'tcx>) -> TResult<'tcx> {
        use rustc_middle::ty::ParamEnv;
        use rustc_type_ir::TyKind::*;

        let tcx = self.tcx;
        let layout = layout_of(self.tcx, ty)?;
        self.layout = self.layout.align_to(layout.align())?;
        self.pad_to_align(layout.align())?;

        match ty.kind() {
            Bool => {
                self.repeat_byte(1, (0..=1).into())?;
                self.layout = self.layout.extend(layout)?.0;
                Ok(())
            }

            Int(_) | Uint(_) | Float(_) => self.number(layout.size(), layout),

            &Array(ty, size) => {
                self.debug_enter(|ip, parent| DebugEntry::EnterArray { ip, parent, ty });

                for _index in 0..size.eval_usize(tcx, ParamEnv::reveal_all()) {
                    self.extend_from_ty(ty)?;
                }

                self.debug_exit();
                Ok(())
            }

            Tuple(list) => match list.len() {
                0 => Ok(()),
                1 => self.extend_from_ty(list[0]),
                _ => Err(TransmuteError::TuplesNonReprC(ty)),
            },

            Adt(adt_def, substs_ref) => {
                use rustc_middle::ty::AdtKind::*;
                match adt_def.adt_kind() {
                    Struct => self.extend_struct(ty, *adt_def, substs_ref),
                    Enum => self.extend_enum(ty, *adt_def, substs_ref),
                    Union => self.extend_union(ty, *adt_def, substs_ref),
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
                let tail_size =
                    layout.size().checked_sub(1).expect("Pointer should be at least one byte long");
                self.extend(repeat_with(|| Inst::RefTail).take(tail_size))?;
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
                let tail_size =
                    layout.size().checked_sub(1).expect("Pointer should be at least one byte long");
                self.extend(repeat_with(|| Inst::RefTail).take(tail_size))?;
                Ok(())
            }

            _ => Err(TransmuteError::TypeNotSupported(ty)),
        }
    }

    fn is_visible(&self, def_id: DefId) -> bool {
        self.tcx.visibility(def_id).is_accessible_from(self.module, self.tcx)
    }

    fn extend_struct(
        &mut self,
        ty: Ty<'tcx>,
        adt_def: AdtDef<'tcx>,
        substs_ref: SubstsRef<'tcx>,
    ) -> TResult<'tcx> {
        let tcx = self.tcx;
        let layout = layout_of(self.tcx, ty)?;
        let repr = adt_def.repr();
        // is the layout well-defined?
        if !repr.c() {
            return Err(TransmuteError::NonReprC(ty));
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
            assert!(field_layout.align() <= layout.align(), "Field align must fit struct align");
            self.pad_to_align(field_layout.align())?;

            let private = !self.is_visible(field_def.did);
            self.priv_depth += private as usize;
            self.extend_from_ty(field_ty)?;
            self.priv_depth -= private as usize;

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
    ) -> TResult<'tcx> {
        use rustc_index::vec::Idx;
        use rustc_target::abi::VariantIdx;

        let tcx = self.tcx;
        let layout = layout_of(self.tcx, ty)?;
        let repr = adt_def.repr();
        if !repr.c() || adt_def.variants().len() < 1 {
            return Err(TransmuteError::NonReprC(ty));
        }
        self.debug_enter(|ip, parent| DebugEntry::EnterEnum { ip, parent, ty });

        let orig_layout = self.layout;

        let variants = adt_def.variants();
        let mut variant_it = variants.iter().enumerate();
        let (last_idx, last_variant) =
            variant_it.next_back().expect("At least one variant is present");

        let mut patches = Vec::with_capacity(adt_def.variants().len());
        let mut prev_patch: Option<usize> = None;

        for (index, variant) in variant_it {
            let split = self.push_split()?;
            if let Some(prev_split) = prev_patch {
                self.insts[prev_split].patch_split(split as InstPtr);
            }
            prev_patch = Some(split);

            let discr = adt_def.discriminant_for_variant(tcx, VariantIdx::new(index));
            let tag_layout = layout_of(tcx, discr.ty)?;
            self.extend_enum_variant(layout, substs, tag_layout, discr.val, index, variant)?;

            patches.push(self.push_goto()?);
            self.layout = orig_layout;
        }

        if let Some(last_split) = prev_patch {
            let ip = self.insts.len() as InstPtr;
            self.insts[last_split].patch_split(ip);
        }

        let discr = adt_def.discriminant_for_variant(tcx, VariantIdx::new(last_idx));
        let tag_layout = layout_of(tcx, discr.ty)?;
        self.extend_enum_variant(layout, substs, tag_layout, discr.val, last_idx, last_variant)?;

        let ip = self.insts.len() as InstPtr;

        for patch in patches {
            self.insts[patch].patch_goto(ip);
        }
        self.debug_exit();
        Ok(())
    }

    fn extend_enum_variant(
        &mut self,
        layout: Layout,
        substs: SubstsRef<'tcx>,
        tag_layout: Layout,
        discr: u128,
        index: usize,
        variant: &'tcx VariantDef,
    ) -> TResult<'tcx> {
        use rustc_target::abi::HasDataLayout;
        let endian = self.tcx.data_layout().endian;

        self.debug_enter(|ip, parent| DebugEntry::EnterEnumVariant {
            ip,
            parent,
            def_id: variant.def_id,
            index,
        });

        let private = self.priv_depth > 0;
        let tag = InstByte::for_literal(endian, tag_layout.size(), discr, private);
        self.insts.extend(tag);
        self.layout = self.layout.extend(tag_layout).map_err(|_| TransmuteError::LayoutOverflow)?.0;
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
        self.debug_exit();
        Ok(())
    }

    fn extend_union(
        &mut self,
        ty: Ty<'tcx>,
        adt_def: AdtDef<'tcx>,
        substs: SubstsRef<'tcx>,
    ) -> TResult<'tcx> {
        use rustc_index::vec::Idx;
        use rustc_target::abi::VariantIdx;
        let layout = layout_of(self.tcx, ty)?;
        let repr = adt_def.repr();
        assert!(adt_def.variants().len() == 1, "Unions must have one variant");
        let all_fields = &adt_def.variant(VariantIdx::new(0)).fields;
        if !repr.c() || all_fields.len() < 1 {
            return Err(TransmuteError::NonReprC(ty));
        }
        self.debug_enter(|ip, parent| DebugEntry::EnterUnion { ip, parent, ty });

        let orig_layout = self.layout;

        let mut fields_it = all_fields.iter().enumerate();
        let (last_idx, last_field) =
            fields_it.next_back().expect("At least one variant is present");

        let mut patches = Vec::with_capacity(adt_def.variants().len());
        let mut prev_patch: Option<usize> = None;

        for (index, field) in fields_it {
            let split = self.push_split()?;
            if let Some(prev_split) = prev_patch {
                self.insts[prev_split].patch_split(split as InstPtr);
            }
            prev_patch = Some(split);

            self.extend_union_variant(layout, substs, index, field)?;

            patches.push(self.push_goto()?);
            self.layout = orig_layout;
            self.debug_exit();
        }

        if let Some(last_split) = prev_patch {
            let ip = self.insts.len() as InstPtr;
            self.insts[last_split].patch_split(ip);
        }

        self.extend_union_variant(layout, substs, last_idx, last_field)?;

        let ip = self.insts.len() as InstPtr;

        for patch in patches {
            self.insts[patch].patch_goto(ip);
        }
        self.debug_exit();
        Ok(())
    }

    fn extend_union_variant(
        &mut self,
        layout: Layout,
        substs: SubstsRef<'tcx>,
        index: usize,
        field: &'tcx FieldDef,
    ) -> TResult<'tcx> {
        let ty = field.ty(self.tcx, substs);
        self.debug_enter(|ip, parent| DebugEntry::EnterUnionVariant {
            ip,
            parent,
            def_id: field.did,
            ty,
            index,
        });

        let ty_layout = layout_of(self.tcx, ty)?;
        assert!(ty_layout.align() <= layout.align(), "Union field align must fit parent align");

        let private = !self.is_visible(field.did);
        self.priv_depth += private as usize;
        self.extend_from_ty(ty)?;
        self.priv_depth -= private as usize;

        self.pad(layout.size() - ty_layout.size())?;
        self.debug_exit();
        Ok(())
    }

    fn push_split(&mut self) -> TResult<'tcx, usize> {
        let ip = self.insts.len();
        self.push(Inst::new_invalid_split())?;
        Ok(ip)
    }

    fn push_goto(&mut self) -> TResult<'tcx, usize> {
        let ip = self.insts.len();
        self.push(Inst::new_invalid_goto())?;
        Ok(ip)
    }

    fn push(&mut self, inst: Inst<'tcx>) -> TResult<'tcx> {
        if self.insts.len() >= MAX_NFA_SIZE {
            Err(TransmuteError::NfaTooLarge)
        } else {
            self.insts.push(inst);
            Ok(())
        }
    }

    fn extend<I>(&mut self, it: I) -> TResult<'tcx>
    where
        I: Iterator<Item = Inst<'tcx>>,
    {
        for item in it {
            self.push(item)?;
        }
        Ok(())
    }

    fn pad(&mut self, padding: usize) -> TResult<'tcx> {
        if padding == 0 {
            return Ok(());
        }

        let parent = self.debug_parent;
        self.debug.push(DebugEntry::Padding { ip: self.insts.len() as InstPtr, parent });

        // println!("i:{}, padding: {}, layout: {:?}", self.insts.len(), padding, self.layout);
        let padding_layout =
            Layout::from_size_align(padding, 1).map_err(|_| TransmuteError::LayoutOverflow)?;
        self.layout =
            self.layout.extend(padding_layout).map_err(|_| TransmuteError::LayoutOverflow)?.0;

        self.extend(repeat_with(|| Inst::Uninit).take(padding))
    }

    fn pad_to_align(&mut self, align: usize) -> TResult<'tcx> {
        let padding = self.layout.padding_needed_for(align);
        self.pad(padding)
    }

    fn number(&mut self, size: usize, layout: Layout) -> TResult<'tcx> {
        self.repeat_byte(size, (0..=255).into())?;
        self.layout = self.layout.extend(layout)?.0;
        Ok(())
    }

    fn repeat_byte(&mut self, size: usize, byte_ranges: RangeInclusive) -> TResult<'tcx> {
        let private = self.priv_depth > 0;
        self.extend(
            repeat_with(|| {
                Inst::ByteRange(InstByteRange { private, range: byte_ranges, alternate: None })
            })
            .take(size),
        )
    }
}
