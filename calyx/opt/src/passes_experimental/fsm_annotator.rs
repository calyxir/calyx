use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, VisResult, Visitor,
};
use calyx_ir::{self as ir};
use calyx_utils::CalyxResult;

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
    fn policy(ctrl: &mut Self, child_fsm_cutoff: u64) -> FSMImplementation;
}

impl FSMPolicy for ir::StaticEnable {
    fn policy(
        ctrl: &mut ir::StaticEnable,
        child_fsm_cutoff: u64,
    ) -> FSMImplementation {
        let (num_states, acyclic) = {
            let latency = ctrl.group.borrow().get_latency();
            if latency < child_fsm_cutoff {
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
    fn policy(ctrl: &mut ir::StaticSeq, _: u64) -> FSMImplementation {
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
    fn policy(ctrl: &mut ir::StaticPar, _: u64) -> FSMImplementation {
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
    fn policy(
        ctrl: &mut ir::StaticRepeat,
        child_fsm_cutoff: u64,
    ) -> FSMImplementation {
        let (num_states, acyclic, loop_attr) = {
            let (body_num_states, body_is_acyclic) =
                (get_num_states_static(&ctrl.body), is_acyclic(&ctrl.body));
            let unrolled_num_states = ctrl.num_repeats * body_num_states;
            if body_is_acyclic && (unrolled_num_states < child_fsm_cutoff) {
                (unrolled_num_states, true, LoopAnnotation::Unroll)
            } else if body_num_states < child_fsm_cutoff {
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
    fn policy(ctrl: &mut ir::StaticIf, _: u64) -> FSMImplementation {
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
    fn policy(_ctrl: &mut ir::Enable, _: u64) -> FSMImplementation {
        FSMImplementation {
            num_states: 1,
            acyclic: false,
            loop_attr: None,
        }
    }
}

impl FSMPolicy for ir::Seq {
    fn policy(ctrl: &mut ir::Seq, _: u64) -> FSMImplementation {
        FSMImplementation {
            num_states: ctrl.stmts.iter().map(get_num_states).sum(),
            acyclic: false,
            loop_attr: None,
        }
    }
}

impl FSMPolicy for ir::Par {
    fn policy(_ctrl: &mut ir::Par, _: u64) -> FSMImplementation {
        FSMImplementation {
            num_states: 1,
            acyclic: false,
            loop_attr: None,
        }
    }
}

impl FSMPolicy for ir::If {
    fn policy(_ctrl: &mut ir::If, _: u64) -> FSMImplementation {
        FSMImplementation {
            num_states: 1,
            acyclic: false,
            loop_attr: None,
        }
    }
}

impl FSMPolicy for ir::While {
    fn policy(
        ctrl: &mut ir::While,
        child_fsm_cutoff: u64,
    ) -> FSMImplementation {
        let num_states = ctrl.body.get_attribute(NUM_STATES).unwrap();
        let loop_attr = Some(if num_states < child_fsm_cutoff {
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
    fn policy(
        ctrl: &mut ir::Repeat,
        child_fsm_cutoff: u64,
    ) -> FSMImplementation {
        let num_states = ctrl.body.get_attribute(NUM_STATES).unwrap();
        let loop_attr = Some(if num_states < child_fsm_cutoff {
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

pub struct FSMAnnotator {
    child_fsm_cutoff: u64,
}

impl Named for FSMAnnotator {
    fn name() -> &'static str {
        "fsm-annotator"
    }

    fn description() -> &'static str {
        "annotate a control program, determining how FSMs should be allocated"
    }

    fn opts() -> Vec<PassOpt> {
        vec![PassOpt::new(
            "child-fsm-cutoff",
            "The maximum number of states a child FSM can have, before it is offloaded",
            ParseVal::Num(300),
            PassOpt::parse_num,
        )]
    }
}
impl ConstructVisitor for FSMAnnotator {
    fn from(ctx: &ir::Context) -> CalyxResult<Self> {
        let opts = Self::get_opts(ctx);
        Ok(FSMAnnotator {
            child_fsm_cutoff: opts[&"child-fsm-cutoff"]
                .pos_num()
                .expect("requires non-negative OHE cutoff parameter"),
        })
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
        let FSMImplementation { num_states, .. } =
            ir::Enable::policy(s, self.child_fsm_cutoff);
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
        } = ir::StaticEnable::policy(s, self.child_fsm_cutoff);
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
        } = ir::StaticSeq::policy(s, self.child_fsm_cutoff);
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
        } = ir::StaticPar::policy(s, self.child_fsm_cutoff);
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
        } = ir::StaticIf::policy(s, self.child_fsm_cutoff);
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
        } = ir::StaticRepeat::policy(s, self.child_fsm_cutoff);

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
        let FSMImplementation { num_states, .. } =
            ir::Seq::policy(s, self.child_fsm_cutoff);
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
        let FSMImplementation { num_states, .. } =
            ir::Par::policy(s, self.child_fsm_cutoff);
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
        let FSMImplementation { num_states, .. } =
            ir::If::policy(s, self.child_fsm_cutoff);
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
        } = ir::While::policy(s, self.child_fsm_cutoff);
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
        } = ir::Repeat::policy(s, self.child_fsm_cutoff);
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
