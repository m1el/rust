#![feature(alloc_layout_extra, control_flow_enum, iterator_try_reduce)]
use core::alloc::LayoutError;
use rustc_macros::TypeFoldable;
use rustc_middle::ty::{Ty, TyCtxt};
// pub(crate) use rustc_data_structures::fx::FxHashMap as Map;
// pub(crate) use rustc_data_structures::fx::FxHashSet as Set;

mod build;
mod debug;
mod exec;
mod maybe_transmutable;
mod prog;

pub use crate::exec::RejectFull;
pub use debug::DebugEntry;

#[derive(Clone, Debug, TypeFoldable)]
pub enum TransmuteError<'tcx> {
    NonReprC(Ty<'tcx>),
    TuplesNonReprC(Ty<'tcx>),
    DstHasPrivateField,
    TypeNotSupported(Ty<'tcx>),
    ImproperContextParameter,
    LayoutOverflow,
    NfaTooLarge,
    Rejected(Vec<RejectFull<'tcx>>),
}

impl<'a> core::convert::From<LayoutError> for TransmuteError<'a> {
    fn from(_err: LayoutError) -> Self {
        TransmuteError::LayoutOverflow
    }
}

pub use maybe_transmutable::check_transmute;

pub struct TransmuteQuery<'tcx> {
    pub ctxt: TyCtxt<'tcx>,
    pub dst: Ty<'tcx>,
    pub src: Ty<'tcx>,
    pub scope: Ty<'tcx>,
    pub assume: Assume,
}

#[derive(Clone, Copy)]
pub struct Assume {
    pub alignment: bool,
    pub lifetimes: bool,
    pub validity: bool,
    pub visibility: bool,
}
