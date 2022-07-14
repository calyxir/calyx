use crate::analysis::OrderAnalysis;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, CloneName};
use std::collections::BTreeMap;

#[derive(Default)]
/// Transforms a group into a seq of 2 smaller groups, if possible.
/// Currently, in order for a group to be transformed, must
/// a) consist of only writes to 2 different non-combination cells (let's
/// call them cell1 and cell2) or the group's done port
/// b) must have cell2.go = cell1.done assignment
/// c) group[done] = cell2.done
pub struct GroupToSeq {
    ///Maps names of group to the sequences that will replace them
    group_seq_map: BTreeMap<ir::Id, ir::Control>,
}

impl Named for GroupToSeq {
    fn name() -> &'static str {
        "group2seq"
    }

    fn description() -> &'static str {
        "split groups under correct conditions"
    }
}

//If asmt is a write to a cell named name returns Some(name).
//If asmt is a write to a group port, returns None.
fn writes_to_cell(asmt: &ir::Assignment) -> Option<ir::Id> {
    match &asmt.dst.borrow().parent {
        ir::PortParent::Cell(cell) => {
            Some(cell.upgrade().borrow().clone_name())
        }
        ir::PortParent::Group(_) => None,
    }
}

impl Visitor for GroupToSeq {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let groups: Vec<ir::RRC<ir::Group>> = comp.groups.drain().collect();
        let mut builder = ir::Builder::new(comp, sigs);
        for g in groups.iter() {
            let mut group = g.borrow_mut();
            let group_name = group.clone_name();

            //builds ordering. If it cannot build a valid linear ordering of length 2,
            //then returns None, and we stop.
            let mut order_analysis = OrderAnalysis::default();
            let (first, second) =
                match order_analysis.get_ordering(&group.assignments) {
                    None => continue,
                    Some(order) => order,
                };

            //If not all assignments either a) write to a non-combinational cell or
            //b) write to group[done], then stops.
            if !group
                .assignments
                .iter()
                .all(OrderAnalysis::writes_stateful_group)
            {
                continue;
            }

            //Will hold the writes to the first and second cell respectively, excluding the go-done asmt.
            let (mut fst_asmts, mut snd_asmts): (
                Vec<ir::Assignment>,
                Vec<ir::Assignment>,
            ) = (Vec::new(), Vec::new());

            //Holds the go-done assignment, i.e. a.go = b.done
            let mut go_done_asmt: Option<ir::Assignment> = None;

            //Holds the first "go" assignment, *if* it is in the form a.go = !a.done ? 1'd1
            let mut first_go_asmt: Option<ir::Assignment> = None;

            //Holds the group[done] = done assignment;
            let mut group_done_asmt: Option<ir::Assignment> = None;

            for asmt in group.assignments.drain(..) {
                match writes_to_cell(&asmt) {
                    Some(cell_name) => {
                        if OrderAnalysis::is_go_done(&asmt) {
                            go_done_asmt = Some(asmt);
                        } else if OrderAnalysis::is_specific_go(&asmt, &first) {
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

            //if there is assignment in the form first.go = !first.done ? 1'd1,
            //turn this into first.go = 1'd1.
            match first_go_asmt {
                None => (),
                Some(go_asmt) => {
                    let con = builder.add_constant(1, 1);
                    let src_ref = con.borrow().get("out");
                    let new_go_asmt = builder.build_assignment(
                        go_asmt.dst,
                        src_ref,
                        ir::Guard::True,
                    );
                    first_group.borrow_mut().assignments.push(new_go_asmt);
                }
            }

            first_group.borrow_mut().assignments.append(&mut fst_asmts);

            let go_done = go_done_asmt.unwrap_or_else(|| {
                unreachable!(
                    "couldn't find a go-done assignment in {}",
                    group_name
                )
            });

            let first_done_assignment = builder.build_assignment(
                first_group.borrow().get("done"),
                go_done.src,
                ir::Guard::True,
            );
            first_group
                .borrow_mut()
                .assignments
                .push(first_done_assignment);

            //building second group
            let mut prefix = String::from("end_split_");
            prefix.push_str(&group_name.id);
            let second_group = builder.add_group(prefix);
            //pushing the a.go = 1'd1
            let con = builder.add_constant(1, 1);
            let src_ref = con.borrow().get("out");
            let cell_go =
                builder.build_assignment(go_done.dst, src_ref, ir::Guard::True);
            second_group.borrow_mut().assignments.push(cell_go);
            second_group.borrow_mut().assignments.append(&mut snd_asmts);

            let group_done = group_done_asmt.unwrap_or_else(|| {
                unreachable!(
                    "Couldn't find a group[done] = _.done assignment in {}",
                    group_name
                )
            });
            let second_done_assignment = builder.build_assignment(
                second_group.borrow().get("done"),
                group_done.src,
                *group_done.guard,
            );
            second_group
                .borrow_mut()
                .assignments
                .push(second_done_assignment);

            //creating seq and inserting it into group_seq_map.
            seq_vec.push(ir::Control::enable(first_group));
            seq_vec.push(ir::Control::enable(second_group));
            let seq = ir::Control::seq(seq_vec);
            self.group_seq_map.insert(group_name, seq);
        }

        comp.groups.append(groups.into_iter().filter(
            |group: &ir::RRC<ir::Group>| !group.borrow().assignments.is_empty(),
        ));
        Ok(Action::Continue)
    }

    fn enable(
        &mut self,
        s: &mut ir::Enable,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let group_name = s.group.borrow().clone_name();
        match self.group_seq_map.get(&group_name) {
            None => Ok(Action::Continue),
            Some(seq) => Ok(Action::Change(Box::new(ir::Control::clone(seq)))),
        }
    }
}
