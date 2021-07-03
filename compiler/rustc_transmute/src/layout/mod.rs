use std::fmt::{self, Debug};
use std::hash::Hash;

pub(crate) mod tree;
pub(crate) use tree::Tree;

pub(crate) mod nfa;
pub(crate) use nfa::Nfa;

pub(crate) mod dfa;
pub(crate) use dfa::Dfa;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub(crate) struct Uninhabited;

/// An instance of a byte is either initialized to a particular value, or uninitialized.
#[derive(Hash, Eq, PartialEq, Clone, Copy)]
pub(crate) enum Byte {
    Uninit,
    Init(u8),
}

impl fmt::Debug for Byte {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::Uninit => f.write_str("??u8"),
            Self::Init(b) => write!(f, "{:#04x}u8", b),
        }
    }
}

pub(crate) trait Def: Debug + Hash + Eq + PartialEq + Copy + Clone {}
pub trait Ref: Debug + Hash + Eq + PartialEq + Copy + Clone {}

impl Def for ! {}
impl Ref for ! {}

#[cfg(feature = "rustc")]
pub(crate) mod rustc {
    use super::Tree;

    use rustc_middle::mir::Mutability;
    use rustc_middle::ty;
    use rustc_middle::ty::util::Discr;
    use rustc_middle::ty::AdtDef;
    use rustc_middle::ty::Region;
    use rustc_middle::ty::SubstsRef;
    use rustc_middle::ty::Ty;
    use rustc_middle::ty::TyCtxt;
    use rustc_middle::ty::VariantDef;
    use std::alloc::Layout;

    /// A reference in the layout [`Nfa`].
    #[derive(Debug, Hash, Eq, PartialEq, PartialOrd, Ord, Clone, Copy)]
    pub struct Ref<'tcx> {
        lifetime: Region<'tcx>,
        ty: Ty<'tcx>,
        mutability: Mutability,
    }

    impl<'tcx> super::Ref for Ref<'tcx> {}

    impl<'tcx> Ref<'tcx> {
        pub fn min_align(&self) -> usize {
            todo!()
        }
    }

    /// A visibility node in the layout [`Nfa`].
    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub enum Def<'tcx> {
        Adt(ty::AdtDef<'tcx>),
        Variant(&'tcx ty::VariantDef),
        Field(&'tcx ty::FieldDef),
        Primitive,
    }

    impl<'tcx> super::Def for Def<'tcx> {}

    impl<'tcx> Tree<Def<'tcx>, Ref<'tcx>> {
        pub fn from_ty(ty: Ty<'tcx>, tcx: TyCtxt<'tcx>) -> Result<Self, ()> {
            use rustc_middle::ty::FloatTy::*;
            use rustc_middle::ty::IntTy::*;
            use rustc_middle::ty::UintTy::*;
            use rustc_target::abi::HasDataLayout;

            let target = tcx.data_layout();

            match ty.kind() {
                ty::Bool => Ok(Self::bool()),

                ty::Int(I8) | ty::Uint(U8) => Ok(Self::u8()),
                ty::Int(I16) | ty::Uint(U16) => Ok(Self::number(2)),
                ty::Int(I32) | ty::Uint(U32) | ty::Float(F32) => Ok(Self::number(4)),
                ty::Int(I64) | ty::Uint(U64) | ty::Float(F64) => Ok(Self::number(8)),
                ty::Int(I128) | ty::Uint(U128) => Ok(Self::number(16)),
                ty::Int(Isize) | ty::Uint(Usize) => {
                    Ok(Self::number(target.pointer_size.bytes_usize()))
                }

                ty::Adt(adt_def, substs_ref) => {
                    use rustc_middle::ty::AdtKind;

                    // The layout begins with this adt's visibility.
                    let vis = Self::vis(Def::Adt(*adt_def));

                    // And is followed the layout(s) of its variants
                    Ok(vis.then(match adt_def.adt_kind() {
                        AdtKind::Struct => {
                            // is the layout well-defined?
                            if !adt_def.repr().c() {
                                return Err(());
                            }

                            Self::from_repr_c_variant(
                                ty,
                                *adt_def,
                                substs_ref,
                                None,
                                adt_def.non_enum_variant(),
                                tcx,
                            )?
                        }
                        AdtKind::Enum => {
                            // is the layout well-defined?
                            if !(adt_def.repr().c() || adt_def.repr().int.is_some()) {
                                return Err(());
                            }

                            let mut variants = vec![];

                            for (idx, discr) in adt_def.discriminants(tcx) {
                                variants.push(Self::from_repr_c_variant(
                                    ty,
                                    *adt_def,
                                    substs_ref,
                                    Some(discr),
                                    adt_def.variant(idx),
                                    tcx,
                                )?);
                            }

                            Self::Alt(variants)
                        }
                        AdtKind::Union => {
                            // is the layout well-defined?
                            if !adt_def.repr().c() {
                                return Err(());
                            }

                            let ty_layout = layout_of(tcx, ty);

                            let mut variants = vec![];

                            for field in adt_def.all_fields() {
                                let variant_ty = field.ty(tcx, substs_ref);
                                let variant_layout = layout_of(tcx, variant_ty);

                                let padding_needed = ty_layout.size() - variant_layout.size();
                                let def = Def::Field(field);
                                let variant = Self::from_ty(variant_ty, tcx)?
                                    .then(Self::padding(padding_needed));

                                variants.push(variant);
                            }

                            Self::Alt(variants)
                        }
                    }))
                }
                _ => Err(()),
            }
        }

        pub fn from_repr_c_variant(
            ty: Ty<'tcx>,
            adt_def: AdtDef<'tcx>,
            substs_ref: SubstsRef<'tcx>,
            discr: Option<Discr<'tcx>>,
            variant_def: &'tcx VariantDef,
            tcx: TyCtxt<'tcx>,
        ) -> Result<Self, ()> {
            use rustc_target::abi::Align;

            let mut seq = vec![];

            let repr = adt_def.repr();
            let max_align = repr.align.unwrap_or(Align::MAX);

            let ty_layout = layout_of(tcx, ty);
            let mut variant_layout = Layout::from_size_align(0, ty_layout.align()).unwrap();

            // The layout of the variant is prefixed by the discriminant, if any.
            if let Some(discr) = discr {
                let discr_layout = layout_of(tcx, discr.ty);
                variant_layout = variant_layout.extend(discr_layout).unwrap().0;
                let disr = Self::from_disr(discr, tcx);
                seq.push(disr);
            }

            // Next come fields.
            for field_def in variant_def.fields.iter() {
                //let field_vis = Self::vis(Def::Field(field_def));
                let field_ty = field_def.ty(tcx, substs_ref);
                let field_layout = layout_of(tcx, field_ty);

                let padding_needed = variant_layout
                    .padding_needed_for(field_layout.align())
                    .min(max_align.bytes().try_into().unwrap());

                let padding = Self::padding(padding_needed);

                variant_layout = variant_layout.extend(field_layout).unwrap().0;

                seq.push(padding);
                seq.push(Self::from_ty(field_ty, tcx)?);
            }

            // Finally: padding.
            let padding_needed = ty_layout.size() - variant_layout.size();
            let nfa = seq.push(Self::padding(padding_needed));

            Ok(Self::Seq(seq))
        }

        pub fn from_disr(discr: Discr<'tcx>, tcx: TyCtxt<'tcx>) -> Self {
            // FIXME(@jswrenn): I'm certain this is missing needed endian nuance.
            let width = layout_of(tcx, discr.ty).size();
            let bytes = discr.val.to_ne_bytes();
            let bytes = &bytes[..width];
            Self::Seq(bytes.into_iter().copied().map(|b| Self::from_bits(b)).collect())
        }
    }

    fn layout_of<'tcx>(ctx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Layout {
        use rustc_middle::ty::{ParamEnv, ParamEnvAnd};
        use rustc_target::abi::TyAndLayout;

        let param_env = ParamEnv::reveal_all();
        let param_env_and_type = ParamEnvAnd { param_env, value: ty };
        let TyAndLayout { layout, .. } = ctx.layout_of(param_env_and_type).unwrap();
        Layout::from_size_align(
            layout.size().bytes_usize(),
            layout.align().abi.bytes().try_into().unwrap(),
        )
        .unwrap()
    }
}
