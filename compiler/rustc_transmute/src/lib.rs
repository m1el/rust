#![feature(alloc_layout_extra, control_flow_enum, iterator_try_reduce)]
#![allow(unused_imports, dead_code, unused_variables)]
use rustc_infer::infer::InferCtxt;
use rustc_macros::TypeFoldable;
use rustc_middle::traits::ObligationCause;
use rustc_middle::ty::Binder;
use rustc_middle::ty::Ty;

pub(crate) use rustc_data_structures::fx::FxHashMap as Map;
pub(crate) use rustc_data_structures::fx::FxHashSet as Set;

mod nfa;
pub use nfa::Nfa;

mod maybe_transmutable;
pub use maybe_transmutable::maybe_transmutable;

#[derive(TypeFoldable, Debug, Clone, Copy)]
pub struct Types<'tcx> {
    pub src: Ty<'tcx>,
    pub dst: Ty<'tcx>,
}

/// The type encodes answers to the question: "Are these types transmutable?"
#[derive(Debug, Hash, Eq, PartialEq, PartialOrd, Ord, Clone)]
pub enum Answer<'tcx> {
    /// `Src` is transmutable into `Dst`.
    Yes,

    /// `Src` is NOT transmutable into `Dst`.
    No,

    /// `Src` is transmutable into `Dst`, if `src` is transmutable into `dst`.
    IfTransmutable { src: nfa::Ref<'tcx>, dst: nfa::Ref<'tcx> },

    /// `Src` is transmutable into `Dst`, if all of the enclosed requirements are met.
    IfAll(Vec<Answer<'tcx>>),

    /// `Src` is transmutable into `Dst` if any of the enclosed requirements are met.
    IfAny(Vec<Answer<'tcx>>),
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
        assume_alignment: bool,
        assume_lifetimes: bool,
        assume_validity: bool,
        assume_visibility: bool,
    ) -> Answer<'tcx> {
        let src_ty = src_and_dst.map_bound(|types| types.src).skip_binder();
        let dst_ty = src_and_dst.map_bound(|types| types.dst).skip_binder();

        let answer = maybe_transmutable(
            src_ty,
            dst_ty,
            scope,
            assume_alignment,
            assume_lifetimes,
            assume_validity,
            assume_visibility,
            false,
            self.infcx.tcx,
        );

        answer
    }
}
