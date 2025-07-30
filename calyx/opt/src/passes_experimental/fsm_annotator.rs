use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, VisResult, Visitor,
};

use calyx_ir::{self as ir};

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

fn transfer_attributes(
    attributes_map: &mut ir::Attributes,
    FSMImplementation {
        num_states,
        acyclic,
        attr,
    }: &FSMImplementation,
) {
    attributes_map.insert(NUM_STATES, *num_states);
    let ir_attribute = match attr {
        ControlNodeAnnotation::Inline(inline_impl) => match inline_impl {
            InlineImplementation::StandardInline => ir::InternalAttr::INLINE,
            InlineImplementation::Unroll => ir::InternalAttr::UNROLL,
        },
        ControlNodeAnnotation::Offload => ir::InternalAttr::OFFLOAD,
    };
    attributes_map.insert(ir::Attribute::Internal(ir_attribute), 1);
    if *acyclic {
        attributes_map.insert(ACYCLIC, 1);
    }
}

enum InlineImplementation {
    Unroll,
    StandardInline,
}

enum ControlNodeAnnotation {
    Inline(InlineImplementation),
    Offload,
}

struct FSMImplementation {
    num_states: u64,
    acyclic: bool,
    attr: ControlNodeAnnotation,
}

trait FSMPolicy {
    /// Given a control node, returns a number of states allocated for the FSM,
    /// whether backedges exist in the FSM, and an attribute for this node.
    fn policy(ctrl: &mut Self, child_fsm_cutoff: u64) -> FSMImplementation;
}

impl FSMPolicy for ir::FSMEnable {
    fn policy(_: &mut Self, _: u64) -> FSMImplementation {
        FSMImplementation {
            num_states: 1,
            acyclic: false,
            attr: ControlNodeAnnotation::Offload,
        }
    }
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
            attr: ControlNodeAnnotation::Inline(
                InlineImplementation::StandardInline,
            ),
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
            attr: ControlNodeAnnotation::Inline(
                InlineImplementation::StandardInline,
            ),
        }
    }
}

impl FSMPolicy for ir::StaticPar {
    fn policy(ctrl: &mut ir::StaticPar, _: u64) -> FSMImplementation {
        let (num_states, acyclic, attr) = if ctrl.stmts.iter().all(is_acyclic) {
            (
                ctrl.latency,
                true,
                ControlNodeAnnotation::Inline(
                    InlineImplementation::StandardInline,
                ),
            )
        } else {
            (1, false, ControlNodeAnnotation::Offload)
        };

        FSMImplementation {
            num_states,
            acyclic,
            attr,
        }
    }
}

impl FSMPolicy for ir::StaticRepeat {
    fn policy(
        ctrl: &mut ir::StaticRepeat,
        child_fsm_cutoff: u64,
    ) -> FSMImplementation {
        let (body_num_states, body_is_acyclic) =
            (get_num_states_static(&ctrl.body), is_acyclic(&ctrl.body));
        let unrolled_num_states = ctrl.num_repeats * body_num_states;
        let (num_states, acyclic, attr) =
            if body_is_acyclic && (unrolled_num_states < child_fsm_cutoff) {
                (
                    unrolled_num_states,
                    true,
                    ControlNodeAnnotation::Inline(InlineImplementation::Unroll),
                )
            } else if body_num_states < child_fsm_cutoff {
                (
                    body_num_states,
                    false,
                    ControlNodeAnnotation::Inline(
                        InlineImplementation::StandardInline,
                    ),
                )
            } else {
                (1, false, ControlNodeAnnotation::Offload)
            };

        FSMImplementation {
            num_states,
            acyclic,
            attr,
        }
    }
}

impl FSMPolicy for ir::StaticIf {
    fn policy(ctrl: &mut ir::StaticIf, _: u64) -> FSMImplementation {
        let (num_states, acyclic, attr) =
            if is_acyclic(&ctrl.tbranch) && is_acyclic(&ctrl.fbranch) {
                (
                    ctrl.latency,
                    true,
                    ControlNodeAnnotation::Inline(
                        InlineImplementation::StandardInline,
                    ),
                )
            } else {
                (1, false, ControlNodeAnnotation::Offload)
            };

        FSMImplementation {
            num_states,
            acyclic,
            attr,
        }
    }
}

impl FSMPolicy for ir::Enable {
    fn policy(_ctrl: &mut ir::Enable, _: u64) -> FSMImplementation {
        FSMImplementation {
            num_states: 1,
            acyclic: false,
            attr: ControlNodeAnnotation::Inline(
                InlineImplementation::StandardInline,
            ),
        }
    }
}

impl FSMPolicy for ir::Seq {
    fn policy(ctrl: &mut ir::Seq, _: u64) -> FSMImplementation {
        FSMImplementation {
            num_states: ctrl.stmts.iter().map(get_num_states).sum(),
            acyclic: false,
            attr: ControlNodeAnnotation::Inline(
                InlineImplementation::StandardInline,
            ),
        }
    }
}

impl FSMPolicy for ir::Par {
    fn policy(_ctrl: &mut ir::Par, _: u64) -> FSMImplementation {
        FSMImplementation {
            num_states: 1,
            acyclic: false,
            attr: ControlNodeAnnotation::Offload,
        }
    }
}

impl FSMPolicy for ir::If {
    fn policy(_ctrl: &mut ir::If, _: u64) -> FSMImplementation {
        FSMImplementation {
            num_states: 1,
            acyclic: false,
            attr: ControlNodeAnnotation::Offload,
        }
    }
}

impl FSMPolicy for ir::While {
    fn policy(ctrl: &mut ir::While, _: u64) -> FSMImplementation {
        FSMImplementation {
            num_states: ctrl.body.get_attribute(NUM_STATES).unwrap(),
            acyclic: false,
            attr: ControlNodeAnnotation::Inline(
                InlineImplementation::StandardInline,
            ),
        }
    }
}

impl FSMPolicy for ir::Repeat {
    fn policy(ctrl: &mut ir::Repeat, _: u64) -> FSMImplementation {
        FSMImplementation {
            num_states: ctrl.body.get_attribute(NUM_STATES).unwrap(),
            acyclic: false,
            attr: ControlNodeAnnotation::Inline(
                InlineImplementation::StandardInline,
            ),
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
    fn from(ctx: &ir::Context) -> calyx_utils::CalyxResult<Self> {
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
        let fsm_impl = ir::Enable::policy(s, self.child_fsm_cutoff);
        transfer_attributes(&mut s.attributes, &fsm_impl);
        Ok(Action::Continue)
    }

    fn static_enable(
        &mut self,
        s: &mut ir::StaticEnable,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let fsm_impl = ir::StaticEnable::policy(s, self.child_fsm_cutoff);
        transfer_attributes(&mut s.attributes, &fsm_impl);
        Ok(Action::Continue)
    }

    fn fsm_enable(
        &mut self,
        s: &mut ir::FSMEnable,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let fsm_impl = ir::FSMEnable::policy(s, self.child_fsm_cutoff);
        transfer_attributes(&mut s.attributes, &fsm_impl);
        Ok(Action::Continue)
    }

    fn finish_static_seq(
        &mut self,
        s: &mut ir::StaticSeq,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let fsm_impl = ir::StaticSeq::policy(s, self.child_fsm_cutoff);
        transfer_attributes(&mut s.attributes, &fsm_impl);
        Ok(Action::Continue)
    }

    fn finish_static_par(
        &mut self,
        s: &mut ir::StaticPar,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let fsm_impl = ir::StaticPar::policy(s, self.child_fsm_cutoff);
        transfer_attributes(&mut s.attributes, &fsm_impl);
        Ok(Action::Continue)
    }

    fn finish_static_if(
        &mut self,
        s: &mut ir::StaticIf,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let fsm_impl = ir::StaticIf::policy(s, self.child_fsm_cutoff);
        transfer_attributes(&mut s.attributes, &fsm_impl);
        Ok(Action::Continue)
    }

    fn finish_static_repeat(
        &mut self,
        s: &mut ir::StaticRepeat,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let fsm_impl = ir::StaticRepeat::policy(s, self.child_fsm_cutoff);
        transfer_attributes(&mut s.attributes, &fsm_impl);
        Ok(Action::Continue)
    }

    fn finish_seq(
        &mut self,
        s: &mut ir::Seq,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let fsm_impl = ir::Seq::policy(s, self.child_fsm_cutoff);
        transfer_attributes(&mut s.attributes, &fsm_impl);
        Ok(Action::Continue)
    }

    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let fsm_impl = ir::Par::policy(s, self.child_fsm_cutoff);
        transfer_attributes(&mut s.attributes, &fsm_impl);
        Ok(Action::Continue)
    }

    fn finish_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let fsm_impl = ir::If::policy(s, self.child_fsm_cutoff);
        transfer_attributes(&mut s.attributes, &fsm_impl);
        Ok(Action::Continue)
    }

    fn finish_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let fsm_impl = ir::While::policy(s, self.child_fsm_cutoff);
        transfer_attributes(&mut s.attributes, &fsm_impl);
        Ok(Action::Continue)
    }

    fn finish_repeat(
        &mut self,
        s: &mut calyx_ir::Repeat,
        _comp: &mut calyx_ir::Component,
        _sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        let fsm_impl = ir::Repeat::policy(s, self.child_fsm_cutoff);
        transfer_attributes(&mut s.attributes, &fsm_impl);
        Ok(Action::Continue)
    }
}
