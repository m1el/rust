use crate::prog::InstPtr;
use rustc_middle::ty::Ty;

pub enum DebugEntry<R> {
    Root {
        ip: InstPtr,
        ty: R,
    },
    EnterStruct {
        ip: InstPtr,
        parent: usize,
        ty: R,
    },
    EnterStructField {
        ip: InstPtr,
        parent: usize,
        ty: R,
        index: usize,
    },
    EnterArray {
        ip: InstPtr,
        parent: usize,
        ty: R,
    },
    // EnterFork {
    //     ip: InstPtr,
    //     parent: InstPtr,
    //     offset: InstPtr,
    // },
    Padding {
        ip: InstPtr,
        parent: usize,
    },
}