use crate::analysis::OrderAnalysis;
use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Default)]
/// Transforms a group into a seq of smaller groups, if possible.
/// Currently, in order for a group to be transformed, must a) consist of only
/// writes to non-combination cells or the group's done port, and b) there must be a clear,linear ordering of
/// the execution of each cell in the group by looking at go-done assignments and c) group[done] = cell.done for
/// some cell.
pub struct GroupToSeq;

impl Named for GroupToSeq {
    fn name() -> &'static str {
        "group-to-seq"
    }

    fn description() -> &'static str {
        "split groups under correct conditions"
    }
}

//If asmt is a write to a cell named name returns Some(name).
//If asmt is a write to a group port, returns None.
fn is_write_to(asmt: &ir::Assignment) -> Option<ir::Id> {
    match &asmt.dst.borrow().parent {
        ir::PortParent::Cell(cell) => {
            Some(cell.upgrade().borrow().name().clone())
        }
        ir::PortParent::Group(_) => None,
    }
}

//Given asmt a.go = b.done, return (a1, a2), where a1 is group[done] = b.done,
//and a2 is a.go = 1'd1.
fn split_go_done(
    builder: &mut ir::Builder,
    asmt: ir::Assignment,
    group: ir::WRC<ir::Group>,
) -> (ir::Assignment, ir::Assignment) {
    let con = builder.add_constant(1, 1);
    let src = ir::Port {
        name: ir::Id::new("const 1", None),
        width: 1,
        direction: ir::Direction::Output,
        parent: ir::PortParent::Cell(ir::WRC::from(&con)),
        attributes: ir::Attributes::default(),
    };
    let src_ref = Rc::new(RefCell::new(src));

    let dst = ir::Port {
        name: ir::Id::new("done", None),
        width: 1,
        direction: ir::Direction::Input,
        parent: ir::PortParent::Group(group),
        attributes: ir::Attributes::default(),
    };
    let dst_ref = Rc::new(RefCell::new(dst));
    (
        builder.build_assignment(dst_ref, asmt.src, ir::Guard::True),
        builder.build_assignment(asmt.dst, src_ref, ir::Guard::True),
    )
}

//Given asmt old_group[done] = guard? a.done, return group[done] = guard? a.done.
fn rename_group_done(
    builder: &mut ir::Builder,
    asmt: ir::Assignment,
    group: ir::WRC<ir::Group>,
) -> ir::Assignment {
    let dst = ir::Port {
        name: ir::Id::new("done", None),
        width: 1,
        direction: ir::Direction::Input,
        parent: ir::PortParent::Group(group),
        attributes: ir::Attributes::default(),
    };
    let dst_ref = Rc::new(RefCell::new(dst));
    builder.build_assignment(dst_ref, asmt.src, *asmt.guard)
}

//Gets the name of the port parent
fn get_parent_name(port: &ir::RRC<ir::Port>) -> ir::Id {
    match &port.borrow().parent {
        ir::PortParent::Cell(cell) => cell.upgrade().borrow().name().clone(),
        ir::PortParent::Group(group) => group.upgrade().borrow().name().clone(),
    }
}

impl Visitor for GroupToSeq {
    fn enable(
        &mut self,
        s: &mut ir::Enable,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut order_analysis = OrderAnalysis::default();
        let mut builder = ir::Builder::new(comp, sigs);
        let mut group = s.group.borrow_mut();

        //builds ordering. If it cannot build a complete, linear, valid ordering,
        //then returns None, and we stop.
        let ordering = match order_analysis.get_ordering(&group.assignments) {
            None => return Ok(Action::Continue),
            Some(order) => order,
        };

        //If not all assignments either a) write to a self.cell_of_interest or
        //b) write to group[done], then stops.
        if !group
            .assignments
            .iter()
            .all(|asmt| order_analysis.is_orderable_assignment(asmt))
        {
            return Ok(Action::Continue);
        }

        //If length of ordering == 1, then splitting the group is pointless.
        if ordering.len() > 1 {
            //Maps cell names to assignments that write to them, excluding go-done assignments
            let mut model: BTreeMap<ir::Id, Vec<ir::Assignment>> =
                BTreeMap::new();
            //Maps cell names to go-done assignments. For example, "b" would map to
            //a.go = b.done. We hold these separately from model since these will need
            //to be split into 2 different assignments (in this case group[done] = b.done
            //and a.go = 1'd1) before they are added to the group.
            let mut go_done_asmts: BTreeMap<ir::Id, ir::Assignment> =
                BTreeMap::new();
            //Holds the group[done] = done assignment;
            let mut group_done: Vec<ir::Assignment> = Vec::new();

            ordering.iter().for_each(|cell| {
                model.insert(cell.clone(), Vec::new());
            });

            for asmt in group.assignments.drain(..) {
                match is_write_to(&asmt) {
                    Some(cell_name) => {
                        if order_analysis.is_go_done(&asmt) {
                            go_done_asmts
                                .insert(get_parent_name(&asmt.src), asmt);
                        } else if let Some(cur_asmts) =
                            model.get_mut(&cell_name)
                        {
                            cur_asmts.push(asmt);
                        } else {
                            unreachable!(
                                "Writes to cell that is not in in ordering"
                            )
                        }
                    }
                    None => group_done.push(asmt),
                }
            }

            //Meant to hold the enable statments that will eventually
            //form the seq that we return
            let mut seq_vec: Vec<ir::Control> = Vec::new();

            //When we apply split_go_done() on b.go = a.done to get (group[done] = a.done, b.go = 1'd1)
            //we need somewhere to hold b.go = 1'd1. This is the vec that holds it.
            let mut begin_asmt: Vec<ir::Assignment> = Vec::new();

            for cell in ordering {
                if let Some(asmts) = model.remove(&cell) {
                    //building the group name's prefix
                    let mut prefix = String::from("split_");
                    let group_name = group.name().clone().id;
                    prefix.push_str(&group_name);
                    let group = builder.add_group(prefix);

                    //Should only be empty for the first iteration
                    if !begin_asmt.is_empty() {
                        group
                            .borrow_mut()
                            .assignments
                            .push(begin_asmt.remove(0));
                    }
                    group.borrow_mut().assignments.extend(asmts.into_iter());
                    if let Some(asmt) = go_done_asmts.remove(&cell) {
                        let (group_done, cell_go) = split_go_done(
                            &mut builder,
                            asmt,
                            ir::WRC::from(&group),
                        );
                        group.borrow_mut().assignments.push(group_done);
                        begin_asmt.push(cell_go);
                    } else {
                        //This branch should only be reached for the last assignment
                        if group_done.len() == 1 {
                            let new_assign = rename_group_done(
                                &mut builder,
                                group_done.remove(0),
                                ir::WRC::from(&group),
                            );
                            group.borrow_mut().assignments.push(new_assign);
                        } else {
                            unreachable!(
                                "Should only be 1 done write in the group"
                            )
                        }
                    }
                    seq_vec.push(ir::Control::enable(group));
                } else {
                    unreachable!("each cell in ordering should be in model")
                }
            }
            Ok(Action::Change(Box::new(ir::Control::seq(seq_vec))))
        } else {
            Ok(Action::Continue)
        }
    }
}
