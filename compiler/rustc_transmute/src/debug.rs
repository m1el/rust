use core::fmt::Debug;
use rustc_hir::Mutability;
use rustc_macros::TypeFoldable;
use rustc_middle::ty::Ty;
use rustc_span::def_id::DefId;

use crate::prog::InstPtr;

#[derive(Clone, Debug, TypeFoldable)]
pub enum DebugEntry<'tcx> {
    Root { ip: InstPtr, ty: Ty<'tcx> },
    EnterStruct { ip: InstPtr, parent: usize, ty: Ty<'tcx> },
    EnterStructField { ip: InstPtr, parent: usize, ty: Ty<'tcx>, def_id: DefId, index: usize },
    EnterEnum { ip: InstPtr, parent: usize, ty: Ty<'tcx> },
    EnterEnumTag { ip: InstPtr, parent: usize },
    EnterEnumVariant { ip: InstPtr, parent: usize, def_id: DefId, index: usize },
    EnterEnumVariantField { ip: InstPtr, parent: usize, ty: Ty<'tcx>, def_id: DefId, index: usize },
    EnterUnion { ip: InstPtr, parent: usize, ty: Ty<'tcx> },
    EnterUnionVariant { ip: InstPtr, parent: usize, ty: Ty<'tcx>, def_id: DefId, index: usize },
    EnterArray { ip: InstPtr, parent: usize, ty: Ty<'tcx> },
    EnterPtr { ip: InstPtr, parent: usize, ty: Ty<'tcx>, mutbl: Mutability },
    EnterRef { ip: InstPtr, parent: usize, ty: Ty<'tcx>, mutbl: Mutability },
    EnterFork { ip: InstPtr, offset: InstPtr },
    Padding { ip: InstPtr, parent: usize },
}

impl<'tcx> DebugEntry<'tcx> {
    pub fn ip(&self) -> InstPtr {
        use DebugEntry::*;
        match self {
            Root { ip, .. }
            | EnterStruct { ip, .. }
            | EnterStructField { ip, .. }
            | EnterEnum { ip, .. }
            | EnterEnumTag { ip, .. }
            | EnterEnumVariant { ip, .. }
            | EnterEnumVariantField { ip, .. }
            | EnterUnion { ip, .. }
            | EnterUnionVariant { ip, .. }
            | EnterArray { ip, .. }
            | EnterPtr { ip, .. }
            | EnterRef { ip, .. }
            | EnterFork { ip, .. }
            | Padding { ip, .. } => *ip,
        }
    }

    pub fn ident(&self) -> &'static str {
        use DebugEntry::*;
        match self {
            Root { .. } => "root",
            EnterStruct { .. } => "struct",
            EnterStructField { .. } => "s_field",
            EnterEnum { .. } => "enum",
            EnterEnumTag { .. } => "enum_tag",
            EnterEnumVariant { .. } => "variant",
            EnterEnumVariantField { .. } => "v_field",
            EnterUnion { .. } => "union",
            EnterUnionVariant { .. } => "u_field",
            EnterArray { .. } => "array",
            EnterPtr { .. } => "ptr",
            EnterRef { .. } => "ref",
            EnterFork { .. } => "fork",
            Padding { .. } => "padding",
        }
    }

    pub fn parent_id(&self) -> Option<usize> {
        use DebugEntry::*;
        match self {
            Root { .. } | EnterFork { .. } => None,
            EnterStruct { parent, .. }
            | EnterStructField { parent, .. }
            | EnterEnum { parent, .. }
            | EnterEnumTag { parent, .. }
            | EnterEnumVariant { parent, .. }
            | EnterEnumVariantField { parent, .. }
            | EnterUnion { parent, .. }
            | EnterUnionVariant { parent, .. }
            | EnterArray { parent, .. }
            | Padding { parent, .. }
            | EnterPtr { parent, .. }
            | EnterRef { parent, .. } => Some(*parent),
        }
    }
}
