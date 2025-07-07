use calyx_ir::{self as ir};

const FSM_STATE_CUTOFF: u64 = 300;

#[allow(dead_code)]
pub struct FSMCallGraph {
    pub graph: Vec<AbstractFSM>,
}

#[allow(dead_code)]
impl FSMCallGraph {
    /// Empty FSMCallGraph analysis struct
    pub fn new() -> Self {
        Self { graph: vec![] }
    }

    #[inline]
    fn register_fsm(&mut self, fsm: AbstractFSM) -> u64 {
        let id = self.graph.len().try_into().unwrap();
        self.graph.push(fsm);
        id
    }

    /// Given a reference to an `ir::StaticControl`, creates an abstract FSM if
    /// necessary and returns a pointer to that FSM.
    pub fn build_from_static_control(
        &mut self,
        sctrl: &ir::StaticControl,
    ) -> Option<StatePossibility> {
        match sctrl {
            ir::StaticControl::Empty(_) => None,
            ir::StaticControl::Enable(sen) => {
                Some(StatePossibility::HardwareEnable {
                    num_states: sen.group.borrow().get_latency(),
                })
            }
            ir::StaticControl::Seq(sseq) => {
                let sseq_node = AbstractFSM::Generated {
                    states: sseq
                        .stmts
                        .iter()
                        .filter_map(|stmt| self.build_from_static_control(stmt))
                        .collect(),
                };

                let sseq_node_id = self.register_fsm(sseq_node);
                Some(StatePossibility::Call(FSMCallGraphNode::StaticSeq {
                    num_states: None,
                    pointer: sseq_node_id,
                }))
            }
            ir::StaticControl::Repeat(srep) => {
                self.build_from_static_control(&srep.body).map(|st_poss| {
                    let repeat_body_wrapper = FSMCallGraphNode::StaticRepeat {
                        num_states: None,
                        pointer: Box::new(st_poss),
                        num_repeats: srep.num_repeats,
                        annotation: RepeatNodeAnnotation::Offload, // default behavior
                    };
                    StatePossibility::Call(repeat_body_wrapper)
                })
            }
            ir::StaticControl::Par(spar) => {
                let spar_threads_wrapper = FSMCallGraphNode::StaticPar {
                    num_states: None,
                    pointers: spar
                        .stmts
                        .iter()
                        .filter_map(|stmt| self.build_from_static_control(stmt))
                        .collect(),
                };
                Some(StatePossibility::Call(spar_threads_wrapper))
            }
            ir::StaticControl::If(sif) => {
                let mut f = |b| self.build_from_static_control(b).map(Box::new);
                let if_branches_wrapper = FSMCallGraphNode::StaticIf {
                    num_states: None,
                    t_branch_pointer: f(&sif.tbranch),
                    f_branch_pointer: f(&sif.fbranch),
                };
                Some(StatePossibility::Call(if_branches_wrapper))
            }
            ir::StaticControl::Invoke(_) => {
                unreachable!("Invoke nodes should have been compiled away")
            }
        }
    }

    /// Given a reference to an `ir::Control`, creates an abstract FSM if
    /// necessary and returns a pointer to that FSM.
    pub fn build_from_control(
        &mut self,
        ctrl: &ir::Control,
    ) -> Option<StatePossibility> {
        match ctrl {
            ir::Control::Empty(_) => None,
            ir::Control::Static(sctrl) => self.build_from_static_control(sctrl),
            ir::Control::Enable(_) => {
                Some(StatePossibility::HardwareEnable { num_states: 1 })
            }
            ir::Control::FSMEnable(fsm_en) => {
                self.register_fsm(AbstractFSM::UserDefined);
                let fsm_call = FSMCallGraphNode::UserDefined {
                    num_states: fsm_en.fsm.borrow().num_states(),
                };
                Some(StatePossibility::Call(fsm_call))
            }
            ir::Control::Seq(seq) => {
                let seq_node = AbstractFSM::Generated {
                    states: seq
                        .stmts
                        .iter()
                        .filter_map(|stmt| self.build_from_control(stmt))
                        .collect(),
                };
                let seq_node_id = self.register_fsm(seq_node);
                Some(StatePossibility::Call(FSMCallGraphNode::DynamicSeq {
                    num_states: None,
                    pointer: seq_node_id,
                }))
            }
            ir::Control::Repeat(rep) => {
                self.build_from_control(&rep.body).map(|st_poss| {
                    let repeat_body_wrapper = FSMCallGraphNode::DynamicRepeat {
                        num_states: None,
                        pointer: Box::new(st_poss),
                        num_repeats: rep.num_repeats,
                    };
                    StatePossibility::Call(repeat_body_wrapper)
                })
            }
            ir::Control::Par(par) => {
                let par_threads_wrapper = FSMCallGraphNode::DynamicPar {
                    num_states: None,
                    pointers: par
                        .stmts
                        .iter()
                        .filter_map(|stmt| self.build_from_control(stmt))
                        .collect(),
                };
                Some(StatePossibility::Call(par_threads_wrapper))
            }
            ir::Control::If(ifc) => {
                let mut f = |b| self.build_from_control(b).map(Box::new);
                let if_branches_wrapper = FSMCallGraphNode::DynamicIf {
                    num_states: None,
                    t_branch_pointer: f(&ifc.tbranch),
                    f_branch_pointer: f(&ifc.fbranch),
                };
                Some(StatePossibility::Call(if_branches_wrapper))
            }
            ir::Control::While(whl) => {
                self.build_from_control(&whl.body).map(|st_poss| {
                    let while_body_wrapper = FSMCallGraphNode::DynamicWhile {
                        num_states: None,
                        pointer: Box::new(st_poss),
                    };
                    StatePossibility::Call(while_body_wrapper)
                })
            }
            ir::Control::Invoke(_) => {
                unreachable!("Invoke nodes should have been compiled away")
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum AbstractFSM {
    UserDefined, // need to add node_id field (i.e. pointer to actual control node)
    Generated { states: Vec<StatePossibility> },
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum StatePossibility {
    HardwareEnable { num_states: u64 },
    Call(FSMCallGraphNode),
}

///
/// repeat 5 {
///   repeat 3 {
///     A; B; C;
///   }
/// }
///
///
/// DynamicRepeat (
///   DynamicRepeat (
///     DynamicSeq (
///       AbstractFSM [0; 1; 2]
/// )))
///
///

#[allow(unused)]
#[derive(Debug)]
pub enum RepeatNodeAnnotation {
    Unroll,
    Inline,
    Offload,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum FSMCallGraphNode {
    UserDefined {
        num_states: u64,
    },
    StaticRepeat {
        num_states: Option<u64>,
        pointer: Box<StatePossibility>,
        num_repeats: u64,
        annotation: RepeatNodeAnnotation,
    },
    StaticPar {
        num_states: Option<u64>,
        pointers: Vec<StatePossibility>,
    },
    StaticSeq {
        num_states: Option<u64>,
        pointer: u64,
    },
    StaticIf {
        num_states: Option<u64>,
        t_branch_pointer: Option<Box<StatePossibility>>,
        f_branch_pointer: Option<Box<StatePossibility>>,
    },
    DynamicSeq {
        num_states: Option<u64>,
        pointer: u64,
    },
    DynamicPar {
        num_states: Option<u64>,
        pointers: Vec<StatePossibility>,
    },
    DynamicIf {
        num_states: Option<u64>,
        t_branch_pointer: Option<Box<StatePossibility>>,
        f_branch_pointer: Option<Box<StatePossibility>>,
    },
    DynamicWhile {
        num_states: Option<u64>,
        pointer: Box<StatePossibility>,
    },
    DynamicRepeat {
        num_states: Option<u64>,
        pointer: Box<StatePossibility>,
        num_repeats: u64,
    },
}

impl FSMCallGraphNode {
    /// Gets the number of states in the FSM implementing a given control node.
    /// Assumes `postorder_analysis` has been run on the node.
    fn get_states(&self) -> u64 {
        match self {
            Self::UserDefined { num_states } => *num_states,
            Self::StaticRepeat { num_states, .. }
            | Self::StaticPar { num_states, .. }
            | Self::StaticSeq { num_states, .. }
            | Self::StaticIf { num_states, .. }
            | Self::DynamicSeq { num_states, .. }
            | Self::DynamicPar { num_states, .. }
            | Self::DynamicIf { num_states, .. }
            | Self::DynamicWhile { num_states, .. }
            | Self::DynamicRepeat { num_states, .. } => num_states.unwrap(),
        }
    }

    pub fn postorder_analysis(&mut self) {
        match self {
            FSMCallGraphNode::UserDefined { .. } => (),
            FSMCallGraphNode::StaticRepeat {
                num_states, // None, at this point
                pointer,
                num_repeats,
                annotation,
            } => {
                let num_states_repeat_body = num_states; // naming collision

                // given the number of states in the repeat body, determine
                // whether current level's body should be unrolled, inlined, or
                // completely offloaded.
                let impl_repeat = |num_states_body| {
                    if *num_repeats * num_states_body < FSM_STATE_CUTOFF {
                        (
                            RepeatNodeAnnotation::Unroll,
                            Some(*num_repeats * num_states_body),
                        )
                    } else if num_states_body < FSM_STATE_CUTOFF {
                        (RepeatNodeAnnotation::Inline, Some(num_states_body))
                    } else {
                        (RepeatNodeAnnotation::Offload, Some(1))
                    }
                };

                // decide on implementation of body before implementing current level
                match pointer.as_mut() {
                    StatePossibility::HardwareEnable { num_states } => {
                        let (node_annotation, states_in_body) =
                            impl_repeat(*num_states);
                        *annotation = node_annotation;
                        *num_states_repeat_body = states_in_body;
                    }
                    StatePossibility::Call(call) => {
                        call.postorder_analysis();
                        let (node_annotation, states_in_body) =
                            impl_repeat(call.get_states());
                        *annotation = node_annotation;
                        *num_states_repeat_body = states_in_body;
                    }
                }
            }
            FSMCallGraphNode::StaticPar { pointers, .. } => (),
            FSMCallGraphNode::StaticSeq { pointer, .. } => (),
            FSMCallGraphNode::StaticIf {
                t_branch_pointer,
                f_branch_pointer,
                ..
            } => (),
            FSMCallGraphNode::DynamicSeq { pointer, .. } => (),
            FSMCallGraphNode::DynamicPar { pointers, .. } => (),

            FSMCallGraphNode::DynamicIf {
                t_branch_pointer,
                f_branch_pointer,
                ..
            } => (),
            FSMCallGraphNode::DynamicWhile { pointer, .. } => (),
            FSMCallGraphNode::DynamicRepeat {
                pointer,
                num_repeats,
                ..
            } => (),
        }
    }
}
