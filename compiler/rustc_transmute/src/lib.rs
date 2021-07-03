#![feature(alloc_layout_extra, control_flow_enum, decl_macro, iterator_try_reduce, never_type)]
#![allow(dead_code, unused_variables)]

#[cfg(feature = "rustc")]
pub(crate) use rustc_data_structures::fx::{FxHashMap as Map, FxHashSet as Set};

#[cfg(not(feature = "rustc"))]
pub(crate) use std::collections::{HashMap as Map, HashSet as Set};

pub(crate) mod layout;
pub(crate) mod maybe_transmutable;

#[derive(Default)]
pub struct Assume {
    pub alignment: bool,
    pub lifetimes: bool,
    pub validity: bool,
    pub visibility: bool,
}

/// The type encodes answers to the question: "Are these types transmutable?"
#[derive(Debug, Hash, Eq, PartialEq, PartialOrd, Ord, Clone)]
pub enum Answer<R>
where
    R: layout::Ref,
{
    /// `Src` is transmutable into `Dst`.
    Yes,

    /// `Src` is NOT transmutable into `Dst`.
    No,

    /// `Src` is transmutable into `Dst`, if `src` is transmutable into `dst`.
    IfTransmutable { src: R, dst: R },

    /// `Src` is transmutable into `Dst`, if all of the enclosed requirements are met.
    IfAll(Vec<Answer<R>>),

    /// `Src` is transmutable into `Dst` if any of the enclosed requirements are met.
    IfAny(Vec<Answer<R>>),
}

#[cfg(feature = "rustc")]
mod rustc {
    use rustc_infer::infer::InferCtxt;
    use rustc_macros::TypeFoldable;
    use rustc_middle::traits::ObligationCause;
    use rustc_middle::ty::Binder;
    use rustc_middle::ty::Ty;

    /// The source and destination types of a transmutation.
    #[derive(TypeFoldable, Debug, Clone, Copy)]
    pub struct Types<'tcx> {
        /// The source type.
        pub src: Ty<'tcx>,
        /// The destination type.
        pub dst: Ty<'tcx>,
    }

    pub struct TransmuteTypeEnv<'cx, 'tcx> {
        infcx: &'cx InferCtxt<'cx, 'tcx>,
    }

    impl<'cx, 'tcx> TransmuteTypeEnv<'cx, 'tcx> {
        pub fn new(infcx: &'cx InferCtxt<'cx, 'tcx>) -> Self {
            Self { infcx }
        }

        #[allow(unused)]
        pub fn is_transmutable(
            &mut self,
            cause: ObligationCause<'tcx>,
            src_and_dst: Binder<'tcx, Types<'tcx>>,
            scope: Ty<'tcx>,
            assume: crate::Assume,
        ) -> crate::Answer<crate::layout::rustc::Ref<'tcx>> {
            let src = src_and_dst.map_bound(|types| types.src).skip_binder();
            let dst = src_and_dst.map_bound(|types| types.dst).skip_binder();
            crate::maybe_transmutable::MaybeTransmutableQuery::new(
                src,
                dst,
                scope,
                assume,
                self.infcx.tcx,
            )
            .answer()
        }
    }
}

#[cfg(feature = "rustc")]
pub use rustc::*;
