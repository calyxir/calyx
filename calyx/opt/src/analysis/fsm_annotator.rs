use calyx_ir::{self as ir};

#[allow(unused)]
const FSM_STATE_CUTOFF: u64 = 300;

/// A type to encode the fact that, at the point of translation from an
/// `ir::Control` object to a `StatePossibility` object, the implementation and
/// specific decisions regarding FSM structure is unknown. Only at the point of
/// traversal from leaf to root can we begin to decide the FSM structure (e.g.
/// unrolled, inlined, offloaded bodies of repeat nodes). Equivalent to Option<T>.
#[derive(Copy, Clone)]
#[allow(unused)]
enum Deferred<T> {
    Pending,
    Computed(T),
}

#[allow(unused)]
impl<T: Copy> Deferred<T> {
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
        stmts: Vec<StatePossibility>,
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
        stmts: Vec<StatePossibility>,
        num_states: Deferred<u64>,
    },
    DynamicIf {
        true_thread: Option<Box<StatePossibility>>,
        false_thread: Option<Box<StatePossibility>>,
    },
    DynamicWhile {
        body_opt: Option<Box<StatePossibility>>,
        num_states: Deferred<u64>,
        annotation: Deferred<WhileNodeAnnotation>,
    },
}

#[allow(unused)]
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
                vec![true_thread, false_thread]
                    .into_iter()
                    .for_each(|b_opt| {
                        if let Some(b) = b_opt {
                            b.post_order_analysis();
                        }
                    })
            }
            Self::DynamicWhile {
                body_opt,
                num_states,
                annotation,
            } => {
                let (num_states_ann, node_ann) = body_opt.as_mut().map_or(
                    (0, WhileNodeAnnotation::Inline),
                    |body| {
                        body.post_order_analysis();
                        let body_num_states = body.num_states().unwrap();
                        if body_num_states < FSM_STATE_CUTOFF {
                            (body_num_states, WhileNodeAnnotation::Inline)
                        } else {
                            (1, WhileNodeAnnotation::Offload)
                        }
                    },
                );
                *num_states = Deferred::Computed(num_states_ann);
                *annotation = Deferred::Computed(node_ann);
            }
        }
    }
}

#[allow(unused)]
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
                    stmts: static_seq_states,
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
                    stmts: dynamic_seq_states,
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
                    body_opt: f(&dwhile.body),
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

#[allow(unused)]
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
    fn seq_policy(
        stmts: &mut Vec<StatePossibility>,
    ) -> (LockStepAnnotation, u64) {
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
        threads: &mut Vec<StatePossibility>,
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
        true_branch: &mut Option<Box<StatePossibility>>,
        false_branch: &mut Option<Box<StatePossibility>>,
        latency: u64,
    ) -> (LockStepAnnotation, u64) {
        if vec![true_branch, false_branch]
            .into_iter()
            .filter_map(|branch_opt| {
                branch_opt.as_mut().map(|branch| {
                    branch.post_order_analysis();
                    branch.is_lockstep().unwrap()
                })
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
#[allow(unused)]
enum WhileNodeAnnotation {
    Inline,
    Offload,
}
