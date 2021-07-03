use super::{nfa, Byte, Nfa, Ref};
use crate::{Map, Set};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(PartialEq, Clone, Debug)]
pub(crate) struct Dfa<R>
where
    R: Ref,
{
    pub(crate) transitions: Map<State, Transitions<R>>,
    pub(crate) start: State,
    pub(crate) accepting: State,
}

#[derive(PartialEq, Clone, Debug)]
pub(crate) struct Transitions<R>
where
    R: Ref,
{
    byte_transitions: Map<Byte, State>,
    ref_transitions: Map<R, State>,
}

impl<R> Default for Transitions<R>
where
    R: Ref,
{
    fn default() -> Self {
        Self { byte_transitions: Map::default(), ref_transitions: Map::default() }
    }
}

impl<R> Transitions<R>
where
    R: Ref,
{
    fn insert(&mut self, transition: Transition<R>, state: State) {
        match transition {
            Transition::Byte(b) => {
                self.byte_transitions.insert(b, state);
            }
            Transition::Ref(r) => {
                self.ref_transitions.insert(r, state);
            }
        }
    }
}

/// The states in a `Nfa` represent byte offsets.
#[derive(Hash, Eq, PartialEq, PartialOrd, Ord, Copy, Clone)]
pub(crate) struct State(u64);

#[derive(Hash, Eq, PartialEq, Clone, Copy)]
pub(crate) enum Transition<R>
where
    R: Ref,
{
    Byte(Byte),
    Ref(R),
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "S_{}", self.0)
    }
}

impl<R> fmt::Debug for Transition<R>
where
    R: Ref,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::Byte(b) => b.fmt(f),
            Self::Ref(r) => r.fmt(f),
        }
    }
}

impl<R> Dfa<R>
where
    R: Ref,
{
    pub(crate) fn unit() -> Self {
        let transitions: Map<State, Transitions<R>> = Map::default();
        let start = State::new();
        let accepting = start;

        Self { transitions, start, accepting }
    }

    #[cfg(test)]
    pub(crate) fn bool() -> Self {
        let mut transitions: Map<State, Transitions<R>> = Map::default();
        let start = State::new();
        let accepting = State::new();

        transitions.entry(start).or_default().insert(Transition::Byte(Byte::Init(0x00)), accepting);

        transitions.entry(start).or_default().insert(Transition::Byte(Byte::Init(0x01)), accepting);

        Self { transitions, start, accepting }
    }

    #[tracing::instrument]
    pub(crate) fn from_nfa(nfa: Nfa<R>) -> Self {
        #[cfg_attr(feature = "rustc", allow(rustc::potential_query_instability))]
        fn nfa_set_transitions<R>(
            starts: &Vec<nfa::State>,
            transitions: &Map<nfa::State, Map<nfa::Transition<R>, Set<nfa::State>>>,
        ) -> Map<nfa::Transition<R>, Vec<nfa::State>>
        where
            R: Ref,
        {
            let mut map: Map<nfa::Transition<R>, Vec<nfa::State>> = Map::default();

            for (transition, states) in starts
                .into_iter()
                .map(|start| transitions.get(start).into_iter().flatten())
                .flatten()
            {
                map.entry(transition.clone()).or_default().extend(states);
            }
            map
        }

        let Nfa { transitions: nfa_transitions, start: nfa_start, accepting: nfa_accepting } = nfa;
        let mut dfa_transitions: Map<State, Transitions<R>> = Map::default();

        let mut nfa_to_dfa: Map<Vec<nfa::State>, State> = Map::default();

        let nfa_start_set = vec![nfa_start];
        nfa_to_dfa.insert(nfa_start_set.clone(), State::new());

        let mut queue = vec![nfa_start_set.clone()];

        while let Some(nfa_states) = queue.pop() {
            let dfa_state = *nfa_to_dfa.get(&nfa_states).unwrap();
            #[cfg_attr(feature = "rustc", allow(rustc::potential_query_instability))]
            for (transition, nfa_states_prime) in nfa_set_transitions(&nfa_states, &nfa_transitions)
            {
                let dfa_state_prime = State::new();
                nfa_to_dfa.insert(nfa_states_prime.clone(), dfa_state_prime);
                dfa_transitions
                    .entry(dfa_state)
                    .or_default()
                    .insert(transition.into(), dfa_state_prime);
                queue.push(nfa_states_prime);
            }
        }

        let dfa_start = nfa_to_dfa[&nfa_start_set];
        let dfa_accepting = nfa_to_dfa[&vec![nfa_accepting]];

        Self { transitions: dfa_transitions, start: dfa_start, accepting: dfa_accepting }
    }

    pub(crate) fn bytes_from(&self, start: State) -> Option<&Map<Byte, State>> {
        Some(&self.transitions.get(&start)?.byte_transitions)
    }

    pub(crate) fn byte_from(&self, start: State, byte: Byte) -> Option<State> {
        self.transitions.get(&start)?.byte_transitions.get(&byte).copied()
    }

    pub(crate) fn refs_from(&self, start: State) -> Option<&Map<R, State>> {
        Some(&self.transitions.get(&start)?.ref_transitions)
    }
}

impl State {
    pub(crate) fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl<R> From<nfa::Transition<R>> for Transition<R>
where
    R: Ref,
{
    fn from(nfa_transition: nfa::Transition<R>) -> Self {
        match nfa_transition {
            nfa::Transition::Byte(byte) => Transition::Byte(byte),
            nfa::Transition::Ref(r) => Transition::Ref(r),
        }
    }
}
