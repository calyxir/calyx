use calyx_ir::{self as ir};

struct FSMCallGraph {
    graph: Vec<AbstractFSM>,
}

impl FSMCallGraph {
    /// Empty FSMCallGraph analysis struct
    fn new() -> Self {
        Self { graph: vec![] }
    }

    #[inline]
    fn new_id(&self) -> u64 {
        self.graph.len().try_into().unwrap()
    }

    #[inline]
    fn register_fsm(&mut self, fsm: AbstractFSM) {
        self.graph.push(fsm);
    }

    /// Recursively builds empty FSMCallGraph struct given a control node. Returns
    /// None if `ctrl` is ir::Empty, Some(HardwareEnable) if `ctrl` is a group ir::Enable,
    /// and Some(Call(...)) if the `ctrl` is a non-leaf node in control tree.
    fn from_control(&mut self, ctrl: &ir::Control) -> Option<StatePossibility> {
        match ctrl {
            ir::Control::Empty(_) => None,
            ir::Control::Enable(_) => Some(StatePossibility::HardwareEnable),
            ir::Control::FSMEnable(_) => {
                self.register_fsm(AbstractFSM::UserDefined);
                Some(StatePossibility::Call(FSMCallGraphNode::UserDefined))
            }
            ir::Control::Seq(seq) => {
                let seq_node = AbstractFSM::Generated {
                    states: seq
                        .stmts
                        .iter()
                        .filter_map(|stmt| self.from_control(stmt))
                        .collect(),
                };
                self.register_fsm(seq_node);
                Some(StatePossibility::Call(FSMCallGraphNode::DynamicSeq {
                    pointer: self.new_id(),
                }))
            }
            ir::Control::Repeat(rep) => match self.from_control(&rep.body) {
                None => None,
                Some(state_possibility) => {
                    let repeat_body_wrapper = FSMCallGraphNode::DynamicRepeat {
                        pointer: Box::new(state_possibility),
                        num_repeats: rep.num_repeats,
                    };
                    Some(StatePossibility::Call(repeat_body_wrapper))
                }
            },
            ir::Control::Par(par) => {
                let par_threads_wrapper = FSMCallGraphNode::DynamicPar {
                    pointers: par
                        .stmts
                        .iter()
                        .filter_map(|stmt| self.from_control(stmt))
                        .collect(),
                };
                Some(StatePossibility::Call(par_threads_wrapper))
            }
            ir::Control::If(ifc) => {
                // avoid code duplication between branches
                let mut f = |branch| {
                    self.from_control(branch)
                        .map(|state_possibility| Box::new(state_possibility))
                };

                let if_branches_wrapper = FSMCallGraphNode::DynamicIf {
                    t_branch_pointer: f(&ifc.tbranch),
                    f_branch_pointer: f(&ifc.fbranch),
                };
                Some(StatePossibility::Call(if_branches_wrapper))
            }

            ir::Control::Invoke(_) => {
                unreachable!("Invoke nodes should have been compiled away")
            }
            _ => None,
            // ir::Control::While(whle) => (),
            // ir::Control::Static(stc) => (),
        }
    }
}

enum AbstractFSM {
    UserDefined, // need to add node_id field (i.e. pointer to actual control node)
    Generated { states: Vec<StatePossibility> },
}

enum StatePossibility {
    HardwareEnable,
    Call(FSMCallGraphNode),
}

enum FSMCallGraphNode {
    UserDefined,
    StaticRepeat {
        body_latency: u64,
        pointer: u64,
        num_repeats: u64,
    },
    StaticPar {
        max_thread_latency: u64,
        pointer: u64,
    },
    StaticSeq {
        latency: u64,
        pointer: u64,
    },
    StaticIf {
        max_branch_latency: u64,
        t_branch_pointer: u64,
        f_branch_pointer: u64,
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
        pointer: u64,
    },
    DynamicRepeat {
        pointer: Box<StatePossibility>,
        num_repeats: u64,
    },
}
