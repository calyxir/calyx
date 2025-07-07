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
                    latency: sseq.latency,
                    pointer: sseq_node_id,
                }))
            }
            ir::StaticControl::Repeat(srep) => {
                self.build_from_static_control(&srep.body).map(|st_poss| {
                    let repeat_body_wrapper = FSMCallGraphNode::StaticRepeat {
                        body_latency: srep.body.get_latency(),
                        pointer: Box::new(st_poss),
                        num_repeats: srep.num_repeats,
                        annotation: NodeAnnotation::Offload, // default behavior
                    };
                    StatePossibility::Call(repeat_body_wrapper)
                })
            }
            ir::StaticControl::Par(spar) => {
                let spar_threads_wrapper = FSMCallGraphNode::StaticPar {
                    max_thread_latency: spar.latency,
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
                    max_branch_latency: sif.latency,
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
            ir::Control::FSMEnable(_) => {
                self.register_fsm(AbstractFSM::UserDefined);
                Some(StatePossibility::Call(FSMCallGraphNode::UserDefined))
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
                    pointer: seq_node_id,
                }))
            }
            ir::Control::Repeat(rep) => {
                self.build_from_control(&rep.body).map(|st_poss| {
                    let repeat_body_wrapper = FSMCallGraphNode::DynamicRepeat {
                        pointer: Box::new(st_poss),
                        num_repeats: rep.num_repeats,
                    };
                    StatePossibility::Call(repeat_body_wrapper)
                })
            }
            ir::Control::Par(par) => {
                let par_threads_wrapper = FSMCallGraphNode::DynamicPar {
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
                    t_branch_pointer: f(&ifc.tbranch),
                    f_branch_pointer: f(&ifc.fbranch),
                };
                Some(StatePossibility::Call(if_branches_wrapper))
            }
            ir::Control::While(whl) => {
                self.build_from_control(&whl.body).map(|st_poss| {
                    let while_body_wrapper = FSMCallGraphNode::DynamicWhile {
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

#[allow(unused)]
#[derive(Debug)]
enum NodeAnnotation {
    Unroll,
    Inline,
    Offload,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum FSMCallGraphNode {
    UserDefined,
    StaticRepeat {
        body_latency: u64,
        pointer: Box<StatePossibility>,
        num_repeats: u64,
        annotation: NodeAnnotation,
    },
    StaticPar {
        max_thread_latency: u64,
        pointers: Vec<StatePossibility>,
    },
    StaticSeq {
        latency: u64,
        pointer: u64,
    },
    StaticIf {
        max_branch_latency: u64,
        t_branch_pointer: Option<Box<StatePossibility>>,
        f_branch_pointer: Option<Box<StatePossibility>>,
    },
    DynamicSeq {
        pointer: u64,
    },
    DynamicPar {
        pointers: Vec<StatePossibility>,
    },
    DynamicIf {
        t_branch_pointer: Option<Box<StatePossibility>>,
        f_branch_pointer: Option<Box<StatePossibility>>,
    },
    DynamicWhile {
        pointer: Box<StatePossibility>,
    },
    DynamicRepeat {
        pointer: Box<StatePossibility>,
        num_repeats: u64,
    },
}

impl FSMCallGraphNode {
    fn postorder_analysis(&mut self) {
        match self {
            FSMCallGraphNode::UserDefined => (),
            FSMCallGraphNode::StaticRepeat {
                pointer,
                num_repeats,
                annotation,
                ..
            } => match pointer.as_mut() {
                StatePossibility::HardwareEnable { num_states } => {
                    if *num_repeats * *num_states < FSM_STATE_CUTOFF {
                        *annotation = NodeAnnotation::Unroll;
                    } else {
                        ()
                    }
                }
                StatePossibility::Call(call) => {}
            },
            FSMCallGraphNode::StaticPar {
                max_thread_latency,
                pointers,
            } => (),
            FSMCallGraphNode::StaticSeq { latency, pointer } => (),
            FSMCallGraphNode::StaticIf {
                max_branch_latency,
                t_branch_pointer,
                f_branch_pointer,
            } => (),
            FSMCallGraphNode::DynamicSeq { pointer } => (),
            FSMCallGraphNode::DynamicPar { pointers } => (),

            FSMCallGraphNode::DynamicIf {
                t_branch_pointer,
                f_branch_pointer,
            } => (),
            FSMCallGraphNode::DynamicWhile { pointer } => (),
            FSMCallGraphNode::DynamicRepeat {
                pointer,
                num_repeats,
            } => (),
        }
    }
}
