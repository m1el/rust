use super::{Byte, Def, Ref};

#[derive(Clone, Debug)]
pub(crate) enum Tree<D, R>
where
    D: Def,
    R: Ref,
{
    Seq(Vec<Self>),
    Alt(Vec<Self>),
    Def(D),
    Ref(R),
    Byte(Byte),
}

impl<D, R> Tree<D, R>
where
    D: Def,
    R: Ref,
{
    pub fn vis(def: D) -> Self {
        Self::Def(def)
    }

    /// A `Tree` representing an uninhabited type.
    pub(crate) fn uninhabited() -> Self {
        Self::Alt(vec![])
    }

    /// A `Tree` representing a zero-sized type.
    pub(crate) fn unit() -> Self {
        Self::Seq(Vec::new())
    }

    /// A `Tree` containing a single, uninitialized byte.
    pub(crate) fn uninit() -> Self {
        Self::Byte(Byte::Uninit)
    }

    /// A `Tree` representing the layout of `bool`.
    pub(crate) fn bool() -> Self {
        Self::from_bits(0x00).or(Self::from_bits(0x01))
    }

    pub(crate) fn u8() -> Self {
        Self::Alt((0u8..=255).map(Self::from_bits).collect())
    }

    pub(crate) fn from_bits(bits: u8) -> Self {
        Self::Byte(Byte::Init(bits))
    }

    pub(crate) fn number(width_in_bytes: usize) -> Self {
        Self::Seq(vec![Self::u8(); width_in_bytes])
    }

    pub(crate) fn padding(width_in_bytes: usize) -> Self {
        Self::Seq(vec![Self::uninit(); width_in_bytes])
    }

    pub(crate) fn de_def(self) -> Tree<!, R> {
        match self {
            Self::Seq(elts) => Tree::Seq(
                elts.into_iter()
                    .filter_map(
                        |elt| if let Self::Def(..) = elt { None } else { Some(elt.de_def()) },
                    )
                    .collect(),
            ),
            Self::Alt(alts) => Tree::Alt(
                alts.into_iter()
                    .filter_map(
                        |alt| if let Self::Def(..) = alt { None } else { Some(alt.de_def()) },
                    )
                    .collect(),
            ),
            Self::Byte(b) => Tree::Byte(b),
            Self::Ref(r) => Tree::Ref(r),
            Self::Def(d) => Tree::uninhabited(),
        }
    }

    /// Remove all `Def` nodes, and all branches of the layout for which `f` produces false.
    pub(crate) fn prune<F>(self, f: &F) -> Tree<!, R>
    where
        F: Fn(&D) -> bool,
    {
        match self {
            Self::Seq(elts) => {
                let mut pruned = vec![];
                for elt in elts {
                    if let Self::Def(d) = elt {
                        if !f(&d) {
                            return Tree::uninhabited();
                        }
                    } else {
                        pruned.push(elt.prune(f));
                    }
                }
                Tree::Seq(pruned)
            }
            Self::Alt(alts) => Tree::Alt(
                alts.into_iter()
                    .filter_map(|alt| {
                        if let Self::Def(d) = alt { None } else { Some(alt.prune(f.clone())) }
                    })
                    .collect(),
            ),
            Self::Byte(b) => Tree::Byte(b),
            Self::Ref(r) => Tree::Ref(r),
            Self::Def(d) => Tree::uninhabited(),
        }
    }

    pub(crate) fn is_inhabited(&self) -> bool {
        match self {
            Self::Seq(elts) => elts.into_iter().all(|elt| elt.is_inhabited()),
            Self::Alt(alts) => alts.into_iter().any(|alt| alt.is_inhabited()),
            Self::Byte(..) | Self::Ref(..) | Self::Def(..) => true,
        }
    }
}

impl<D, R> Tree<D, R>
where
    D: Def,
    R: Ref,
{
    pub(crate) fn then(self, other: Self) -> Self {
        match (self, other) {
            (Self::Seq(mut lhs), Self::Seq(mut rhs)) => {
                lhs.append(&mut rhs);
                Self::Seq(lhs)
            }
            (Self::Seq(mut lhs), rhs) => {
                lhs.push(rhs);
                Self::Seq(lhs)
            }
            (lhs, Self::Seq(mut rhs)) => {
                rhs.insert(0, lhs);
                Self::Seq(rhs)
            }
            (lhs, rhs) => Self::Seq(vec![lhs, rhs]),
        }
    }

    pub(crate) fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::Alt(mut lhs), Self::Alt(rhs)) => {
                lhs.extend(rhs);
                Self::Alt(lhs)
            }
            (Self::Alt(mut alts), alt) | (alt, Self::Alt(mut alts)) => {
                alts.push(alt);
                Self::Alt(alts)
            }
            (lhs, rhs) => Self::Alt(vec![lhs, rhs]),
        }
    }
}
