use crate::{
    analysis::{
        AcyclicAnnotation, AnnotatedControlNode, RepeatNodeAnnotation,
        WhileNodeAnnotation,
    },
    traversal::{Action, ConstructVisitor, Named, VisResult, Visitor},
};
use calyx_ir::{self as ir};
use calyx_utils::CalyxResult;

const NODE_ID: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::NODE_ID);
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

impl<'a> FSMAnnotator {
    /// Code abstraction: given an iterator over mutable references to
    /// ir::Control nodes, apply `Self::update_node_with_id`.
    fn map_update_across<I>(
        mut iter: I,
        id: u64,
        (attr, attr_val): (ir::Attribute, u64),
    ) -> Option<()>
    where
        I: Iterator<Item = &'a mut ir::Control>,
    {
        if iter.any(|stmt| {
            matches!(
                Self::update_node_with_id(stmt, id, (attr, attr_val),),
                Some(())
            )
        }) {
            Some(())
        } else {
            None
        }
    }

    /// Code abstraction: given an iterator over mutable references to
    /// ir::StaticControl nodes, apply `Self::update_static_node_with_id`.
    fn map_update_across_static<I>(
        mut iter: I,
        id: u64,
        (attr, attr_val): (ir::Attribute, u64),
    ) -> Option<()>
    where
        I: Iterator<Item = &'a mut ir::StaticControl>,
    {
        if iter.any(|stmt| {
            matches!(
                Self::update_static_node_with_id(stmt, id, (attr, attr_val)),
                Some(())
            )
        }) {
            Some(())
        } else {
            None
        }
    }
}

impl FSMAnnotator {
    /// Given a mutable reference to an `ir::Control` tree, update a specifc node
    /// (identified by `id`) with an attribute.
    fn update_node_with_id(
        ctrl: &mut ir::Control,
        id: u64,
        (attr, attr_val): (ir::Attribute, u64),
    ) -> Option<()> {
        match ctrl.get_attribute(NODE_ID) {
            Some(node_id_val) if node_id_val == id => {
                ctrl.insert_attribute(attr, attr_val);
                Some(())
            }
            _ => match ctrl {
                ir::Control::Empty(_)
                | ir::Control::Enable(_)
                | ir::Control::FSMEnable(_) => None,
                ir::Control::While(whle) => Self::update_node_with_id(
                    &mut whle.body,
                    id,
                    (attr, attr_val),
                ),
                ir::Control::Repeat(repeat) => Self::update_node_with_id(
                    &mut repeat.body,
                    id,
                    (attr, attr_val),
                ),
                ir::Control::Seq(seq) => Self::map_update_across(
                    seq.stmts.iter_mut(),
                    id,
                    (attr, attr_val),
                ),
                ir::Control::Par(par) => Self::map_update_across(
                    par.stmts.iter_mut(),
                    id,
                    (attr, attr_val),
                ),
                ir::Control::If(dif) => Self::map_update_across(
                    vec![dif.tbranch.as_mut(), dif.fbranch.as_mut()]
                        .into_iter(),
                    id,
                    (attr, attr_val),
                ),
                ir::Control::Static(sctrl) => Self::update_static_node_with_id(
                    sctrl,
                    id,
                    (attr, attr_val),
                ),
                ir::Control::Invoke(_) => {
                    unreachable!("Invoke nodes should have been compiled away")
                }
            },
        }
    }

    /// Given a mutable reference to an `ir::StaticControl` tree, update a specifc
    /// node (identified by `id`) with an attribute.
    fn update_static_node_with_id(
        sctrl: &mut ir::StaticControl,
        id: u64,
        (attr, attr_val): (ir::Attribute, u64),
    ) -> Option<()> {
        match sctrl.get_attribute(NODE_ID) {
            Some(node_id_val) if node_id_val == id => {
                sctrl.insert_attribute(attr, attr_val);
                Some(())
            }
            _ => match sctrl {
                ir::StaticControl::Empty(_) | ir::StaticControl::Enable(_) => {
                    None
                }
                ir::StaticControl::Seq(sseq) => Self::map_update_across_static(
                    sseq.stmts.iter_mut(),
                    id,
                    (attr, attr_val),
                ),
                ir::StaticControl::Par(spar) => Self::map_update_across_static(
                    spar.stmts.iter_mut(),
                    id,
                    (attr, attr_val),
                ),
                ir::StaticControl::If(sif) => Self::map_update_across_static(
                    vec![sif.tbranch.as_mut(), sif.fbranch.as_mut()]
                        .into_iter(),
                    id,
                    (attr, attr_val),
                ),
                ir::StaticControl::Repeat(srep) => {
                    Self::update_static_node_with_id(
                        &mut srep.body,
                        id,
                        (attr, attr_val),
                    )
                }
                ir::StaticControl::Invoke(_) => {
                    unreachable!("Invoke nodes should have been compiled away")
                }
            },
        }
    }

    /// Given a reference to the data structure on which analysis occurs,
    /// translate these annotations into the Calyx IR data structure.
    fn project_onto_control(
        abstract_control: &AnnotatedControlNode,
        control: &mut ir::Control,
    ) {
        match abstract_control {
            AnnotatedControlNode::StaticSeq { stmts, .. }
            | AnnotatedControlNode::DynamicSeq { stmts, .. } => stmts
                .iter()
                .for_each(|stmt| Self::project_onto_control(stmt, control)),
            AnnotatedControlNode::StaticPar {
                id,
                threads,
                acyclic,
                ..
            } => {
                if matches!(acyclic.unwrap(), AcyclicAnnotation::True) {
                    let attr =
                        ir::Attribute::Internal(ir::InternalAttr::LOCKSTEP);
                    Self::update_node_with_id(control, *id, (attr, 1));
                }
                threads.iter().for_each(|thread| {
                    Self::project_onto_control(thread, control)
                })
            }
            AnnotatedControlNode::StaticIf {
                id,
                true_thread,
                false_thread,
                acyclic,
                ..
            } => {
                if matches!(acyclic.unwrap(), AcyclicAnnotation::True) {
                    let attr =
                        ir::Attribute::Internal(ir::InternalAttr::LOCKSTEP);
                    Self::update_node_with_id(control, *id, (attr, 1));
                }
                Self::project_onto_control(true_thread, control);
                Self::project_onto_control(false_thread, control);
            }
            AnnotatedControlNode::StaticRepeat {
                id,
                body,
                annotation,
                ..
            }
            | AnnotatedControlNode::DynamicRepeat {
                id,
                body,
                annotation,
                ..
            } => {
                let body_attr =
                    ir::Attribute::Internal(match annotation.unwrap() {
                        RepeatNodeAnnotation::Inline => {
                            ir::InternalAttr::INLINE
                        }
                        RepeatNodeAnnotation::Offload => {
                            ir::InternalAttr::OFFLOAD
                        }
                        RepeatNodeAnnotation::Unroll => {
                            ir::InternalAttr::UNROLL
                        }
                    });
                Self::update_node_with_id(control, *id, (body_attr, 1));
                Self::project_onto_control(body, control);
            }
            AnnotatedControlNode::DynamicWhile {
                id,
                body,
                annotation,
                ..
            } => {
                let body_attr =
                    ir::Attribute::Internal(match annotation.unwrap() {
                        WhileNodeAnnotation::Inline => ir::InternalAttr::INLINE,
                        WhileNodeAnnotation::Offload => {
                            ir::InternalAttr::OFFLOAD
                        }
                    });
                Self::update_node_with_id(control, *id, (body_attr, 1));
                Self::project_onto_control(body, control);
            }
            AnnotatedControlNode::DynamicPar { threads, .. } => threads
                .iter()
                .for_each(|thread| Self::project_onto_control(thread, control)),
            AnnotatedControlNode::DynamicIf {
                true_thread,
                false_thread,
                ..
            } => {
                Self::project_onto_control(true_thread, control);
                Self::project_onto_control(false_thread, control);
            }

            _ => (),
        }
    }
}

impl Visitor for FSMAnnotator {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Build the abstract data structure and analyze nodes at which
        // the annotations should take place
        let mut node: AnnotatedControlNode =
            AnnotatedControlNode::from(&mut *comp.control.borrow_mut());
        node.post_order_analysis();

        // translate these annotations onto ir::Control
        Self::project_onto_control(&node, &mut comp.control.borrow_mut());

        Ok(Action::Continue)
    }
}
