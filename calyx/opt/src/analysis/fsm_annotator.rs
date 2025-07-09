use std::os::macos::raw::stat;

use calyx_ir::{self as ir};
use itertools::Itertools;

const FSM_STATE_CUTOFF: u64 = 300;

#[derive(Copy, Clone)]
enum Deferred<T> {
    Pending,
    Computed(T),
}

impl<T: Copy> Deferred<T> {
    fn unwrap(&self) -> T {
        match self {
            Self::Pending => panic!(),
            Self::Computed(v) => *v,
        }
    }
}

impl<T> Deferred<T> {
    fn map<S, F>(&self, f: F) -> Deferred<S>
    where
        F: FnOnce(&T) -> S,
    {
        match self {
            Self::Pending => Deferred::Pending,
            Self::Computed(t) => Deferred::Computed(f(t)),
        }
    }
}

enum StatePossibility {
    UserDefined {
        num_states: u64, // known at time of construction, no need for deferred computation
    },
    HardwareEnable {
        num_states: Deferred<u64>,
    },
    StaticHardwareEnable {
        latency: u64,
        num_states: Deferred<u64>,
        lockstep: Deferred<LockStepAnnotation>,
    },
    StaticRepeat {
        num_repeats: u64,
        body: Box<StatePossibility>,
        annotation: Deferred<RepeatNodeAnnotation>,
        lockstep: Deferred<LockStepAnnotation>,
        num_states: Deferred<u64>,
    },
    StaticPar {
        latency: u64,
        threads: Vec<StatePossibility>,
        lockstep: Deferred<LockStepAnnotation>,
        num_states: Deferred<u64>,
    },
    StaticSeq {
        states: Vec<StatePossibility>,
        lockstep: Deferred<LockStepAnnotation>,
        num_states: Deferred<u64>,
    },
    StaticIf {
        latency: u64,
        true_thread: Option<Box<StatePossibility>>,
        false_thread: Option<Box<StatePossibility>>,
        lockstep: Deferred<LockStepAnnotation>,
        num_states: Deferred<u64>,
    },
    DynamicRepeat {
        num_repeats: u64,
        body: Box<StatePossibility>,
        num_states: Deferred<u64>,
        annotation: Deferred<RepeatNodeAnnotation>,
    },
    DynamicPar {
        threads: Vec<StatePossibility>,
    },
    DynamicSeq {
        states: Vec<StatePossibility>,
        num_states: Deferred<u64>,
    },
    DynamicIf {
        true_thread: Option<Box<StatePossibility>>,
        false_thread: Option<Box<StatePossibility>>,
    },
    DynamicWhile {
        body: Option<Box<StatePossibility>>,
        num_states: Deferred<u64>,
        annotation: Deferred<WhileNodeAnnotation>,
    },
}

/// Implementation for analysis on control tree
impl StatePossibility {
    fn post_order_analysis(&mut self) {
        match self {
            Self::UserDefined { .. } => (),
            Self::HardwareEnable { num_states } => {
                // policy: dynamic enables get one state in parent fsm
                *num_states = Deferred::Computed(1);
            }
            Self::StaticHardwareEnable {
                latency,
                num_states,
                lockstep,
            } => {
                let (num_states_allocated, lockstep_allocated) =
                    Self::static_hardware_enable_policy(*latency);
                *num_states = Deferred::Computed(num_states_allocated);
                *lockstep = Deferred::Computed(lockstep_allocated);
            }
            Self::StaticRepeat {
                num_repeats,
                body,
                num_states,
                annotation,
                lockstep,
            } => {
                body.as_mut().post_order_analysis();
                let (
                    num_states_allocated,
                    repeat_node_annotation,
                    lockstep_annotation,
                ) = body.as_ref().static_repeat_policy(*num_repeats);
                (*num_states, *annotation, *lockstep) = (
                    Deferred::Computed(num_states_allocated),
                    Deferred::Computed(repeat_node_annotation),
                    Deferred::Computed(lockstep_annotation),
                );
            }
            Self::StaticSeq {
                states,
                num_states,
                lockstep,
            } => {
                let states_analyses = states
                    .iter_mut()
                    .map(|state| {
                        state.post_order_analysis();
                        (
                            state.num_states().unwrap(),
                            state.is_lockstep().unwrap(),
                        )
                    })
                    .collect_vec();
                let num_states_seq = states_analyses
                    .iter()
                    .fold(0, |sum, (stmt_num_states, _)| sum + stmt_num_states);
                let lockstep_seq = states_analyses.iter().all(|(_, l)| *l);
                *num_states = Deferred::Computed(num_states_seq);
                *lockstep =
                    Deferred::Computed(LockStepAnnotation::from(lockstep_seq))
            }
            Self::StaticPar {
                latency,
                threads,
                lockstep,
                num_states,
            } => (),
            Self::StaticIf {
                latency,
                true_thread,
                false_thread,
                lockstep,
                num_states,
            } => (),
            Self::DynamicRepeat {
                num_repeats,
                body,
                num_states,
                annotation,
            } => (),
            Self::DynamicPar { threads } => (),
            Self::DynamicSeq { states, num_states } => (),
            Self::DynamicIf {
                true_thread,
                false_thread,
            } => (),
            Self::DynamicWhile {
                body,
                num_states,
                annotation,
            } => (),
        }
    }
}

/// Implementation for transforming traditional ir::Control into StatePossibility tree.
impl StatePossibility {
    fn build_from_static_control(sctrl: &ir::StaticControl) -> Option<Self> {
        match sctrl {
            ir::StaticControl::Empty(_) => None,
            ir::StaticControl::Enable(sen) => {
                let hardware_enable = Self::StaticHardwareEnable {
                    latency: sen.group.borrow().get_latency(),
                    num_states: Deferred::Pending,
                    lockstep: Deferred::Pending,
                };
                Some(hardware_enable)
            }
            ir::StaticControl::Seq(sseq) => {
                let static_seq_states = sseq
                    .stmts
                    .iter()
                    .filter_map(Self::build_from_static_control)
                    .collect();
                let static_seq = Self::StaticSeq {
                    states: static_seq_states,
                    num_states: Deferred::Pending,
                    lockstep: Deferred::Pending,
                };
                Some(static_seq)
            }
            ir::StaticControl::Par(spar) => {
                let static_par_threads = spar
                    .stmts
                    .iter()
                    .filter_map(Self::build_from_static_control)
                    .collect();
                let static_par = Self::StaticPar {
                    latency: spar.latency,
                    threads: static_par_threads,
                    lockstep: Deferred::Pending,
                    num_states: Deferred::Pending,
                };
                Some(static_par)
            }
            ir::StaticControl::If(sif) => {
                let f = |b| Self::build_from_static_control(b).map(Box::new);
                let static_if = Self::StaticIf {
                    latency: sif.latency,
                    true_thread: f(&sif.tbranch),
                    false_thread: f(&sif.tbranch),
                    lockstep: Deferred::Pending,
                    num_states: Deferred::Pending,
                };
                Some(static_if)
            }
            ir::StaticControl::Repeat(srep) => {
                // if body is ir::SC::Empty, will return None
                Self::build_from_static_control(&srep.body).map(|st_poss| {
                    Self::StaticRepeat {
                        num_repeats: srep.num_repeats,
                        body: Box::new(st_poss),
                        num_states: Deferred::Pending,
                        annotation: Deferred::Pending,
                        lockstep: Deferred::Pending,
                    }
                })
            }
            ir::StaticControl::Invoke(_) => {
                unreachable!("Invoke nodes should have been compiled away")
            }
        }
    }

    fn build_from_control(ctrl: &ir::Control) -> Option<Self> {
        let f = |b| Self::build_from_control(b).map(Box::new);
        match ctrl {
            ir::Control::Empty(_) => None,
            ir::Control::Static(sc) => Self::build_from_static_control(sc),
            ir::Control::Enable(_) => {
                let hardware_enable = Self::HardwareEnable {
                    num_states: Deferred::Pending,
                };
                Some(hardware_enable)
            }
            ir::Control::FSMEnable(fsm_en) => {
                let user_defined = Self::UserDefined {
                    num_states: fsm_en.fsm.borrow().num_states(),
                };
                Some(user_defined)
            }
            ir::Control::Seq(dseq) => {
                let dynamic_seq_states = dseq
                    .stmts
                    .iter()
                    .filter_map(Self::build_from_control)
                    .collect();
                let dymamic_seq = Self::DynamicSeq {
                    states: dynamic_seq_states,
                    num_states: Deferred::Pending,
                };
                Some(dymamic_seq)
            }
            ir::Control::Par(dpar) => {
                let dynamic_par_threads = dpar
                    .stmts
                    .iter()
                    .filter_map(Self::build_from_control)
                    .collect();
                let dynamic_par = Self::DynamicPar {
                    threads: dynamic_par_threads,
                };
                Some(dynamic_par)
            }
            ir::Control::If(dif) => {
                let dynamic_if = Self::DynamicIf {
                    true_thread: f(&dif.tbranch),
                    false_thread: f(&dif.fbranch),
                };
                Some(dynamic_if)
            }
            ir::Control::Repeat(drep) => Self::build_from_control(&drep.body)
                .map(|st_poss| Self::DynamicRepeat {
                    num_repeats: drep.num_repeats,
                    body: Box::new(st_poss),
                    num_states: Deferred::Pending,
                    annotation: Deferred::Pending,
                }),
            ir::Control::While(dwhile) => {
                let dynamic_while = Self::DynamicWhile {
                    body: f(&dwhile.body),
                    num_states: Deferred::Pending,
                    annotation: Deferred::Pending,
                };
                Some(dynamic_while)
            }
            ir::Control::Invoke(_) => {
                unreachable!("Invoke nodes should have been compiled away")
            }
        }
    }
}

/// Implementation of helper functions
impl StatePossibility {
    /// Given a control node, returns the number of states allocated for its
    /// child schedule if it has been resolved.
    fn num_states(&self) -> Deferred<u64> {
        match self {
            Self::UserDefined { num_states } => Deferred::Computed(*num_states),
            Self::DynamicIf { .. } | Self::DynamicPar { .. } => {
                Deferred::Computed(1)
            }
            Self::HardwareEnable { num_states }
            | Self::StaticHardwareEnable { num_states, .. }
            | Self::StaticRepeat { num_states, .. }
            | Self::StaticIf { num_states, .. }
            | Self::StaticPar { num_states, .. }
            | Self::StaticSeq { num_states, .. }
            | Self::DynamicSeq { num_states, .. }
            | Self::DynamicRepeat { num_states, .. }
            | Self::DynamicWhile { num_states, .. } => *num_states,
        }
    }

    fn is_lockstep(&self) -> Deferred<bool> {
        match self {
            Self::HardwareEnable { .. }
            | Self::UserDefined { .. }
            | Self::DynamicIf { .. }
            | Self::DynamicPar { .. }
            | Self::DynamicRepeat { .. }
            | Self::DynamicSeq { .. }
            | Self::DynamicWhile { .. } => Deferred::Computed(false),
            Self::StaticHardwareEnable { lockstep, .. }
            | Self::StaticRepeat { lockstep, .. }
            | Self::StaticIf { lockstep, .. }
            | Self::StaticPar { lockstep, .. }
            | Self::StaticSeq { lockstep, .. } => lockstep.map(|l| match l {
                LockStepAnnotation::True => true,
                LockStepAnnotation::False => false,
            }),
        }
    }
}

/// Implementations for policy (i.e. determining annotations on control nodes)
impl StatePossibility {
    /// Given the latency of a static enable, find the number of states allocated
    /// for this enable and whether state progression corresponds to cycle increment.
    #[inline]
    fn static_hardware_enable_policy(
        latency: u64,
    ) -> (u64, LockStepAnnotation) {
        if latency < FSM_STATE_CUTOFF {
            (latency, LockStepAnnotation::True)
        } else {
            (1, LockStepAnnotation::False)
        }
    }

    /// Given the post-analysis body of a static repeat, find a proposed
    /// implementation for the repeat itself. Depending on both the number
    /// of states in the body and whether there are self-loops in the body
    /// (if there are self-loops in the body, then unrolling might allocate too
    /// many registers )
    #[inline]
    fn static_repeat_policy(
        &self,
        num_repeats: u64,
    ) -> (u64, RepeatNodeAnnotation, LockStepAnnotation) {
        let (body_num_states, body_in_lockstep) =
            (self.num_states().unwrap(), self.is_lockstep().unwrap());
        if ((num_repeats * body_num_states) < FSM_STATE_CUTOFF)
            && (body_in_lockstep)
        {
            (
                num_repeats * body_num_states,
                RepeatNodeAnnotation::Unroll,
                LockStepAnnotation::True,
            )
        } else if body_num_states < FSM_STATE_CUTOFF {
            (
                body_num_states,
                RepeatNodeAnnotation::Inline,
                LockStepAnnotation::False,
            )
        } else {
            (1, RepeatNodeAnnotation::Offload, LockStepAnnotation::False)
        }
    }
}

enum RepeatNodeAnnotation {
    Offload,
    Unroll,
    Inline,
}

enum LockStepAnnotation {
    False,
    True,
}

impl From<bool> for LockStepAnnotation {
    fn from(b: bool) -> Self {
        if b { Self::True } else { Self::False }
    }
}

enum WhileNodeAnnotation {
    Inline,
    Offload,
}
