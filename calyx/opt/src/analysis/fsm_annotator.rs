use calyx_ir::{self as ir};
enum Deferred<T> {
    Pending,
    Computed(T),
}

enum StatePossibility {
    UserDefined {
        num_states: u64, // known at time of construction, no need for deferred computation
    },
    HardwareEnable {
        num_states: Deferred<HardwareEnableAnnotation>,
    },
    StaticHardwareEnable {
        latency: u64,
        num_states: Deferred<HardwareEnableAnnotation>,
    },
    StaticRepeat {
        num_repeats: u64,
        body: Box<StatePossibility>,
        body_num_states: Deferred<u64>,
        annotation: Deferred<RepeatNodeAnnotation>,
    },
    StaticPar {
        latency: u64,
        threads: Vec<StatePossibility>,
        annotation: Deferred<LockStepAnnotation>,
    },
    StaticSeq {
        states: Vec<StatePossibility>,
        num_states: Deferred<u64>,
    },
    StaticIf {
        latency: u64,
        true_thread: Option<Box<StatePossibility>>,
        false_thread: Option<Box<StatePossibility>>,
        annotation: Deferred<LockStepAnnotation>,
    },
    DynamicRepeat {
        num_repeats: u64,
        body: Box<StatePossibility>,
        body_num_states: Deferred<u64>,
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
        body_num_states: Deferred<u64>,
        annotation: Deferred<WhileNodeAnnotation>,
    },
}

impl StatePossibility {
    fn build_from_static_control(sctrl: &ir::StaticControl) -> Option<Self> {
        match sctrl {
            ir::StaticControl::Empty(_) => None,
            ir::StaticControl::Enable(sen) => {
                let hardware_enable = Self::StaticHardwareEnable {
                    latency: sen.group.borrow().get_latency(),
                    num_states: Deferred::Pending,
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
                    annotation: Deferred::Pending,
                };
                Some(static_par)
            }
            ir::StaticControl::If(sif) => {
                let f = |b| Self::build_from_static_control(b).map(Box::new);
                let static_if = Self::StaticIf {
                    latency: sif.latency,
                    true_thread: f(&sif.tbranch),
                    false_thread: f(&sif.tbranch),
                    annotation: Deferred::Pending,
                };
                Some(static_if)
            }
            ir::StaticControl::Repeat(srep) => {
                // if body is ir::SC::Empty, will return None
                Self::build_from_static_control(&srep.body).map(|st_poss| {
                    Self::StaticRepeat {
                        num_repeats: srep.num_repeats,
                        body: Box::new(st_poss),
                        body_num_states: Deferred::Pending,
                        annotation: Deferred::Pending,
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
                    body_num_states: Deferred::Pending,
                    annotation: Deferred::Pending,
                }),
            ir::Control::While(dwhile) => {
                let dynamic_while = Self::DynamicWhile {
                    body: f(&dwhile.body),
                    body_num_states: Deferred::Pending,
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

enum HardwareEnableAnnotation {
    SelfLoop,
    MultiState { num_states: u64 },
}

enum RepeatNodeAnnotation {
    Offload,
    Unroll,
    Inline,
}

enum LockStepAnnotation {
    False,
    True { num_states: u64 },
}

enum WhileNodeAnnotation {
    Inline,
    Offload,
}
