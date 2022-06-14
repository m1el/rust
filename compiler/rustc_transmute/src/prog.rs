use crate::debug::DebugEntry;
use core::fmt::{self, Debug};
use core::marker::PhantomData;

use rustc_macros::TypeFoldable;
use rustc_middle::mir::interpret::write_target_uint;
use rustc_middle::ty::Ty;
use rustc_middle::TrivialTypeFoldableImpls;
use rustc_target::abi::Endian;

pub type InstPtr = u32;

#[derive(Clone)]
pub enum Inst<'tcx> {
    Accept,
    Uninit,
    Ref(InstRef<'tcx>),
    RefTail,
    ByteRange(InstByteRange),
    Split(InstSplit),
    JoinGoto(InstPtr),
}

impl<'tcx> fmt::Debug for Inst<'tcx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        use Inst::*;
        match self {
            Accept => write!(f, "Accept"),
            Uninit => write!(f, "Uninit"),
            Ref(ref d_ref) => {
                let ref_kind = match &d_ref.ref_kind {
                    RefKind::Mut => "Unique",
                    RefKind::Not => "Shared",
                };
                let name = if d_ref.is_ptr { "Ptr" } else { "Ref" };
                write!(
                    f,
                    "{}(kind={}, data_size={}, data_align={})",
                    name, ref_kind, d_ref.data_align, d_ref.data_size
                )
            }
            Inst::RefTail => {
                write!(f, "RefTail")
            }
            ByteRange(ref range) => {
                write!(f, "ByteRange(")?;
                if range.private {
                    write!(f, "private, ")?;
                }
                if let Some(alternate) = range.alternate {
                    write!(f, "alt={}, ", alternate)?;
                }
                write!(f, "0x{:02x}-0x{:02x})", range.range.start, range.range.end)
            }
            Split(ref split) => {
                write!(f, "Split(alt={})", split.alternate)
            }
            JoinGoto(ref addr) => {
                write!(f, "JoinGoto({})", addr)
            }
        }
    }
}

impl<'tcx> Inst<'tcx> {
    pub fn new_invalid_split() -> Self {
        Inst::Split(InstSplit { alternate: InstPtr::MAX })
    }
    pub fn new_invalid_goto() -> Self {
        Inst::JoinGoto(InstPtr::MAX)
    }
    pub fn patch_split(&mut self, alternate: InstPtr) {
        match self {
            Inst::Split(ref mut split) => {
                split.alternate = alternate;
            }
            _ => panic!("invalid use of patch_split"),
        }
    }
    pub fn patch_goto(&mut self, addr: InstPtr) {
        match self {
            Inst::JoinGoto(ref mut goto) => {
                *goto = addr;
            }
            _ => panic!("invalid use of patch_goto"),
        }
    }
}

#[derive(Clone, Debug, TypeFoldable)]
pub enum AcceptState<'tcx> {
    Always,
    NeverReadUninit,
    NeverReadPrivate,
    NeverWritePrivate,
    NeverOutOfRange(RangeInclusive, RangeInclusive),
    NeverUnreachable,
    MaybeCheckRange(RangeInclusive, RangeInclusive),
    MaybeCheckRef(Ty<'tcx>, Ty<'tcx>),
    NeverReadRef,
    NeverWriteRef,
}

impl<'tcx> AcceptState<'tcx> {
    pub fn always(&self) -> bool {
        matches!(self, AcceptState::Always)
    }
}

#[derive(Debug, Clone)]
pub enum StepByte<'tcx> {
    Uninit,
    ByteRange(bool, RangeInclusive),
    RefHead(InstRef<'tcx>),
    RefTail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeInclusive {
    pub start: u8,
    pub end: u8,
}

TrivialTypeFoldableImpls! { RangeInclusive, }

impl core::convert::From<core::ops::RangeInclusive<u8>> for RangeInclusive {
    fn from(src: core::ops::RangeInclusive<u8>) -> Self {
        RangeInclusive { start: *src.start(), end: *src.end() }
    }
}
impl RangeInclusive {
    pub fn contains_range(&self, small: RangeInclusive) -> bool {
        self.start <= small.start && self.end >= small.end
    }
    pub fn intersects(&self, other: RangeInclusive) -> bool {
        self.end >= other.start && self.start <= other.end
    }
}

impl<'tcx> StepByte<'tcx> {
    pub fn accepts(&self, source: &StepByte<'tcx>) -> AcceptState<'tcx> {
        use AcceptState::*;
        use StepByte::*;
        match (self, source) {
            // Uninit bytes can accpet anything
            (Uninit, _) => Always,
            // Nothing can accept uninit
            (_, Uninit) => NeverReadUninit,
            // Cannot write private memory
            (&ByteRange(true, _), _) => NeverWritePrivate,
            // Cannot read private memory
            (_, &ByteRange(true, _)) => NeverReadPrivate,
            (RefHead(ra), RefHead(rb)) => MaybeCheckRef(ra.ty.clone(), rb.ty.clone()),
            (RefTail, RefTail) => Always,
            (RefTail | RefHead(_), _) => NeverWriteRef,
            (_, RefTail | RefHead(_)) => NeverReadRef,
            // Constant tags must match
            (&ByteRange(false, a), &ByteRange(false, b)) => {
                if a.contains_range(b) {
                    AcceptState::Always
                } else if a.intersects(b) {
                    AcceptState::MaybeCheckRange(a, b)
                } else {
                    AcceptState::NeverOutOfRange(a, b)
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct ProgFork {
    ip: InstPtr,
    pos: usize,
}

pub enum LayoutStep<'tcx> {
    Byte { ip: InstPtr, pos: usize, byte: StepByte<'tcx> },
    Fork(ProgFork),
}

pub struct Program<'tcx> {
    pub insts: Vec<Inst<'tcx>>,
    pub debug: Vec<DebugEntry<'tcx>>,
    pub has_private: bool,
    size: usize,
    ip: InstPtr,
    pos: usize,
    sforks: usize,
    took_fork: Option<InstPtr>,
    current: Option<LayoutStep<'tcx>>,
}

impl<'tcx> Program<'tcx> {
    pub fn new(
        insts: Vec<Inst<'tcx>>,
        debug: Vec<DebugEntry<'tcx>>,
        size: usize,
        has_private: bool,
    ) -> Self {
        Self {
            insts,
            debug,
            has_private,
            size,
            ip: 0,
            pos: 0,
            sforks: 0,
            took_fork: None,
            current: None,
        }
    }

    pub fn extend_to(&mut self, other: &Self) {
        let to_pad = other.size.saturating_sub(self.size);
        if to_pad == 0 {
            return;
        }

        assert!(self.sforks == 0, "Cannot extend program after synthetic fork");
        assert!(
            matches!(self.insts.pop(), Some(Inst::Accept)),
            "Expected the last instruction to be Accept"
        );

        self.debug.push(DebugEntry::Padding { ip: self.insts.len() as InstPtr, parent: 0 });
        self.insts.extend((0..to_pad).map(|_| Inst::Uninit));
        self.insts.push(Inst::Accept);
    }

    #[cfg(feature = "print_dot")]
    pub fn print_dot<W: std::io::Write>(
        &self,
        dst: &mut W,
        name: &str,
        accepts: Option<&[AcceptState<'tcx>]>,
    ) -> std::io::Result<()> {
        let mut pos = 0;
        let mut ip = 0;
        let mut to_visit = Vec::<(InstPtr, usize)>::new();
        let mut stack = Vec::new();

        writeln!(dst, "  {}_accepting [shape=rectangle, label=\"accepting {}\"];", name, name)?;
        loop {
            let color = match accepts {
                Some(accepts) => match accepts[ip as usize] {
                    AcceptState::Always => "#65bc68",
                    AcceptState::NeverUnreachable => "#eeeeee",
                    AcceptState::MaybeCheckRange(_, _) => "#d3d345",
                    _ => "#f78d6c",
                },
                None => "transparent",
            };
            // let accept_fmt = match accepts {
            //     Some(accepts) => format!("\\n{:?}", accepts[ip as usize]),
            //     None => "".into(),
            // };
            writeln!(
                dst,
                "  {}_ip_{} [style=filled,fillcolor=\"{}\",shape=ellipse, label=\"pos={}, ip{}\"];",
                name, ip, color, pos, ip
            )?;
            match &self.insts[ip as usize] {
                Inst::Accept => {
                    writeln!(dst, "  {}_ip_{} -> {}_accepting;", name, ip, name)?;
                    if let Some((oip, opos)) = to_visit.pop() {
                        pos = opos;
                        ip = oip;
                        continue;
                    } else {
                        break;
                    }
                }
                Inst::Uninit => {
                    writeln!(
                        dst,
                        "  {}_ip_{} -> {}_ip_{} [label=\"uninit\"];",
                        name,
                        ip,
                        name,
                        ip + 1
                    )?;
                    pos += 1;
                }
                Inst::ByteRange(range) => {
                    let start = range.range.start;
                    let end = range.range.end;
                    if start == end {
                        writeln!(
                            dst,
                            "  {}_ip_{} -> {}_ip_{} [label=\"byte=0x{:02x}\"];",
                            name,
                            ip,
                            name,
                            ip + 1,
                            start
                        )?;
                    } else {
                        writeln!(
                            dst,
                            "  {}_ip_{} -> {}_ip_{} [label=\"range=0x{:02x}-0x{:02x}\"];",
                            name,
                            ip,
                            name,
                            ip + 1,
                            start,
                            end
                        )?;
                    }
                    if let Some(alt) = range.alternate {
                        writeln!(
                            dst,
                            "  {}_ip_{} -> {}_ip_{} [label=\"fork\"];",
                            name, ip, name, alt
                        )?;
                    }
                    if let Some(alt) = range.alternate {
                        to_visit.push((alt, pos));
                    }
                    pos += 1;
                }
                Inst::Split(split) => {
                    writeln!(dst, "  {}_ip_{} -> {}_ip_{};", name, ip, name, split.alternate)?;
                    writeln!(dst, "  {}_ip_{} -> {}_ip_{};", name, ip, name, ip + 1)?;
                    stack.push(pos);
                }
                Inst::JoinGoto(addr) => {
                    writeln!(
                        dst,
                        "  {}_ip_{} -> {}_ip_{} [label=\"goto\"];",
                        name, ip, name, addr
                    )?;
                    pos =
                        stack.pop().expect("JoinGoto without matching split in the state machine");
                }
                _ => {
                    unimplemented!()
                }
            }
            ip += 1;
        }
        Ok(())
    }

    pub fn accept_state(&self, start: usize) -> impl Iterator<Item = AcceptState<'tcx>> + '_ {
        self.insts[start..].iter().map(|inst| match inst {
            Inst::Split(_) | Inst::JoinGoto(_) | Inst::Accept => AcceptState::Always,
            _ => AcceptState::NeverUnreachable,
        })
    }

    pub fn synthetic_fork(
        &mut self,
        ip: InstPtr,
        accepts: AcceptState<'tcx>,
        can_fork: bool,
        marks: &mut Vec<AcceptState<'tcx>>,
    ) -> (AcceptState<'tcx>, Option<ProgFork>) {
        let original = accepts.clone();
        let (dst, src) = match accepts {
            AcceptState::MaybeCheckRange(dst, src) => (dst, src),
            _ => {
                return (original, None);
            }
        };
        if !dst.intersects(src) || !can_fork {
            return (original, None);
        }
        let mut previous = match &self.insts[ip as usize] {
            Inst::ByteRange(range) => range.clone(),
            _ => {
                return (original, None);
            }
        };
        if src.start < dst.start {
            let missing_range = (src.start..=(dst.start - 1)).into();
            let location = self.copy_fork(ip);
            let alternate = previous.alternate.replace(location);
            self.insts[location as usize] = Inst::ByteRange(InstByteRange {
                private: previous.private,
                range: missing_range,
                alternate,
            });
            marks.extend(self.accept_state(location as usize));
            self.sforks += 1;
        }
        if src.end > dst.end {
            let missing_range = ((dst.end + 1)..=src.end).into();
            let location = self.copy_fork(ip);
            let alternate = previous.alternate.replace(location);
            self.insts[location as usize] = Inst::ByteRange(InstByteRange {
                private: previous.private,
                range: missing_range,
                alternate,
            });
            marks.extend(self.accept_state(location as usize));
            self.sforks += 1;
        }
        previous.range = dst;
        let fork = previous.alternate.map(|ip| ProgFork { ip, pos: self.pos });
        self.insts[ip as usize] = Inst::ByteRange(previous);
        // println!("after synthetic_fork: {:?}", self);
        (AcceptState::Always, fork)
    }

    fn copy_fork(&mut self, start: InstPtr) -> InstPtr {
        let mut depth = 0;
        let dst = self.insts.len();
        let mut pos = start as usize;
        let mut offset = (dst - pos) as InstPtr;
        let mut more_forks = Vec::new();

        self.debug.push(DebugEntry::EnterFork { ip: dst as InstPtr, offset });

        loop {
            let mut inst = self.insts[pos].clone();
            match &mut inst {
                Inst::Split(ref mut split) => {
                    depth += 1;
                    split.alternate += offset;
                }
                Inst::JoinGoto(ref mut goto) => {
                    let dst = self.insts.len();
                    if depth == 0 {
                        pos = *goto as usize;
                        offset = (dst - pos) as InstPtr;
                        continue;
                    }

                    self.debug.push(DebugEntry::EnterFork { ip: dst as InstPtr, offset });

                    depth -= 1;
                    *goto += offset;
                }
                Inst::ByteRange(ref range) => {
                    if let Some(alt) = range.alternate {
                        more_forks.push((pos, alt));
                    }
                }
                Inst::Accept => {
                    self.insts.push(inst);
                    break;
                }
                _ => {}
            }
            self.insts.push(inst);
            pos += 1;
        }
        for (pos, alt) in more_forks {
            let dst = self.copy_fork(alt);
            match self.insts[pos] {
                Inst::ByteRange(ref mut range) => {
                    range.alternate = Some(dst);
                }
                _ => unreachable!("we should point to ByteRange"),
            }
        }
        dst as InstPtr
    }

    pub fn save_fork(&self) -> ProgFork {
        // println!("{} save fork ip={} pos={}", self.name, self.ip, self.pos);
        ProgFork { ip: self.ip, pos: self.pos }
    }

    pub fn restore_fork(&mut self, fork: ProgFork) {
        // println!("{} restore fork ip={} pos={}", self.name, fork.ip, fork.pos);
        self.ip = fork.ip;
        self.pos = fork.pos;
    }

    pub fn next_fork(&mut self) -> Option<ProgFork> {
        if self.current.is_none() {
            self.advance();
        }
        match &self.current {
            Some(LayoutStep::Fork(fork)) => Some(fork.clone()),
            _ => None,
        }
    }

    pub fn next(&mut self) -> Option<LayoutStep<'tcx>> {
        if self.current.is_none() {
            self.advance();
        }
        self.current.take()
    }

    //  0(2 3|5 6)
    // (1 2|4 5)7
    // pub fn split_byte(&mut self) -> Option<>
    fn advance(&mut self) {
        while let Some(inst) = self.insts.get(self.ip as usize) {
            // print!("{} ip={} inst={:?} ", self.name, self.ip, inst);
            // println!("stack={:?}", self.stack);
            let rv = match inst {
                Inst::Accept => {
                    self.current = None;
                    return;
                }
                Inst::ByteRange(ref range) => {
                    if let Some(alternate) = range.alternate {
                        if self.took_fork.take() == Some(self.ip) {
                            Some(LayoutStep::Byte {
                                ip: self.ip,
                                pos: self.pos,
                                byte: StepByte::ByteRange(range.private, range.range),
                            })
                        } else {
                            self.took_fork = Some(self.ip);
                            self.current =
                                Some(LayoutStep::Fork(ProgFork { ip: alternate, pos: self.pos }));
                            return;
                        }
                    } else {
                        Some(LayoutStep::Byte {
                            ip: self.ip,
                            pos: self.pos,
                            byte: StepByte::ByteRange(range.private, range.range),
                        })
                    }
                }
                Inst::Uninit => {
                    Some(LayoutStep::Byte { ip: self.ip, pos: self.pos, byte: StepByte::Uninit })
                }
                Inst::Split(ref split) => {
                    Some(LayoutStep::Fork(ProgFork { ip: split.alternate, pos: self.pos }))
                }
                Inst::Ref(ref d_ref) => Some(LayoutStep::Byte {
                    ip: self.ip,
                    pos: self.pos,
                    byte: StepByte::RefHead(d_ref.clone()),
                }),
                Inst::RefTail => {
                    Some(LayoutStep::Byte { ip: self.ip, pos: self.pos, byte: StepByte::RefTail })
                }
                &Inst::JoinGoto(addr) => {
                    self.ip = addr;
                    continue;
                }
            };
            self.ip += 1;
            if matches!(rv, Some(LayoutStep::Byte { .. })) {
                self.pos += 1;
            }
            self.current = rv;
            if self.current.is_some() {
                return;
            }
        }
    }

    #[allow(dead_code)]
    pub fn resolve_debug(&self, ip: InstPtr) -> Vec<DebugEntry<'tcx>> {
        let mut result = Vec::new();
        let seek = |ip| match self.debug.binary_search_by(|entry| entry.ip().cmp(&ip)) {
            Ok(idx) => &self.debug[idx],
            Err(idx) => &self.debug[idx.saturating_sub(1)],
        };
        let mut tail = seek(ip);

        loop {
            match tail {
                DebugEntry::Root { .. } => {
                    result.push(tail.clone());
                    break;
                }
                DebugEntry::EnterFork { offset, .. } => {
                    let parent_ip =
                        ip.checked_sub(*offset).expect("Debug entry for fork is invalid");
                    tail = seek(parent_ip);
                }
                _ => {
                    result.push(tail.clone());
                    let parent = tail.parent_id().expect("Should not be root or fork");
                    tail = &self.debug[parent];
                }
            }
        }

        result.reverse();
        result
    }
}

impl<'tcx> fmt::Debug for Program<'tcx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "FiniteAutomaton {{")?;
        for (idx, inst) in self.insts.iter().enumerate() {
            writeln!(f, "  {:03} {:?}", idx, inst)?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

pub type RefKind = rustc_hir::Mutability;

#[derive(Clone)]
pub struct InstRef<'tcx> {
    pub ref_kind: RefKind,
    pub is_ptr: bool,
    pub ty: Ty<'tcx>,
    pub data_size: u32,
    pub data_align: u32,
}

impl<'tcx> fmt::Debug for InstRef<'tcx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let ref_kind = match &self.ref_kind {
            RefKind::Mut => "Unique",
            RefKind::Not => "Shared",
        };
        let name = if self.is_ptr { "Ptr" } else { "Ref" };
        write!(
            f,
            "{}(kind={}, data_size={}, data_align={})",
            name, ref_kind, self.data_align, self.data_size
        )
    }
}

#[derive(Clone)]
pub struct InstSplit {
    pub alternate: InstPtr,
}

#[derive(Clone)]
pub struct InstByte {
    pub private: bool,
    pub byte: u8,
}

impl InstByte {
    pub fn for_literal<'tcx>(
        endian: Endian,
        size: usize,
        value: u128,
        private: bool,
    ) -> impl Iterator<Item = Inst<'tcx>> {
        let mut data = [0_u8; 16];
        let start = data.len() - size;
        write_target_uint(endian, &mut data[start..], value)
            .expect("writing int literal should always succeed");
        LiteralBytes { data, private, pos: start, _marker: PhantomData }
    }
}

struct LiteralBytes<'tcx> {
    data: [u8; 16],
    private: bool,
    pos: usize,
    _marker: PhantomData<&'tcx ()>,
}

impl<'tcx> Iterator for LiteralBytes<'tcx> {
    type Item = Inst<'tcx>;
    fn next(&mut self) -> Option<Self::Item> {
        let byte = *self.data.get(self.pos)?;
        let range = (byte..=byte).into();
        let private = self.private;
        self.pos += 1;
        Some(Inst::ByteRange(InstByteRange { alternate: None, private, range }))
    }
}

#[derive(Clone)]
pub struct InstByteRange {
    pub private: bool,
    pub range: RangeInclusive,
    pub alternate: Option<InstPtr>,
}
