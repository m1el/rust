use crate::build::NfaBuilder;
// use crate::nfa::{Byte, Nfa, State, Transition};
// use crate::Answer;

use crate::exec::Execution;
use crate::TransmuteError;
// use crate::Assume; //Map, Set, Types};

// use rustc_middle::ty::layout::HasTyCtxt;

pub fn check_transmute<'tcx>(
    query: crate::TransmuteQuery<'tcx>,
) -> Result<(), TransmuteError<'tcx>> {
    let dst_nfa = NfaBuilder::build_ty(query.ctxt, query.scope, query.dst)?;
    let src_nfa = NfaBuilder::build_ty(query.ctxt, query.scope, query.src)?;
    // println!("dst: {:?}", dst_nfa);
    // println!("src: {:?}", src_nfa);
    let mut exec = Execution::new(dst_nfa, src_nfa);
    let result = exec.check();
    if result.len() == 0 {
        Ok(())
    } else {
        /*
        for reject in result.iter() {
            let src_dbg = reject.src
                .iter().map(|dbg| dbg.ident())
                .collect::<Vec<_>>();
            let dst_dbg = reject.dst
                .iter().map(|dbg| dbg.ident())
                .collect::<Vec<_>>();
            println!("Reject: reason={:?}, pos={}, {:?} -> {:?}", reject.reason, reject.pos, src_dbg, dst_dbg);
        }
        */
        Err(TransmuteError::Rejected(result))
    }
}
