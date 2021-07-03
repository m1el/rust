use crate::Answer;
use rustc_data_structures::fx::FxHashMap as Map;
use rustc_data_structures::fx::FxHashSet as Set;
use rustc_middle::mir::Mutability;
use rustc_middle::ty;
use rustc_middle::ty::Region;
use rustc_middle::ty::Ty;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

/// A non-deterministic finite automaton (NFA) that represents the layout of a type.
/// The transmutability of two given types is computed by comparing their `Nfa`s.
#[derive(Debug)]
pub struct Nfa<'tcx> {
    pub transitions: Map<State, Map<Transition<'tcx>, Set<State>>>,
    pub start: State,
    pub accepting: State,
}

impl<'tcx> Nfa<'tcx> {
    /// Construct an `Nfa` containing a single visibility constraint.
    pub fn vis(vis: Vis<'tcx>) -> Self {
        let mut transitions: Map<State, Map<Transition<'tcx>, Set<State>>> = Map::default();
        let start = State::new();
        let accepting = State::new();

        let source = transitions.entry(start).or_default();
        let edge = source.entry(Transition::Vis(vis)).or_default();
        edge.insert(accepting);

        Nfa { transitions, start, accepting }
    }

    // Constructs an `Nfa` that describes the layout of `()`.
    pub fn unit() -> Self {
        let transitions: Map<State, Map<Transition<'tcx>, Set<State>>> = Map::default();
        let start = State::new();
        let accepting = start;
        Nfa { transitions, start, accepting }
    }

    // Constructs an `Nfa` that describes the layout of a padding byte.
    pub fn uninit() -> Self {
        let mut transitions: Map<State, Map<Transition<'tcx>, Set<State>>> = Map::default();
        let start = State::new();
        let accepting = State::new();

        let source = transitions.entry(start).or_default();
        let edge = source.entry(Transition::Byte(Byte::Uninit)).or_default();
        edge.insert(accepting);

        Nfa { transitions, start, accepting }
    }

    // Constructs an `Nfa` that describes the layout of `bool`
    pub fn bool() -> Self {
        let mut transitions: Map<State, Map<Transition<'tcx>, Set<State>>> = Map::default();
        let start = State::new();
        let accepting = State::new();

        let source = transitions.entry(start).or_default();
        (0..=1).map(Byte::Init).map(Transition::Byte).for_each(|instance| {
            let edge = source.entry(instance).or_default();
            edge.insert(accepting);
        });

        Nfa { transitions, start, accepting }
    }

    // Constructs an `Nfa` that describes the layout of `u8`
    pub fn u8() -> Self {
        let mut transitions: Map<State, Map<Transition<'tcx>, Set<State>>> = Map::default();
        let start = State::new();
        let accepting = State::new();

        let source = transitions.entry(start).or_default();
        (0..=u8::MAX).map(Byte::Init).map(Transition::Byte).for_each(|instance| {
            let edge = source.entry(instance).or_default();
            edge.insert(accepting);
        });

        Nfa { transitions, start, accepting }
    }

    // Constructs an `Nfa` that describes the layout of primitive number type (e.g., `u8`, `f32`, `i64`) of
    // `width_in_bytes` size.
    #[allow(non_snake_case)]
    pub fn number(width_in_bytes: usize) -> Self {
        core::iter::repeat_with(Self::u8)
            .take(width_in_bytes)
            .reduce(|a, b| a.concat(b))
            .unwrap_or_else(Self::unit)
    }

    // Constructs an `Nfa` that describes the layout of padding of `width_in_bytes` size.
    #[allow(non_snake_case)]
    pub fn padding(width_in_bytes: usize) -> Self {
        core::iter::repeat_with(Self::uninit)
            .take(width_in_bytes)
            .reduce(|a, b| a.concat(b))
            .unwrap_or_else(Self::unit)
    }

    pub fn from_ty(ty: Ty<'tcx>, tcx: TyCtxt<'tcx>) -> Result<Self, ()> {
        use rustc_middle::ty::FloatTy::*;
        use rustc_middle::ty::IntTy::*;
        use rustc_middle::ty::TyKind::*;
        use rustc_middle::ty::UintTy::*;
        use rustc_target::abi::Align;
        use rustc_target::abi::HasDataLayout;
        use std::alloc::Layout;
        use std::iter;

        fn layout_of<'tcx>(ctx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Layout {
            use rustc_middle::ty::{ParamEnv, ParamEnvAnd};
            use rustc_target::abi::TyAndLayout;

            let param_env = ParamEnv::reveal_all();
            let param_env_and_type = ParamEnvAnd { param_env, value: ty };
            let TyAndLayout { layout, .. } = ctx.layout_of(param_env_and_type).unwrap();
            Layout::from_size_align(
                layout.size.bytes_usize(),
                layout.align.abi.bytes().try_into().unwrap(),
            )
            .unwrap()
        }

        let target = tcx.data_layout();

        match ty.kind() {
            Bool => Ok(Self::bool()),

            Int(I8) | Uint(U8) => Ok(Self::u8()),
            Int(I16) | Uint(U16) => Ok(Self::number(2)),
            Int(I32) | Uint(U32) | Float(F32) => Ok(Self::number(4)),
            Int(I64) | Uint(U64) | Float(F64) => Ok(Self::number(8)),
            Int(I128) | Uint(U128) => Ok(Self::number(16)),
            Int(Isize) | Uint(Usize) => Ok(Self::number(target.pointer_size.bytes_usize())),

            Adt(adt_def, substs_ref) => {
                use rustc_middle::ty::AdtKind::*;
                match adt_def.adt_kind() {
                    Struct => {
                        let repr = adt_def.repr;

                        // is the layout well-defined?
                        if !repr.c() {
                            return Err(());
                        }

                        let max_align = repr.align.unwrap_or(Align::MAX);

                        let size_and_align = layout_of(tcx, ty);
                        let mut struct_layout =
                            Layout::from_size_align(0, size_and_align.align()).unwrap();

                        let vis = Self::vis(Vis::Adt(*adt_def));

                        let fields = adt_def.all_fields().try_fold(vis, |nfa, field_def| {
                            let field_vis = Self::vis(Vis::Field(field_def));
                            let field_ty = field_def.ty(tcx, substs_ref);
                            let field_layout = layout_of(tcx, field_ty);

                            let padding_needed = struct_layout
                                .padding_needed_for(field_layout.align())
                                .min(max_align.bytes().try_into().unwrap());

                            let padding = Self::padding(padding_needed);

                            struct_layout = struct_layout.extend(field_layout).unwrap().0;
                            // FIXME: does where `field_vis` go matter? test this!
                            Ok(nfa
                                .concat(padding)
                                .concat(field_vis)
                                .concat(Self::from_ty(field_ty, tcx)?))
                        })?;

                        let padding_needed =
                            struct_layout.pad_to_align().size() - struct_layout.size();
                        let padding = Self::padding(padding_needed);
                        let result = fields.concat(padding);
                        Ok(result)
                    }
                    _ => return Err((/* FIXME: implement this for other kinds of types */)),
                }
            }

            _ => Err(()),
        }
    }

    pub fn edges_from(&self, start: State) -> Option<&Map<Transition<'tcx>, Set<State>>> {
        self.transitions.get(&start)
    }

    /// Concatenate two `Nfa`s.
    pub fn concat(self, other: Self) -> Self {
        if self.start == self.accepting {
            return other;
        } else if other.start == other.accepting {
            return self;
        }

        let start = self.start;
        let accepting = other.accepting;

        let mut transitions: Map<State, Map<Transition<'tcx>, Set<State>>> = self.transitions;

        for (source, transition) in other.transitions {
            let fix_state = |state| if state == other.start { self.accepting } else { state };
            let entry = transitions.entry(fix_state(source)).or_default();
            for (edge, destinations) in transition {
                let entry = entry.entry(edge.clone()).or_default();
                for destination in destinations {
                    entry.insert(fix_state(destination));
                }
            }
        }

        Self { transitions, start, accepting }
    }

    /// Compute the union of two `Nfa`s.
    pub fn union(&self, other: &Self) -> Self {
        let start = self.start;
        let accepting = self.accepting;

        let mut transitions: Map<State, Map<Transition<'tcx>, Set<State>>> =
            self.transitions.clone();

        for (&(mut source), transition) in other.transitions.iter() {
            // if source is starting state of `other`, replace with starting state of `self`
            if source == other.start {
                source = self.start;
            }
            let entry = transitions.entry(source).or_default();
            for (edge, destinations) in transition {
                let entry = entry.entry(edge.clone()).or_default();
                for &(mut destination) in destinations {
                    // if dest is accepting state of `other`, replace with accepting state of `self`
                    if destination == other.accepting {
                        destination = self.accepting;
                    }
                    entry.insert(destination);
                }
            }
        }

        Self { transitions, start, accepting }
    }
}

/// The states in a `Nfa` represent byte offsets.
#[derive(Debug, Hash, Eq, PartialEq, PartialOrd, Ord, Copy, Clone)]
pub struct State(u64);

impl State {
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// The transitions between states in a `Nfa` reflect bit validity.
#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub enum Transition<'tcx> {
    Byte(Byte),
    Ref(Ref<'tcx>),
    Vis(Vis<'tcx>),
}

/// An instance of a byte is either initialized to a particular value, or uninitialized.
#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub enum Byte {
    Uninit,
    Init(u8),
}

#[derive(Debug, Hash, Eq, PartialEq, PartialOrd, Ord, Clone, Copy)]
pub struct Ref<'tcx> {
    lifetime: Region<'tcx>,
    ty: Ty<'tcx>,
    mutability: Mutability,
}

impl<'tcx> Ref<'tcx> {
    pub fn min_align(&self) -> usize {
        todo!()
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub enum Vis<'tcx> {
    Adt(&'tcx ty::AdtDef),
    Variant(&'tcx ty::VariantDef),
    Field(&'tcx ty::FieldDef),
    Primitive,
}

impl<'tcx> Vis<'tcx> {
    /// Is `self` accessible from the defining module of `scope`?
    pub fn is_accessible_from(self, scope: Ty<'tcx>, tcx: TyCtxt<'tcx>) -> Answer<'tcx> {
        use rustc_middle::ty::TyKind::*;
        let module = if let Adt(adt_def, ..) = scope.kind() {
            use rustc_middle::ty::DefIdTree;
            tcx.parent(adt_def.did).unwrap()
        } else {
            // is this reachable?
            return Answer::No;
        };
        let def_id = match self {
            Vis::Adt(&ty::AdtDef { did, .. })
            | Vis::Variant(&ty::VariantDef { def_id: did, .. })
            | Vis::Field(&ty::FieldDef { did, .. }) => did,
            Vis::Primitive => return Answer::Yes,
        };
        if tcx.visibility(def_id).is_accessible_from(module, tcx) {
            Answer::Yes
        } else {
            Answer::No
        }
    }
}
