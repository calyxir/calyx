use calyx_ir::{self as ir};
use itertools::Itertools;

const FSM_STATE_CUTOFF: u64 = 300;
const NODE_ID: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::NODE_ID);

/// A type to encode the fact that, at the point of translation from an
/// `ir::Control` object to a `StatePossibility` object, the implementation and
/// specific decisions regarding FSM structure is unknown. Only at the point of
/// traversal from leaf to root can we begin to decide the FSM structure (e.g.
/// unrolled, inlined, offloaded bodies of repeat nodes). Equivalent to Option<T>.
#[derive(Copy, Clone, Debug)]

pub enum Deferred<T> {
    Pending,
    Computed(T),
}

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

#[derive(Debug)]
pub enum StatePossibility {
    Empty {
        id: u64,
    },
    UserDefined {
        id: u64,
        num_states: u64, // known at time of construction, no need for deferred computation
    },
    HardwareEnable {
        id: u64,
        num_states: Deferred<u64>,
    },
    StaticHardwareEnable {
        id: u64,
        latency: u64,
        num_states: Deferred<u64>,
        lockstep: Deferred<LockStepAnnotation>,
    },
    StaticRepeat {
        id: u64,
        num_repeats: u64,
        body: Box<StatePossibility>,
        annotation: Deferred<RepeatNodeAnnotation>,
        lockstep: Deferred<LockStepAnnotation>,
        num_states: Deferred<u64>,
    },
    StaticPar {
        id: u64,
        latency: u64,
        threads: Vec<StatePossibility>,
        lockstep: Deferred<LockStepAnnotation>,
        num_states: Deferred<u64>,
    },
    StaticSeq {
        id: u64,
        stmts: Vec<StatePossibility>,
        lockstep: Deferred<LockStepAnnotation>,
        num_states: Deferred<u64>,
    },
    StaticIf {
        id: u64,
        latency: u64,
        true_thread: Box<StatePossibility>,
        false_thread: Box<StatePossibility>,
        lockstep: Deferred<LockStepAnnotation>,
        num_states: Deferred<u64>,
    },
    DynamicRepeat {
        id: u64,
        num_repeats: u64,
        body: Box<StatePossibility>,
        num_states: Deferred<u64>,
        annotation: Deferred<RepeatNodeAnnotation>,
    },
    DynamicPar {
        id: u64,
        threads: Vec<StatePossibility>,
    },
    DynamicSeq {
        id: u64,
        stmts: Vec<StatePossibility>,
        num_states: Deferred<u64>,
    },
    DynamicIf {
        id: u64,
        true_thread: Box<StatePossibility>,
        false_thread: Box<StatePossibility>,
    },
    DynamicWhile {
        id: u64,
        body: Box<StatePossibility>,
        num_states: Deferred<u64>,
        annotation: Deferred<WhileNodeAnnotation>,
    },
}

/// Implementation for analysis on control tree
impl StatePossibility {
    /// Provide annotations on the control nodes present in `self`. These annotations
    /// will have a one-to-one mapping onto the `ir::Control` node from which `self`
    /// was built.
    pub fn post_order_analysis(&mut self) {
        match self {
            Self::Empty { .. } | Self::UserDefined { .. } => (),
            Self::HardwareEnable { num_states, .. } => {
                // policy: dynamic enables get one state in parent fsm
                *num_states = Deferred::Computed(1);
            }
            Self::StaticHardwareEnable {
                latency,
                num_states,
                lockstep,
                ..
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
                ..
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
                ..
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
                ..
            } => {
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
                ..
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
                ..
            } => {
                body.post_order_analysis();
                let (num_states_ann, node_ann, _) =
                    body.repeat_policy(*num_repeats);
                *num_states = Deferred::Computed(num_states_ann);
                *annotation = Deferred::Computed(node_ann);
            }
            Self::DynamicPar { threads, .. } => {
                threads.iter_mut().for_each(Self::post_order_analysis)
            }
            Self::DynamicSeq {
                stmts, num_states, ..
            } => {
                let (_, num_states_ann) = Self::seq_policy(stmts);
                *num_states = Deferred::Computed(num_states_ann);
            }
            Self::DynamicIf {
                true_thread,
                false_thread,
                ..
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
                ..
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

impl StatePossibility {
    pub fn build_from_static_control(
        sctrl: &mut ir::StaticControl,
        id: u64,
    ) -> (Self, u64) {
        match sctrl {
            ir::StaticControl::Empty(empty) => {
                empty.attributes.insert(NODE_ID, id);
                (Self::Empty { id }, id + 1)
            }
            ir::StaticControl::Enable(sen) => {
                sen.attributes.insert(NODE_ID, id);
                (
                    Self::StaticHardwareEnable {
                        id,
                        latency: sen.group.borrow().get_latency(),
                        num_states: Deferred::Pending,
                        lockstep: Deferred::Pending,
                    },
                    id + 1,
                )
            }
            ir::StaticControl::Seq(sseq) => {
                sseq.attributes.insert(NODE_ID, id);
                let mut stmt_id = id + 1;
                let stmts = sseq
                    .stmts
                    .iter_mut()
                    .map(|stmt| {
                        let (child, upd_stmt_id) =
                            Self::build_from_static_control(stmt, stmt_id);
                        stmt_id = upd_stmt_id;
                        child
                    })
                    .collect();
                (
                    Self::StaticSeq {
                        id,
                        stmts,
                        num_states: Deferred::Pending,
                        lockstep: Deferred::Pending,
                    },
                    stmt_id,
                )
            }
            ir::StaticControl::Par(spar) => {
                spar.attributes.insert(NODE_ID, id);
                let mut thread_id = id + 1;
                let threads = spar
                    .stmts
                    .iter_mut()
                    .map(|thread| {
                        let (child, upd_thread_id) =
                            Self::build_from_static_control(thread, thread_id);
                        thread_id = upd_thread_id;
                        child
                    })
                    .collect();
                (
                    Self::StaticPar {
                        id,
                        latency: spar.latency,
                        threads,
                        lockstep: Deferred::Pending,
                        num_states: Deferred::Pending,
                    },
                    thread_id,
                )
            }
            ir::StaticControl::If(sif) => {
                sif.attributes.insert(NODE_ID, id);
                let mut branch_id = id + 1;
                let branches: (Self, Self) =
                    [&mut sif.tbranch, &mut sif.fbranch]
                        .iter_mut()
                        .map(|branch| {
                            let (child, upd_branch_id) =
                                Self::build_from_static_control(
                                    branch, branch_id,
                                );
                            branch_id = upd_branch_id;
                            child
                        })
                        .collect_tuple()
                        .unwrap();

                (
                    Self::StaticIf {
                        id,
                        latency: sif.latency,
                        true_thread: Box::new(branches.0),
                        false_thread: Box::new(branches.1),
                        lockstep: Deferred::Pending,
                        num_states: Deferred::Pending,
                    },
                    branch_id,
                )
            }
            ir::StaticControl::Repeat(srep) => {
                srep.attributes.insert(NODE_ID, id);
                let (child, new_id) =
                    Self::build_from_static_control(&mut srep.body, id + 1);
                (
                    Self::StaticRepeat {
                        id,
                        num_repeats: srep.num_repeats,
                        body: Box::new(child),
                        num_states: Deferred::Pending,
                        annotation: Deferred::Pending,
                        lockstep: Deferred::Pending,
                    },
                    new_id,
                )
            }
            ir::StaticControl::Invoke(_) => {
                unreachable!("Invoke nodes should have been compiled away")
            }
        }
    }

    pub fn build_from_control(ctrl: &mut ir::Control, id: u64) -> (Self, u64) {
        match ctrl {
            ir::Control::Empty(empty) => {
                empty.attributes.insert(NODE_ID, id);
                (Self::Empty { id }, id + 1)
            }
            ir::Control::Static(sc) => Self::build_from_static_control(sc, id),
            ir::Control::Enable(den) => {
                den.attributes.insert(NODE_ID, id);
                (
                    Self::HardwareEnable {
                        id,
                        num_states: Deferred::Pending,
                    },
                    id + 1,
                )
            }
            ir::Control::FSMEnable(fsm_en) => {
                fsm_en.attributes.insert(NODE_ID, id);
                (
                    Self::UserDefined {
                        id,
                        num_states: fsm_en.fsm.borrow().num_states(),
                    },
                    id + 1,
                )
            }
            ir::Control::Seq(dseq) => {
                dseq.attributes.insert(NODE_ID, id);
                let mut stmt_id = id + 1;
                let dynamic_seq_states = dseq
                    .stmts
                    .iter_mut()
                    .map(|stmt| {
                        let (child, upd_stmt_id) =
                            Self::build_from_control(stmt, stmt_id);
                        stmt_id = upd_stmt_id;
                        child
                    })
                    .collect();
                (
                    Self::DynamicSeq {
                        id,
                        stmts: dynamic_seq_states,
                        num_states: Deferred::Pending,
                    },
                    stmt_id,
                )
            }
            ir::Control::Par(dpar) => {
                dpar.attributes.insert(NODE_ID, id);
                let mut thread_id = id + 1;
                let dynamic_par_threads = dpar
                    .stmts
                    .iter_mut()
                    .map(|thread| {
                        let (child, upd_thread_id) =
                            Self::build_from_control(thread, thread_id);
                        thread_id = upd_thread_id;
                        child
                    })
                    .collect();
                (
                    Self::DynamicPar {
                        id,
                        threads: dynamic_par_threads,
                    },
                    thread_id,
                )
            }
            ir::Control::If(dif) => {
                dif.attributes.insert(NODE_ID, id);
                let mut branch_id = id + 1;
                let branches: (Self, Self) =
                    [dif.tbranch.as_mut(), dif.fbranch.as_mut()]
                        .iter_mut()
                        .map(|branch| {
                            let (child, upd_branch_id) =
                                Self::build_from_control(branch, branch_id);
                            branch_id = upd_branch_id;
                            child
                        })
                        .collect_tuple()
                        .unwrap();
                (
                    Self::DynamicIf {
                        id,
                        true_thread: Box::new(branches.0),
                        false_thread: Box::new(branches.1),
                    },
                    branch_id,
                )
            }
            ir::Control::Repeat(drep) => {
                drep.attributes.insert(NODE_ID, id);
                let (child, new_id) =
                    Self::build_from_control(&mut drep.body, id + 1);
                (
                    Self::DynamicRepeat {
                        id,
                        num_repeats: drep.num_repeats,
                        body: Box::new(child),
                        num_states: Deferred::Pending,
                        annotation: Deferred::Pending,
                    },
                    new_id,
                )
            }
            ir::Control::While(dwhile) => {
                dwhile.attributes.insert(NODE_ID, id);
                let (child, new_id) =
                    Self::build_from_control(&mut dwhile.body, id + 1);
                (
                    Self::DynamicWhile {
                        id,
                        body: Box::new(child),
                        num_states: Deferred::Pending,
                        annotation: Deferred::Pending,
                    },
                    new_id,
                )
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
            Self::Empty { .. } => Deferred::Computed(0),
            Self::UserDefined { num_states, .. } => {
                Deferred::Computed(*num_states)
            }
            Self::DynamicIf { .. } | Self::DynamicPar { .. } => {
                Deferred::Computed(1)
            }
            Self::HardwareEnable { num_states, .. }
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
            Self::Empty { .. } => Deferred::Computed(true),
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
    #[inline]
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

    /// policy: the fsms implementing the threads of a static par will
    /// be merged exactly when every thread is lockstep (e.g. no threads
    /// have backedges)
    #[inline]
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
    /// Merge FSMs implmementing threads of the static_if exactly when each
    /// thread is in lockstep (similar to static_par)
    #[inline]
    fn static_if_policy(
        true_branch: &mut Box<StatePossibility>,
        false_branch: &mut Box<StatePossibility>,
        latency: u64,
    ) -> (LockStepAnnotation, u64) {
        if vec![true_branch, false_branch].into_iter().all(|branch| {
            branch.post_order_analysis();
            branch.is_lockstep().unwrap()
        }) {
            (LockStepAnnotation::True, latency)
        } else {
            (LockStepAnnotation::False, 1)
        }
    }
}

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

#[derive(Debug)]
pub enum WhileNodeAnnotation {
    Inline,
    Offload,
}
