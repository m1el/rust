use crate::build::NfaBuilder;
// use crate::nfa::{Byte, Nfa, State, Transition};
// use crate::Answer;

use crate::exec::Execution;
use crate::{TransmuteError, TransmuteQuery};
// use crate::Assume; //Map, Set, Types};

// use rustc_middle::ty::layout::HasTyCtxt;

pub fn check_transmute<'tcx>(query: TransmuteQuery<'tcx>) -> Result<(), TransmuteError<'tcx>> {
    let TransmuteQuery { ctxt, scope, dst, src, assume } = query;
    let dst_nfa = NfaBuilder::build_ty(ctxt, scope, dst, assume)?;
    let src_nfa = NfaBuilder::build_ty(ctxt, scope, src, assume)?;
    if dst_nfa.has_private {
        return Err(TransmuteError::DstHasPrivateField);
    }
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
