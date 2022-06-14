use crate::prog::InstPtr;
use rustc_hir::Mutability;
use rustc_span::def_id::DefId;

pub enum DebugEntry<R> {
    Root { ip: InstPtr, ty: R },
    EnterStruct { ip: InstPtr, parent: usize, ty: R },
    EnterStructField { ip: InstPtr, parent: usize, ty: R, def_id: DefId, index: usize },
    EnterEnum { ip: InstPtr, parent: usize, ty: R },
    EnterEnumVariant { ip: InstPtr, parent: usize, def_id: DefId, index: usize },
    EnterEnumVariantField { ip: InstPtr, parent: usize, ty: R, def_id: DefId, index: usize },
    EnterArray { ip: InstPtr, parent: usize, ty: R },
    EnterPtr { ip: InstPtr, parent: usize, ty: R, mutbl: Mutability },
    EnterRef { ip: InstPtr, parent: usize, ty: R, mutbl: Mutability },
    EnterFork { ip: InstPtr, offset: InstPtr },
    Padding { ip: InstPtr, parent: usize },
}

impl<R> DebugEntry<R> {
    pub fn parent_id(&self) -> Option<usize> {
        match self {
            DebugEntry::Root { .. } | DebugEntry::EnterFork { .. } => None,
            DebugEntry::EnterStruct { parent, .. }
            | DebugEntry::EnterStructField { parent, .. }
            | DebugEntry::EnterEnum { parent, .. }
            | DebugEntry::EnterEnumVariant { parent, .. }
            | DebugEntry::EnterEnumVariantField { parent, .. }
            | DebugEntry::EnterArray { parent, .. }
            | DebugEntry::Padding { parent, .. }
            | DebugEntry::EnterPtr { parent, .. }
            | DebugEntry::EnterRef { parent, .. } => Some(*parent),
        }
    }
}
