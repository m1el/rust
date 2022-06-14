use core::ops::ControlFlow;

use std::process::Command;
use std::error::Error;
use std::io::Write;
use std::fs::OpenOptions;

use rustc_middle::ty::Ty;

use crate::prog::{InstPtr, Program, ProgFork, StepByte, LayoutStep, AcceptState};

pub enum CheckReason {
    Root,
    Ref,
}

enum ForkReason {
    Src,
    Dst,
}

struct ExecFork {
    dst: ProgFork,
    src: ProgFork,
    reason: ForkReason,
}

#[derive(Debug)]
pub struct Reject<'tcx> {
    src: InstPtr,
    dst: InstPtr,
    reason: AcceptState<Ty<'tcx>>,
}

pub struct Execution<'tcx> {
    forks: Vec<ExecFork>,
    dst_forks: usize,
    accept: Vec<AcceptState<Ty<'tcx>>>,
    reject: Vec<Reject<'tcx>>,
    dst: Program<Ty<'tcx>>,
    src: Program<Ty<'tcx>>,
}

impl<'tcx> Execution<'tcx> {
    pub fn new(dst: Program<Ty<'tcx>>, mut src: Program<Ty<'tcx>>) -> Self {
        src.extend_to(dst.size);
        Self {
            forks: Vec::new(),
            dst_forks: 0,
            accept: src.accept_state(0).collect(),
            reject: Vec::new(),
            dst,
            src,
        }
    }
    fn push_fork(&mut self, dst: ProgFork, src: ProgFork, reason: ForkReason) {
        self.dst_forks += matches!(reason, ForkReason::Dst) as usize;
        self.forks.push(ExecFork { src, dst, reason });
    }
    fn pop_fork(&mut self) -> ControlFlow<(), ()> {
        if let Some(fork) = self.forks.pop() {
            self.dst_forks -= matches!(fork.reason, ForkReason::Dst) as usize;
            self.src.restore_fork(fork.src);
            self.dst.restore_fork(fork.dst);
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    }

    /*
    fn print_dot(&self) -> Result<(), Box<dyn Error>> {
        let mut file = OpenOptions::new()
            .create(true).truncate(true)
            .write(true)
            .open("graph.dot")?;

        writeln!(file, "digraph q {{")?;
        self.dst.print_dot(&mut file, None)?;
        self.src.print_dot(&mut file, Some(&self.accept))?;
        writeln!(file, "}}")?;
        core::mem::drop(file);

        let success = Command::new("dot")
                .args(["-Tsvg", "-O", "graph.dot"])
                .status()?.success();

        if success {
            Ok(())
        } else {
            Err("failed to run dot".into())
        }
    }
    */

    pub fn check(&mut self) -> Vec<Reject<'tcx>> {
        'outer: loop {
            macro_rules! pop {
                () => {
                    match self.pop_fork() {
                        ControlFlow::Continue(_) => continue 'outer,
                        ControlFlow::Break(_) => break 'outer,
                    }
                }
            }

            let src_fork = self.src.save_fork();
            let dst_fork = self.dst.save_fork();
            if let Some(next_src) = self.src.next_fork() {
                self.src.next();
                self.push_fork(self.dst.save_fork(), next_src, ForkReason::Src);
                continue;
            }

            if let Some(next_dst) = self.dst.next_fork() {
                self.dst.next();
                self.push_fork(next_dst, src_fork, ForkReason::Dst);
                continue;
            }

            let (s_ip, s_byte, d_ip, d_byte) =
                match (self.src.next(), self.dst.next()) {
                    (_, None) => pop!(),
                    (None, Some(_)) => {
                        unreachable!("src should have been extended to match dst");
                    },
                    (
                        Some(LayoutStep::Byte { ip: s_ip, byte: s_byte, .. }),
                        Some(LayoutStep::Byte { ip: d_ip, byte: d_byte, .. }),
                    ) => (s_ip, s_byte, d_ip, d_byte),
                    (Some(_), Some(_)) => {
                        unreachable!("next_fork() must prevent us from getting LayoutStep::Fork from next()");
                    },
                };

            if self.accept[s_ip as usize].always() {
                pop!();
            }

            let accepts = d_byte.accepts(&s_byte);
            let (accepts, fork) = self.src.synthetic_fork(
                s_ip, accepts, self.dst_forks != 0, &mut self.accept);

            self.accept[s_ip as usize] = accepts.clone();

            if let Some(src_fork) = fork {
                self.push_fork(dst_fork, src_fork, ForkReason::Dst);
            }

            if !accepts.always() {
                self.reject.push(Reject {
                    src: s_ip,
                    dst: d_ip,
                    reason: accepts,
                });
                pop!();
            }
        }

        self.reject.drain(..)
            .filter(|rej| !self.accept[rej.src as usize].always())
            .collect()
    }
}
