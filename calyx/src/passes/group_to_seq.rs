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
pub struct GroupToSeq {
    group_seq_map: BTreeMap<ir::Id, ir::Control>,
}

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

//Given asmt old_group[done] = guard? a.done, return group[done] = guard? a.done.
fn make_go_const(
    builder: &mut ir::Builder,
    asmt: ir::Assignment,
) -> ir::Assignment {
    let con = builder.add_constant(1, 1);
    let src = ir::Port {
        name: ir::Id::new("const 1", None),
        width: 1,
        direction: ir::Direction::Output,
        parent: ir::PortParent::Cell(ir::WRC::from(&con)),
        attributes: ir::Attributes::default(),
    };
    let src_ref = Rc::new(RefCell::new(src));
    builder.build_assignment(asmt.dst, src_ref, ir::Guard::True)
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
        let group_name = group.name().clone();

        //builds ordering. If it cannot build a valid linear ordering of length 2,
        //then returns None, and we stop.
        let (first, second) =
            match order_analysis.get_ordering(&group.assignments) {
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

        //The writes to the first and second cell respectively, excluding the go-done asmt.
        let (mut fst_asmts, mut snd_asmts): (
            Vec<ir::Assignment>,
            Vec<ir::Assignment>,
        ) = (Vec::new(), Vec::new());

        //Holds the go-done assignment
        let mut go_done_asmt: Option<ir::Assignment> = None;

        //Holds the first "go" assignment
        let mut first_go_asmt: Option<ir::Assignment> = None;

        //Holds the group[done] = done assignment;
        let mut group_done_asmt: Option<ir::Assignment> = None;

        for asmt in group.assignments.drain(..) {
            match is_write_to(&asmt) {
                Some(cell_name) => {
                    if order_analysis.is_go_done(&asmt) {
                        go_done_asmt = Some(asmt);
                    } else if order_analysis.writes_to_go(&asmt, &first) {
                        first_go_asmt = Some(asmt);
                    } else if cell_name == first {
                        fst_asmts.push(asmt);
                    } else if cell_name == second {
                        snd_asmts.push(asmt);
                    } else {
                        unreachable!(
                            "Does not write to one of the two \"stateful\" cells"
                        )
                    }
                }
                None => group_done_asmt = Some(asmt),
            }
        }

        //Meant to hold the enable statments that will eventually
        //form the seq that we return
        let mut seq_vec: Vec<ir::Control> = Vec::new();

        //building the first group name's prefix
        let mut prefix = String::from("begin_split_");
        prefix.push_str(&group_name.id);
        let first_group = builder.add_group(prefix);

        match first_go_asmt {
            None => (),
            Some(go_asmt) => {
                let new_go_asmt = make_go_const(&mut builder, go_asmt);
                first_group.borrow_mut().assignments.push(new_go_asmt);
            }
        }

        first_group.borrow_mut().assignments.append(&mut fst_asmts);

        let (group_done, cell_go) = split_go_done(
            &mut builder,
            go_done_asmt.unwrap_or_else(|| {
                unreachable!(
                    "couldn't find a go-done assignment in {}",
                    group_name
                )
            }),
            ir::WRC::from(&first_group),
        );
        first_group.borrow_mut().assignments.push(group_done);

        let mut prefix = String::from("end_split_");
        prefix.push_str(&group_name.id);
        let second_group = builder.add_group(prefix);
        second_group.borrow_mut().assignments.push(cell_go);
        second_group.borrow_mut().assignments.append(&mut snd_asmts);

        let new_done = rename_group_done(
            &mut builder,
            group_done_asmt.unwrap_or_else(|| {
                unreachable!(
                    "Couldn't find a group[done] = _.done assignment in {}",
                    group_name
                )
            }),
            ir::WRC::from(&second_group),
        );
        second_group.borrow_mut().assignments.push(new_done);

        seq_vec.push(ir::Control::enable(first_group));
        seq_vec.push(ir::Control::enable(second_group));

        let seq = ir::Control::seq(seq_vec);
        //self.group_seq_map.insert(group_name, seq);
        Ok(Action::Change(Box::new(seq)))
    }
}
