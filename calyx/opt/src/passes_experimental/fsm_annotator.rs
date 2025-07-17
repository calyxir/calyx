use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{self as ir};
use calyx_utils::CalyxResult;

const FSM_STATE_CUTOFF: u64 = 300;
const NUM_STATES: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::NUM_STATES);
const ACYCLIC: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::ACYCLIC);

fn is_acyclic(ctrl: &ir::StaticControl) -> bool {
    ctrl.get_attribute(ACYCLIC).is_some()
}

fn get_num_states(ctrl: &ir::Control) -> u64 {
    ctrl.get_attribute(NUM_STATES).unwrap()
}

fn get_num_states_static(ctrl: &ir::StaticControl) -> u64 {
    ctrl.get_attribute(NUM_STATES).unwrap()
}

enum LoopAnnotation {
    Unroll,
    Inline,
    Offload,
}

struct FSMImplementation {
    num_states: u64,
    acyclic: bool,
    loop_attr: Option<LoopAnnotation>,
}

trait FSMPolicy {
    /// Given a control node, returns a number of states allocated for the FSM,
    /// along with whether there exist backedges in the FSM.
    fn policy(ctrl: &mut Self) -> FSMImplementation;
}

impl FSMPolicy for ir::StaticEnable {
    fn policy(ctrl: &mut ir::StaticEnable) -> FSMImplementation {
        let (num_states, acyclic) = {
            let latency = ctrl.group.borrow().get_latency();
            if latency < FSM_STATE_CUTOFF {
                (latency, true)
            } else {
                (1, false)
            }
        };
        FSMImplementation {
            num_states,
            acyclic,
            loop_attr: None,
        }
    }
}

impl FSMPolicy for ir::StaticSeq {
    fn policy(ctrl: &mut ir::StaticSeq) -> FSMImplementation {
        let (num_states, acyclic) =
            ctrl.stmts
                .iter()
                .fold((0, true), |(num_states, acyclic), stmt| {
                    let stmt_latency = get_num_states_static(stmt);
                    let stmt_acyclic = is_acyclic(stmt);
                    (num_states + stmt_latency, acyclic && stmt_acyclic)
                });
        FSMImplementation {
            num_states,
            acyclic,
            loop_attr: None,
        }
    }
}

impl FSMPolicy for ir::StaticPar {
    fn policy(ctrl: &mut ir::StaticPar) -> FSMImplementation {
        let (num_states, acyclic) = if ctrl.stmts.iter().all(is_acyclic) {
            (ctrl.latency, true)
        } else {
            (1, false)
        };
        FSMImplementation {
            num_states,
            acyclic,
            loop_attr: None,
        }
    }
}

impl FSMPolicy for ir::StaticRepeat {
    fn policy(ctrl: &mut ir::StaticRepeat) -> FSMImplementation {
        let (num_states, acyclic, loop_attr) = {
            let (body_num_states, body_is_acyclic) =
                (get_num_states_static(&ctrl.body), is_acyclic(&ctrl.body));
            let unrolled_num_states = ctrl.num_repeats * body_num_states;
            if body_is_acyclic && (unrolled_num_states < FSM_STATE_CUTOFF) {
                (unrolled_num_states, true, LoopAnnotation::Unroll)
            } else if body_num_states < FSM_STATE_CUTOFF {
                (body_num_states, false, LoopAnnotation::Inline)
            } else {
                (1, false, LoopAnnotation::Offload)
            }
        };
        FSMImplementation {
            num_states,
            acyclic,
            loop_attr: Some(loop_attr),
        }
    }
}

impl FSMPolicy for ir::StaticIf {
    fn policy(ctrl: &mut ir::StaticIf) -> FSMImplementation {
        let (num_states, acyclic) =
            if is_acyclic(&ctrl.tbranch) && is_acyclic(&ctrl.fbranch) {
                (ctrl.latency, true)
            } else {
                (1, false)
            };
        FSMImplementation {
            num_states,
            acyclic,
            loop_attr: None,
        }
    }
}

impl FSMPolicy for ir::Enable {
    fn policy(_ctrl: &mut ir::Enable) -> FSMImplementation {
        FSMImplementation {
            num_states: 1,
            acyclic: false,
            loop_attr: None,
        }
    }
}

impl FSMPolicy for ir::Seq {
    fn policy(ctrl: &mut ir::Seq) -> FSMImplementation {
        FSMImplementation {
            num_states: ctrl.stmts.iter().map(get_num_states).sum(),
            acyclic: false,
            loop_attr: None,
        }
    }
}

impl FSMPolicy for ir::Par {
    fn policy(_ctrl: &mut ir::Par) -> FSMImplementation {
        FSMImplementation {
            num_states: 1,
            acyclic: false,
            loop_attr: None,
        }
    }
}

impl FSMPolicy for ir::If {
    fn policy(_ctrl: &mut ir::If) -> FSMImplementation {
        FSMImplementation {
            num_states: 1,
            acyclic: false,
            loop_attr: None,
        }
    }
}

impl FSMPolicy for ir::While {
    fn policy(ctrl: &mut ir::While) -> FSMImplementation {
        let num_states = ctrl.body.get_attribute(NUM_STATES).unwrap();
        let loop_attr = Some(if num_states < FSM_STATE_CUTOFF {
            LoopAnnotation::Inline
        } else {
            LoopAnnotation::Offload
        });
        FSMImplementation {
            num_states,
            acyclic: false,
            loop_attr,
        }
    }
}

impl FSMPolicy for ir::Repeat {
    fn policy(ctrl: &mut ir::Repeat) -> FSMImplementation {
        let num_states = ctrl.body.get_attribute(NUM_STATES).unwrap();
        let loop_attr = Some(if num_states < FSM_STATE_CUTOFF {
            LoopAnnotation::Inline
        } else {
            LoopAnnotation::Offload
        });
        FSMImplementation {
            num_states,
            acyclic: false,
            loop_attr,
        }
    }
}

pub struct FSMAnnotator {}

impl Named for FSMAnnotator {
    fn name() -> &'static str {
        "fsm-annotator"
    }
    fn description() -> &'static str {
        "annotate a control program, determining how FSMs should be allocated"
    }
}
impl ConstructVisitor for FSMAnnotator {
    fn from(_: &ir::Context) -> CalyxResult<Self> {
        Ok(FSMAnnotator {})
    }
    fn clear_data(&mut self) {}
}

impl Visitor for FSMAnnotator {
    fn empty(
        &mut self,
        s: &mut ir::Empty,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        s.attributes.insert(NUM_STATES, 0);
        Ok(Action::Continue)
    }
    fn enable(
        &mut self,
        s: &mut ir::Enable,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let FSMImplementation { num_states, .. } = ir::Enable::policy(s);
        s.attributes.insert(NUM_STATES, num_states);
        Ok(Action::Continue)
    }

    fn static_enable(
        &mut self,
        s: &mut ir::StaticEnable,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let FSMImplementation {
            num_states,
            acyclic,
            ..
        } = ir::StaticEnable::policy(s);
        s.attributes.insert(NUM_STATES, num_states);
        if acyclic {
            s.attributes.insert(ACYCLIC, 1);
        }
        Ok(Action::Continue)
    }

    fn fsm_enable(
        &mut self,
        s: &mut ir::FSMEnable,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let num_states = s.fsm.borrow().num_states();
        s.attributes.insert(NUM_STATES, num_states);
        Ok(Action::Continue)
    }

    fn finish_static_seq(
        &mut self,
        s: &mut ir::StaticSeq,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let FSMImplementation {
            num_states,
            acyclic,
            ..
        } = ir::StaticSeq::policy(s);
        s.attributes.insert(NUM_STATES, num_states);
        if acyclic {
            s.attributes.insert(ACYCLIC, 1);
        }
        Ok(Action::Continue)
    }

    fn finish_static_par(
        &mut self,
        s: &mut ir::StaticPar,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let FSMImplementation {
            num_states,
            acyclic,
            ..
        } = ir::StaticPar::policy(s);
        s.attributes.insert(NUM_STATES, num_states);
        if acyclic {
            s.attributes.insert(ACYCLIC, 1);
        }
        Ok(Action::Continue)
    }

    fn finish_static_if(
        &mut self,
        s: &mut ir::StaticIf,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let FSMImplementation {
            num_states,
            acyclic,
            ..
        } = ir::StaticIf::policy(s);
        s.attributes.insert(NUM_STATES, num_states);
        if acyclic {
            s.attributes.insert(ACYCLIC, 1);
        }
        Ok(Action::Continue)
    }

    fn finish_static_repeat(
        &mut self,
        s: &mut ir::StaticRepeat,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let FSMImplementation {
            num_states,
            acyclic,
            loop_attr,
        } = ir::StaticRepeat::policy(s);

        s.attributes.insert(NUM_STATES, num_states);

        if acyclic {
            s.attributes.insert(ACYCLIC, 1);
        }

        s.attributes.insert(
            ir::Attribute::Internal(match loop_attr.unwrap() {
                LoopAnnotation::Unroll => ir::InternalAttr::UNROLL,
                LoopAnnotation::Inline => ir::InternalAttr::INLINE,
                LoopAnnotation::Offload => ir::InternalAttr::OFFLOAD,
            }),
            1,
        );
        Ok(Action::Continue)
    }

    fn finish_seq(
        &mut self,
        s: &mut ir::Seq,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let FSMImplementation { num_states, .. } = ir::Seq::policy(s);
        s.attributes.insert(NUM_STATES, num_states);
        Ok(Action::Continue)
    }

    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let FSMImplementation { num_states, .. } = ir::Par::policy(s);
        s.attributes.insert(NUM_STATES, num_states);
        Ok(Action::Continue)
    }

    fn finish_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let FSMImplementation { num_states, .. } = ir::If::policy(s);
        s.attributes.insert(NUM_STATES, num_states);
        Ok(Action::Continue)
    }

    fn finish_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let FSMImplementation {
            num_states,
            loop_attr,
            ..
        } = ir::While::policy(s);
        s.attributes.insert(NUM_STATES, num_states);
        s.attributes.insert(
            ir::Attribute::Internal(match loop_attr.unwrap() {
                LoopAnnotation::Inline => ir::InternalAttr::INLINE,
                LoopAnnotation::Offload => ir::InternalAttr::OFFLOAD,
                LoopAnnotation::Unroll => unreachable!(),
            }),
            1,
        );
        Ok(Action::Continue)
    }

    fn finish_repeat(
        &mut self,
        s: &mut calyx_ir::Repeat,
        _comp: &mut calyx_ir::Component,
        _sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        let FSMImplementation {
            num_states,
            loop_attr,
            ..
        } = ir::Repeat::policy(s);
        s.attributes.insert(NUM_STATES, num_states);
        s.attributes.insert(
            ir::Attribute::Internal(match loop_attr.unwrap() {
                LoopAnnotation::Inline => ir::InternalAttr::INLINE,
                LoopAnnotation::Offload => ir::InternalAttr::OFFLOAD,
                LoopAnnotation::Unroll => unreachable!(),
            }),
            1,
        );
        Ok(Action::Continue)
    }
}
