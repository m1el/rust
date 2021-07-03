use crate::nfa::{Byte, Nfa, State, Transition};
use crate::Answer;
use crate::{Map, Set};

use rustc_middle::ty::layout::HasTyCtxt;
use rustc_middle::ty::Ty;
use rustc_middle::ty::TyCtxt;

pub fn maybe_transmutable<'tcx>(
    src_ty: Ty<'tcx>,
    dst_ty: Ty<'tcx>,
    scope: Ty<'tcx>,
    assume_alignment: bool,
    assume_lifetimes: bool,
    assume_validity: bool,
    assume_visibility: bool,
    within_references: bool,
    tcx: TyCtxt<'tcx>,
) -> Answer<'tcx> {
    let src_nfa = &(if let Ok(nfa) = Nfa::from_ty(src_ty, tcx) { nfa } else { return Answer::No });
    let dst_nfa = &(if let Ok(nfa) = Nfa::from_ty(dst_ty, tcx) { nfa } else { return Answer::No });

    MaybeTransmutableQuery {
        src_nfa,
        dst_nfa,
        scope,
        assume_alignment,
        assume_lifetimes,
        assume_validity,
        assume_visibility,
        within_references,
        tcx: tcx.tcx(),
        cache: Map::default(),
    }
    .answer(src_nfa.start, dst_nfa.start)
}

struct MaybeTransmutableQuery<'tcx, 'nfa, C>
where
    C: HasTyCtxt<'tcx>,
{
    src_nfa: &'nfa Nfa<'tcx>,
    dst_nfa: &'nfa Nfa<'tcx>,
    scope: Ty<'tcx>,
    assume_alignment: bool,
    assume_lifetimes: bool,
    assume_validity: bool,
    assume_visibility: bool,
    within_references: bool,
    tcx: C,
    cache: Map<(State, State), Answer<'tcx>>,
}

impl<'tcx, 'nfa, C> MaybeTransmutableQuery<'tcx, 'nfa, C>
where
    C: HasTyCtxt<'tcx>,
{
    pub fn answer(&mut self, src_state: State, dst_state: State) -> Answer<'tcx> {
        let empty_map = Map::default();
        let empty_set = Set::default();
        if dst_state == self.dst_nfa.accepting {
            // truncation: `size_of(Src) >= size_of(Dst)`
            Answer::Yes
        } else if (src_state == self.src_nfa.accepting) && !self.within_references {
            // extension: `size_of(Src) < size_of(Dst)`
            // the remaining bytes of Dst must accept `Uninit`
            let dst_state_primes = self
                .dst_nfa
                .edges_from(dst_state)
                .unwrap_or(&empty_map)
                .get(&Transition::Byte(Byte::Uninit))
                .unwrap_or(&empty_set);

            there_exists(dst_state_primes, |&dst_state_prime| {
                self.answer_cached(src_state, dst_state_prime)
            })
        } else {
            let src_quantification = if self.assume_validity {
                // if the compiler may assume that the programmer is doing additional validity checks,
                // (e.g.: that `src != 3u8` when the destination type is `bool`)
                // then there must exist at least one transition out of `src_state` such that the transmute is viable...
                there_exists
            } else {
                // if the compiler cannot assume that the programmer is doing additional validity checks,
                // then for all transitions out of `src_state`, such that the transmute is viable...
                // then there must exist at least one transition out of `src_state` such that the transmute is viable...
                for_all
            };
            src_quantification(
                self.src_nfa.edges_from(src_state).unwrap_or(&empty_map),
                |(&src_transition, src_state_primes)| {
                    let dst_quantification = if self.assume_validity {
                        // for some successive src state reached via `src_transition`...
                        there_exists
                    } else {
                        // for all successive src states reached via `src_transition`...
                        for_all
                    };
                    dst_quantification(src_state_primes, |&(mut src_state_prime)| {
                        let dst_transitions =
                            self.dst_nfa.edges_from(dst_state).unwrap_or(&empty_map);
                        // there must exist at least one dst_transition
                        there_exists(dst_transitions, |(&dst_transition, dst_state_primes)| {
                            // that is compatible with the src_transition
                            (match (src_transition, dst_transition) {
                                // check visibility constraint in the dst nfa
                                (_, Transition::Vis(vis)) => {
                                    // if the dst transition is a visibility constraint we don't advance src_state_prime
                                    src_state_prime = src_state;
                                    // check the visibility constraint, if needed
                                    if self.assume_visibility {
                                        // if visibility is assumed, we don't need to actually check the visibility constraint
                                        Answer::Yes
                                    } else {
                                        // otherwise, we do
                                        vis.is_accessible_from(self.scope, self.tcx.tcx())
                                    }
                                }
                                // ignore visibility constraints in the dst nfa
                                (Transition::Vis(..), _) => {
                                    // advance the src state, but not the dst state
                                    return self.answer_cached(src_state_prime, dst_state);
                                }
                                (Transition::Byte(src_byte), Transition::Byte(dst_byte))
                                    if src_byte == dst_byte =>
                                {
                                    Answer::Yes
                                }
                                (Transition::Byte(_), Transition::Byte(Byte::Uninit)) => {
                                    Answer::Yes
                                }
                                (Transition::Ref(src), Transition::Ref(dst))
                                    if src.min_align() >= dst.min_align() =>
                                {
                                    Answer::IfTransmutable { src: src.clone(), dst: dst.clone() }
                                }
                                _ => Answer::No,
                            })
                            .and(there_exists(
                                dst_state_primes,
                                |&dst_state_prime| {
                                    // such that successive bytes of `src` are transmutable
                                    // into some path of successive bytes of `dst`
                                    self.answer_cached(src_state_prime, dst_state_prime)
                                },
                            ))
                        })
                    })
                },
            )
        }
    }

    #[inline(always)]
    fn answer_cached(&mut self, src_state_prime: State, dst_state_prime: State) -> Answer<'tcx> {
        if let Some(result) = self.cache.get(&(src_state_prime, dst_state_prime)) {
            result.clone()
        } else {
            let result = self.answer(src_state_prime, dst_state_prime);
            self.cache.insert((src_state_prime, dst_state_prime), result.clone());
            result
        }
    }
}

pub fn is_compatible<'tcx>(
    src_transition: &Transition<'tcx>,
    dst_transition: &Transition<'tcx>,
) -> Answer<'tcx> {
    match (src_transition, dst_transition) {
        (Transition::Byte(src_byte), Transition::Byte(dst_byte)) if src_byte == dst_byte => {
            Answer::Yes
        }
        (Transition::Byte(_), Transition::Byte(Byte::Uninit)) => Answer::Yes,
        (Transition::Ref(src), Transition::Ref(dst)) if src.min_align() >= dst.min_align() => {
            Answer::IfTransmutable { src: src.clone(), dst: dst.clone() }
        }
        _ => Answer::No,
    }
}

impl<'tcx> Answer<'tcx> {
    fn and(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::No, _) | (_, Self::No) => Self::No,
            (Self::Yes, Self::Yes) => Self::Yes,
            (Self::IfAll(mut lhs), Self::IfAll(ref mut rhs)) => {
                lhs.append(rhs);
                Self::IfAll(lhs)
            }
            (constraint, Self::IfAll(mut constraints))
            | (Self::IfAll(mut constraints), constraint) => {
                constraints.push(constraint);
                Self::IfAll(constraints)
            }
            (lhs, rhs) => Self::IfAll(vec![lhs, rhs]),
        }
    }

    fn or(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::Yes, _) | (_, Self::Yes) => Self::Yes,
            (Self::No, Self::No) => Self::No,
            (Self::IfAny(mut lhs), Self::IfAny(ref mut rhs)) => {
                lhs.append(rhs);
                Self::IfAny(lhs)
            }
            (constraint, Self::IfAny(mut constraints))
            | (Self::IfAny(mut constraints), constraint) => {
                constraints.push(constraint);
                Self::IfAny(constraints)
            }
            (lhs, rhs) => Self::IfAny(vec![lhs, rhs]),
        }
    }
}

pub fn for_all<'tcx, I, F>(iter: I, f: F) -> Answer<'tcx>
where
    I: IntoIterator,
    F: FnMut(<I as IntoIterator>::Item) -> Answer<'tcx>,
{
    use std::ops::ControlFlow::{Break, Continue};
    let (Continue(result) | Break(result)) =
        iter.into_iter().map(f).try_fold(Answer::Yes, |constraints, constraint| {
            match constraint.and(constraints) {
                Answer::No => Break(Answer::No),
                maybe => Continue(maybe),
            }
        });
    result
}

pub fn there_exists<'tcx, I, F>(iter: I, f: F) -> Answer<'tcx>
where
    I: IntoIterator,
    F: FnMut(<I as IntoIterator>::Item) -> Answer<'tcx>,
{
    use std::ops::ControlFlow::{Break, Continue};
    let (Continue(result) | Break(result)) =
        iter.into_iter().map(f).try_fold(Answer::No, |constraints, constraint| {
            match constraint.or(constraints) {
                Answer::Yes => Break(Answer::Yes),
                maybe => Continue(maybe),
            }
        });
    result
}
