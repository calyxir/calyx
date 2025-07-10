use calyx_ir::{self as ir};

#[allow(unused)]
const FSM_STATE_CUTOFF: u64 = 300;

/// A type to encode the fact that, at the point of translation from an
/// `ir::Control` object to a `StatePossibility` object, the implementation and
/// specific decisions regarding FSM structure is unknown. Only at the point of
/// traversal from leaf to root can we begin to decide the FSM structure (e.g.
/// unrolled, inlined, offloaded bodies of repeat nodes). Equivalent to Option<T>.
#[derive(Copy, Clone, Debug)]
#[allow(unused)]
pub enum Deferred<T> {
    Pending,
    Computed(T),
}

#[allow(unused)]

impl<T> Deferred<T>
where
    T: Copy,
{
    fn unwrap(&self) -> T {
        match self {
            Self::Pending => panic!(),
            Self::Computed(v) => *v,
        }
    }
}

#[allow(unused)]
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

#[allow(unused)]
#[derive(Debug)]
pub enum StatePossibility {
    Empty,
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
        stmts: Vec<StatePossibility>,
        lockstep: Deferred<LockStepAnnotation>,
        num_states: Deferred<u64>,
    },
    StaticIf {
        latency: u64,
        true_thread: Box<StatePossibility>,
        false_thread: Box<StatePossibility>,
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
        stmts: Vec<StatePossibility>,
        num_states: Deferred<u64>,
    },
    DynamicIf {
        true_thread: Box<StatePossibility>,
        false_thread: Box<StatePossibility>,
    },
    DynamicWhile {
        body: Box<StatePossibility>,
        num_states: Deferred<u64>,
        annotation: Deferred<WhileNodeAnnotation>,
    },
}

#[allow(unused)]
/// Implementation for analysis on control tree
impl StatePossibility {
    pub fn post_order_analysis(&mut self) {
        match self {
            Self::Empty | Self::UserDefined { .. } => (),
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
                body.post_order_analysis();
                let (
                    num_states_allocated,
                    repeat_node_annotation,
                    lockstep_annotation,
                ) = body.repeat_policy(*num_repeats);
                (*num_states, *annotation, *lockstep) = (
                    Deferred::Computed(num_states_allocated),
                    Deferred::Computed(repeat_node_annotation),
                    Deferred::Computed(lockstep_annotation),
                );
            }
            Self::StaticSeq {
                stmts,
                num_states,
                lockstep,
            } => {
                let (lockstep_ann, num_states_ann) = Self::seq_policy(stmts);

                *num_states = Deferred::Computed(num_states_ann);
                *lockstep = Deferred::Computed(lockstep_ann)
            }
            Self::StaticPar {
                latency,
                threads,
                lockstep,
                num_states,
            } => {
                // policy: the fsms implementing the threads of a static par are
                // be merged when every thread is lockstep (i.e. no threads
                // have backedges)
                let (lockstep_ann, num_states_ann) =
                    Self::static_par_policy(threads, *latency);
                *lockstep = Deferred::Computed(lockstep_ann);
                *num_states = Deferred::Computed(num_states_ann);
            }
            Self::StaticIf {
                latency,
                true_thread,
                false_thread,
                lockstep,
                num_states,
            } => {
                let (lockstep_ann, num_states_ann) =
                    Self::static_if_policy(true_thread, false_thread, *latency);
                *lockstep = Deferred::Computed(lockstep_ann);
                *num_states = Deferred::Computed(num_states_ann);
            }
            Self::DynamicRepeat {
                num_repeats,
                body,
                num_states,
                annotation,
            } => {
                body.post_order_analysis();
                let (num_states_ann, node_ann, _) =
                    body.repeat_policy(*num_repeats);
                *num_states = Deferred::Computed(num_states_ann);
                *annotation = Deferred::Computed(node_ann);
            }
            Self::DynamicPar { threads } => {
                threads.iter_mut().for_each(Self::post_order_analysis)
            }
            Self::DynamicSeq { stmts, num_states } => {
                let (_, num_states_ann) = Self::seq_policy(stmts);
                *num_states = Deferred::Computed(num_states_ann);
            }
            Self::DynamicIf {
                true_thread,
                false_thread,
            } => {
                vec![true_thread, false_thread].into_iter().for_each(
                    |branch| {
                        branch.post_order_analysis();
                    },
                );
            }
            Self::DynamicWhile {
                body,
                num_states,
                annotation,
            } => {
                body.post_order_analysis();
                let (num_states_ann, node_ann) = {
                    let body_num_states = body.num_states().unwrap();
                    if body_num_states < FSM_STATE_CUTOFF {
                        (body_num_states, WhileNodeAnnotation::Inline)
                    } else {
                        (1, WhileNodeAnnotation::Offload)
                    }
                };
                *num_states = Deferred::Computed(num_states_ann);
                *annotation = Deferred::Computed(node_ann);
            }
        }
    }
}

impl From<&ir::StaticControl> for StatePossibility {
    fn from(sctrl: &ir::StaticControl) -> Self {
        match sctrl {
            ir::StaticControl::Empty(_) => Self::Empty,
            ir::StaticControl::Enable(sen) => {
                let hardware_enable = Self::StaticHardwareEnable {
                    latency: sen.group.borrow().get_latency(),
                    num_states: Deferred::Pending,
                    lockstep: Deferred::Pending,
                };
                hardware_enable
            }
            ir::StaticControl::Seq(sseq) => {
                let static_seq_states =
                    sseq.stmts.iter().map(Self::from).collect();
                let static_seq = Self::StaticSeq {
                    stmts: static_seq_states,
                    num_states: Deferred::Pending,
                    lockstep: Deferred::Pending,
                };
                static_seq
            }
            ir::StaticControl::Par(spar) => {
                let static_par_threads =
                    spar.stmts.iter().map(Self::from).collect();
                let static_par = Self::StaticPar {
                    latency: spar.latency,
                    threads: static_par_threads,
                    lockstep: Deferred::Pending,
                    num_states: Deferred::Pending,
                };
                static_par
            }
            ir::StaticControl::If(sif) => {
                let static_if = Self::StaticIf {
                    latency: sif.latency,
                    true_thread: Box::new(Self::from(sif.tbranch.as_ref())),
                    false_thread: Box::new(Self::from(sif.fbranch.as_ref())),
                    lockstep: Deferred::Pending,
                    num_states: Deferred::Pending,
                };
                static_if
            }
            ir::StaticControl::Repeat(srep) => {
                let static_rep = Self::StaticRepeat {
                    num_repeats: srep.num_repeats,
                    body: Box::new(Self::from(srep.body.as_ref())),
                    num_states: Deferred::Pending,
                    annotation: Deferred::Pending,
                    lockstep: Deferred::Pending,
                };
                static_rep
            }
            ir::StaticControl::Invoke(_) => {
                unreachable!("Invoke nodes should have been compiled away")
            }
        }
    }
}

impl From<&ir::Control> for StatePossibility {
    fn from(ctrl: &ir::Control) -> Self {
        match ctrl {
            ir::Control::Empty(_) => Self::Empty,
            ir::Control::Static(sc) => Self::from(sc),
            ir::Control::Enable(_) => {
                let hardware_enable = Self::HardwareEnable {
                    num_states: Deferred::Pending,
                };
                hardware_enable
            }
            ir::Control::FSMEnable(fsm_en) => {
                let user_defined = Self::UserDefined {
                    num_states: fsm_en.fsm.borrow().num_states(),
                };
                user_defined
            }
            ir::Control::Seq(dseq) => {
                let dynamic_seq_states =
                    dseq.stmts.iter().map(Self::from).collect();
                let dymamic_seq = Self::DynamicSeq {
                    stmts: dynamic_seq_states,
                    num_states: Deferred::Pending,
                };
                dymamic_seq
            }
            ir::Control::Par(dpar) => {
                let dynamic_par_threads =
                    dpar.stmts.iter().map(Self::from).collect();
                let dynamic_par = Self::DynamicPar {
                    threads: dynamic_par_threads,
                };
                dynamic_par
            }
            ir::Control::If(dif) => {
                let dynamic_if = Self::DynamicIf {
                    true_thread: Box::new(Self::from(dif.tbranch.as_ref())),
                    false_thread: Box::new(Self::from(dif.fbranch.as_ref())),
                };
                dynamic_if
            }
            ir::Control::Repeat(drep) => {
                let dynamic_rep = Self::DynamicRepeat {
                    num_repeats: drep.num_repeats,
                    body: Box::new(Self::from(drep.body.as_ref())),
                    num_states: Deferred::Pending,
                    annotation: Deferred::Pending,
                };
                dynamic_rep
            }
            ir::Control::While(dwhile) => {
                let dynamic_while = Self::DynamicWhile {
                    body: Box::new(Self::from(dwhile.body.as_ref())),
                    num_states: Deferred::Pending,
                    annotation: Deferred::Pending,
                };
                dynamic_while
            }
            ir::Control::Invoke(_) => {
                unreachable!("Invoke nodes should have been compiled away")
            }
        }
    }
}

#[allow(unused)]
/// Implementation of helper functions
impl StatePossibility {
    /// Given a control node, returns the number of states allocated for its
    /// child schedule if it has been resolved.
    fn num_states(&self) -> Deferred<u64> {
        match self {
            Self::Empty => Deferred::Computed(0),
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
            Self::Empty => Deferred::Computed(true),
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

#[allow(unused)]
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
    fn repeat_policy(
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

    /// policy: number of states allocated for the seq is a summation of
    /// the number of states allocated for the individual statements
    /// policy: a seq proceeds in lockstep iff its individual statements
    /// are all also lockstep
    fn seq_policy(stmts: &mut [StatePossibility]) -> (LockStepAnnotation, u64) {
        let (stmts_num_states, stmts_lockstep): (Vec<_>, Vec<_>) = stmts
            .iter_mut()
            .map(|stmt| {
                stmt.post_order_analysis();
                (stmt.num_states().unwrap(), stmt.is_lockstep().unwrap())
            })
            .unzip();

        let lockstep =
            LockStepAnnotation::from(stmts_lockstep.into_iter().all(|l| l));
        let num_states = stmts_num_states.into_iter().sum();

        (lockstep, num_states)
    }

    fn static_par_policy(
        threads: &mut [StatePossibility],
        latency: u64,
    ) -> (LockStepAnnotation, u64) {
        if threads.iter_mut().all(|thread| {
            thread.post_order_analysis();
            thread.is_lockstep().unwrap()
        }) {
            (LockStepAnnotation::True, latency)
        } else {
            (LockStepAnnotation::False, 1)
        }
    }

    fn static_if_policy(
        true_branch: &mut Box<StatePossibility>,
        false_branch: &mut Box<StatePossibility>,
        latency: u64,
    ) -> (LockStepAnnotation, u64) {
        if vec![true_branch, false_branch]
            .into_iter()
            .map(|branch| {
                branch.post_order_analysis();
                branch.is_lockstep().unwrap()
            })
            .all(|b| b)
        {
            (LockStepAnnotation::True, latency)
        } else {
            (LockStepAnnotation::False, 1)
        }
    }
}

#[allow(unused)]
#[derive(Debug)]
pub enum RepeatNodeAnnotation {
    Offload,
    Unroll,
    Inline,
}
#[derive(Debug)]
pub enum LockStepAnnotation {
    False,
    True,
}

impl From<bool> for LockStepAnnotation {
    fn from(b: bool) -> Self {
        if b { Self::True } else { Self::False }
    }
}
#[allow(unused)]
#[derive(Debug)]
pub enum WhileNodeAnnotation {
    Inline,
    Offload,
}
