use crate::layout;

/// Context necessary to answer the question "Are these types transmutable?".
pub(crate) trait QueryContext {
    type Def: layout::Def;
    type Ref: layout::Ref;
    type Scope: Copy;

    /// Is `def` accessible from the defining module of `scope`?
    fn is_accessible_from(&self, def: Self::Def, scope: Self::Scope) -> bool;

    fn min_align(&self, reference: Self::Ref) -> usize;
}

#[cfg(test)]
pub(crate) mod test {
    use super::QueryContext;

    pub(crate) struct UltraMinimal;

    impl QueryContext for UltraMinimal {
        type Def = !;
        type Ref = !;
        type Scope = ();

        fn is_accessible_from(&self, def: !, scope: ()) -> bool {
            unimplemented!()
        }

        fn min_align(&self, reference: !) -> usize {
            unimplemented!()
        }
    }
}

#[cfg(feature = "rustc")]
mod rustc {
    use super::*;
    use rustc_middle::ty::{Ty, TyCtxt};

    impl<'tcx> super::QueryContext for TyCtxt<'tcx> {
        type Def = layout::rustc::Def<'tcx>;
        type Ref = layout::rustc::Ref<'tcx>;

        type Scope = Ty<'tcx>;

        fn is_accessible_from(&self, def: Self::Def, scope: Self::Scope) -> bool {
            use layout::rustc::Def;
            use rustc_middle::ty;

            let parent = if let ty::Adt(adt_def, ..) = scope.kind() {
                use rustc_middle::ty::DefIdTree;
                self.parent(adt_def.did())
            } else {
                // is this reachable?
                return false;
            };

            let def_id = match def {
                Def::Adt(adt_def) => adt_def.did(),
                Def::Variant(variant_def) => variant_def.def_id,
                Def::Field(field_def) => field_def.did,
                Def::Primitive => {
                    // primitives do not have a def_id, but they're always accessible
                    return true;
                }
            };

            if self.visibility(def_id).is_accessible_from(parent, *self) { true } else { false }
        }

        fn min_align(&self, reference: Self::Ref) -> usize {
            unimplemented!()
        }
    }
}
