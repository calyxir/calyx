use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, VisResult, Visitor,
};
use calyx_ir::{self as ir};

/// From the perspective of a parent control node, the annotation `@NUM_STATES(n)`
/// on one of its child nodes means that the parent should allocate `n` states
/// in order to implement the child's schedule.
const NUM_STATES: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::NUM_STATES);

/// From the perspective of a parent control node, if the attribute `@ACYCLIC` is
/// present on one of its children, then the following implication is guaranteed:
///
/// If the child's FSM has an @UNROLL or @INLINE attribute, then
/// the states implementing the child's schedule should have the property of
/// having one state for every cycle.
///
/// If the attribute `@ACYCLIC` is not present on a child node, then this simply
/// means that the states implementing the child's control has a backedge.
///
/// For now, it is undefined behavior for one child to have both `@ACYCLIC` and
/// `@OFFLOAD` attributes attached.
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

/// Given the attributes field of a control node, insert the annotations derived
/// in this pass into the `control {...}` section of a Calyx program.
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

/// Encodes the possible FSM implementations of a control node. From the perspective
/// of a parent FSM, the states implementing each of its children can
/// either be brought into the parent FSM (i.e. inlined) or can be outsourced
/// to another FSM (offloaded).
enum ControlNodeAnnotation {
    Inline(InlineImplementation),
    Offload,
}

/// Encodes the possible ways for a control node's FSM to be inlined into that of its parent.
/// Here, `Unroll` can only be used on ir::Repeat or ir::StaticRepeat nodes. In the event
/// that `StandardInline` is used on one of these nodes, then, within the parent
/// FSM, there will be a backedge from the bottom to the top of the repeat.
///
/// The annotation `StandardInline` is used to indicate inlining for every other
/// control node.
enum InlineImplementation {
    Unroll,
    StandardInline,
}

/// An instance of this struct is computed for every control node. It will be used
/// by its parent control node.
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

/// For each of the following three bullets, this pass will annotate each
/// control node in a program with one of the given possibilities in that bullet:
///
/// - num_states: in the attribute @NUM_STATES(n), either n = 1 or n is an arbitrary
/// - acyclic: either the node has an @ACYCLIC attribute or it does not
/// - attr: one of the following will appear on the node: @UNROLL, @INLINE, @OFFLOAD
///
/// Then, in total, there are 12 possibilities for annotations on any given
/// control node, but some cases are not possible or can be combined.
/// We enumerate the cases and list the complete set of nodes on
/// which these annotations can appear:
///
///     @NUM_STATES(n) @ACYCLIC @UNROLL -- possible only for:
///         - static repeat
///
///     @NUM_STATES(n) @ACYCLIC @INLINE -- possible only for:
///         - static enable
///         - static seq
///         - static par
///         - static if
///
///     @NUM_STATES(n) @ACYCLIC @OFFLOAD
///         - not possible: @ACYCLIC and @OFFLOAD cannot exist together
///
///     @NUM_STATES(n) @UNROLL -- possible only for:
///         - dynamic repeat
///
///     @NUM_STATES(n) @INLINE -- possible only for:
///         - while
///         - static / dynamic repeat
///         - static / dynamic enable
///         - static / dynamic seq
///
///     @NUM_STATES(1) @OFFLOAD -- possible only for:
///         - while
///         - fsm enable
///         - static / dynamic par
///         - static / dynamic if
///         - static / dynamic repeat
///         - static / dynamic seq
///
///     @NUM_STATES(n) @OFFLOAD
///         - not possible: if a child FSM is offloaded, it cannot occupy an
///           arbitrary number of states in the parent
///
/// A strange quirk about the @offload annotation is that it does not specify
/// how the child schedule should be implemented. For dynamic schedules,
/// @OFFLOAD just means the parent should wait for the [done] of the child, and
/// for static schedules, it means the parent should wait the correct number of
/// cycles. It gives no indication about how the child should be implemented.
/// This is an area of improvement for the above attribute system.
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
