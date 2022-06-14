use crate::prog::InstPtr;
use rustc_hir::Mutability;
use rustc_span::def_id::DefId;

#[derive(Clone)]
pub enum DebugEntry<R: Clone> {
    Root { ip: InstPtr, ty: R },
    EnterStruct { ip: InstPtr, parent: usize, ty: R },
    EnterStructField { ip: InstPtr, parent: usize, ty: R, def_id: DefId, index: usize },
    EnterEnum { ip: InstPtr, parent: usize, ty: R },
    EnterEnumTag { ip: InstPtr, parent: usize },
    EnterEnumVariant { ip: InstPtr, parent: usize, def_id: DefId, index: usize },
    EnterEnumVariantField { ip: InstPtr, parent: usize, ty: R, def_id: DefId, index: usize },
    EnterUnion { ip: InstPtr, parent: usize, ty: R },
    EnterUnionVariant { ip: InstPtr, parent: usize, ty: R, def_id: DefId, index: usize },
    EnterArray { ip: InstPtr, parent: usize, ty: R },
    EnterPtr { ip: InstPtr, parent: usize, ty: R, mutbl: Mutability },
    EnterRef { ip: InstPtr, parent: usize, ty: R, mutbl: Mutability },
    EnterFork { ip: InstPtr, offset: InstPtr },
    Padding { ip: InstPtr, parent: usize },
}

impl<R: Clone> DebugEntry<R> {
    pub fn ip(&self) -> InstPtr {
        use DebugEntry::*;
        match self {
            Root { ip, .. }
            | EnterStruct { ip, .. }
            | EnterStructField { ip, .. }
            | EnterEnum { ip, .. }
            | EnterEnumTag { ip, ..  }
            | EnterEnumVariant { ip, .. }
            | EnterEnumVariantField { ip, .. }
            | EnterUnion { ip, .. }
            | EnterUnionVariant { ip, .. }
            | EnterArray { ip, .. }
            | EnterPtr { ip, .. }
            | EnterRef { ip, .. }
            | EnterFork { ip, .. }
            | Padding { ip, .. } => *ip
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
