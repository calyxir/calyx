use crate::{
    analysis::{LockStepAnnotation, RepeatNodeAnnotation, StatePossibility},
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
    fn from(_ctx: &ir::Context) -> CalyxResult<Self> {
        Ok(FSMAnnotator {})
    }
    fn clear_data(&mut self) {}
}

impl<'a> FSMAnnotator {
    /// Small code re-use
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

    /// Small code re-use for static control
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
    fn project_onto_control(
        abstract_control: &StatePossibility,
        control: &mut ir::Control,
    ) {
        match abstract_control {
            StatePossibility::Empty { .. }
            | StatePossibility::HardwareEnable { .. }
            | StatePossibility::StaticHardwareEnable { .. } => (),
            StatePossibility::StaticSeq { stmts, .. } => stmts
                .iter()
                .for_each(|stmt| Self::project_onto_control(stmt, control)),
            StatePossibility::StaticPar {
                id,
                threads,
                lockstep,
                ..
            } => {
                if matches!(lockstep.unwrap(), LockStepAnnotation::True) {
                    let attr =
                        ir::Attribute::Internal(ir::InternalAttr::LOCKSTEP);
                    Self::update_node_with_id(control, *id, (attr, 1));
                }
                threads.iter().for_each(|thread| {
                    Self::project_onto_control(thread, control)
                })
            }
            StatePossibility::StaticIf {
                id,
                true_thread,
                false_thread,
                lockstep,
                ..
            } => {
                if matches!(lockstep.unwrap(), LockStepAnnotation::True) {
                    let attr =
                        ir::Attribute::Internal(ir::InternalAttr::LOCKSTEP);
                    Self::update_node_with_id(control, *id, (attr, 1));
                }
                Self::project_onto_control(true_thread, control);
                Self::project_onto_control(false_thread, control);
            }
            StatePossibility::StaticRepeat {
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

            _ => (),
        }
    }

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
}

impl Visitor for FSMAnnotator {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        //
        let mut ctrl_ref = comp.control.borrow_mut();
        let (mut st_poss, _) =
            StatePossibility::build_from_control(&mut ctrl_ref, 0);
        Self::project_onto_control(&st_poss, &mut ctrl_ref);

        println!("BEFORE");

        println!("{:?}", st_poss);

        println!();
        println!("AFTER");

        st_poss.post_order_analysis();

        println!("{:?}", st_poss);
        println!();

        Ok(Action::Continue)
    }
}
