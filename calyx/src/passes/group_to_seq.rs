use crate::analysis::ReadWriteSet;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, CloneName};
use std::collections::BTreeMap;

#[derive(Default)]
/// Transforms a group into a seq of 2 smaller groups, if possible.
/// Currently, in order for a group to be transformed must
/// a) consist of only writes to 2 different non-combination cells (let's
/// call them cell1 and cell2) or the group's done port
/// b) have cell2.go = cell1.done assignment
/// c) have group[done] = cell2.done
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
            match SplitAnalysis::get_split(
                group.assignments.drain(..).collect::<Vec<ir::Assignment>>(),
                group.clone_name(),
                &mut builder,
            ) {
                Ok((group1, group2)) => {
                    let seq = ir::Control::seq(vec![
                        ir::Control::enable(group1),
                        ir::Control::enable(group2),
                    ]);
                    self.group_seq_map.insert(group.clone_name(), seq);
                }
                // If we don't do the transformation, just add back the assignemnts
                // we just drained back into group
                Err(mut assigns) => group.assignments.append(&mut assigns),
            }
        }

        // Add back the groups we drained at the beginning of this method, but
        // filter out the empty groups that were split into smaller groups
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

// For all port reads from name in assignment, returns whether all ports are either stable
// or done.
fn if_name_stable_or_done(assign: &ir::Assignment, name: &ir::Id) -> bool {
    let reads = ReadWriteSet::port_reads(assign);
    reads
        .filter(|port_ref| port_ref.borrow().get_parent_name() == name)
        .all(|port_ref| {
            let atts = &port_ref.borrow().attributes;
            atts.has("stable") || atts.has("done")
        })
}

// Returns true if the cell is a component or a non-combinational primitive
fn comp_or_non_comb(cell: &ir::RRC<ir::Cell>) -> bool {
    match &cell.borrow().prototype {
        ir::CellType::Primitive { is_comb, .. } => !*is_comb,
        ir::CellType::Component { .. } => true,
        _ => false,
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

#[derive(Default)]
///Primarily used to help determine the order cells are executed within
///the group, and if possible, to transform a group into a seq of two smaller groups
pub struct SplitAnalysis {
    // First 2 fields help determine if transformation can be applied. Remaining
    // fields help to apply the transformation.
    ///Holds, (b,a) for assignment of form a.go = b.done,
    done_go: Option<(ir::Id, ir::Id)>,

    ///Holds a for assignment of form group[done] = a.done
    last: Option<ir::Id>,

    /// Holds the go-done assignment, i.e. a.go = b.done
    go_done_asmt: Option<ir::Assignment>,

    /// Holds the first "go" assignment, *if* it is in the form a.go = !a.done ? 1'd1
    first_go_asmt: Option<ir::Assignment>,

    /// Holds the group[done] = done assignment;
    group_done_asmt: Option<ir::Assignment>,

    /// Assignments that write to first cell, unless the assignment is already accounted by a different field
    fst_asmts: Vec<ir::Assignment>,

    /// Assignments that write to second cell, unless the assignment is already accounted by a different field
    snd_asmts: Vec<ir::Assignment>,
}

impl SplitAnalysis {
    /// Based on assigns, returns Ok(group1, group2), where (group1,group2) are
    /// the groups that can be made by splitting assigns. If it is not possible to split
    /// assigns into two groups, then just regurn Err(assigns).
    /// Criteria for being able to split assigns into two groups:
    /// 1) Group must write to exactly 2 cells -- let's call them cell1 and cell2
    /// 2) cell1 and cell2 must be either non-combinational primitives or components
    /// 3) Must have group[done] = cell2.done and cell2.go = cell1.done;
    /// 4) All reads of cell1 must be a stable port or cell1.done.
    pub fn get_split(
        assigns: Vec<ir::Assignment>,
        group_name: ir::Id,
        builder: &mut ir::Builder,
    ) -> Result<(ir::RRC<ir::Group>, ir::RRC<ir::Group>), Vec<ir::Assignment>>
    {
        let signal_on = builder.add_constant(1, 1);

        // Builds ordering. If it cannot build a valid linear ordering of length 2,
        // then returns None, and we stop.
        let mut split_analysis = SplitAnalysis::default();
        let (first, second) = match split_analysis.possible_split(&assigns) {
            None => return Err(assigns),
            Some(order) => order,
        };

        // Sets the first_go_asmt, fst_asmts, snd_asmts group_done_asmt, go_done_asmt
        // fields for split_analysis
        split_analysis.organize_assignments(assigns, &first, &second);

        // If there is assignment in the form first.go = !first.done ? 1'd1,
        // turn this into first.go = 1'd1.
        if let Some(go_asmt) = split_analysis.first_go_asmt {
            let new_go_asmt = builder.build_assignment(
                go_asmt.dst,
                signal_on.borrow().get("out"),
                ir::Guard::True,
            );
            split_analysis.fst_asmts.push(new_go_asmt);
        }

        let go_done = split_analysis.go_done_asmt.unwrap_or_else(|| {
            unreachable!("couldn't find a go-done assignment in {}", group_name)
        });

        let first_group = Self::make_group(
            go_done.src,
            ir::Guard::True,
            split_analysis.fst_asmts,
            builder,
            format!("beg_spl_{}", group_name.id),
        );

        // Pushing second.go = 1'd1 onto snd_asmts
        let cell_go = builder.build_assignment(
            go_done.dst,
            signal_on.borrow().get("out"),
            ir::Guard::True,
        );
        split_analysis.snd_asmts.push(cell_go);

        let group_done = split_analysis.group_done_asmt.unwrap_or_else(|| {
            unreachable!(
                "Couldn't find a group[done] = _.done assignment in {}",
                group_name
            )
        });

        let second_group = Self::make_group(
            group_done.src,
            *group_done.guard,
            split_analysis.snd_asmts,
            builder,
            format!("end_spl_{}", group_name.id),
        );

        Ok((first_group, second_group))
    }

    // Goes through assignments, and properly fills in the fields go_done_asmt,
    // first_go_asmt, fst_asmts, snd_asmts, and group_done_asmt.
    fn organize_assignments(
        &mut self,
        mut assigns: Vec<ir::Assignment>,
        first_cell_name: &ir::Id,
        second_cell_name: &ir::Id,
    ) {
        for asmt in assigns.drain(..) {
            match writes_to_cell(&asmt) {
                Some(cell_name) => {
                    if Self::is_go_done(&asmt) {
                        self.go_done_asmt = Some(asmt);
                    } else if Self::is_specific_go(&asmt, first_cell_name) {
                        self.first_go_asmt = Some(asmt);
                    } else if cell_name == first_cell_name {
                        self.fst_asmts.push(asmt);
                    } else if cell_name == second_cell_name {
                        self.snd_asmts.push(asmt);
                    } else {
                        unreachable!(
                            "Does not write to one of the two \"stateful\" cells"
                            )
                    }
                }
                None => self.group_done_asmt = Some(asmt),
            }
        }
    }

    // Builds ordering for self. Returns true if this is a complete, valid,
    // linear ordering in which all reads from the fist cell are from a
    // stable port, false otherwise.
    pub fn possible_split(
        &mut self,
        asmts: &[ir::Assignment],
    ) -> Option<(ir::Id, ir::Id)> {
        let v = ReadWriteSet::write_set(asmts.iter())
            .map(|cell| cell.clone_name())
            .collect::<Vec<ir::Id>>();

        if v.len() == 2 {
            //Update self.go_done_map and self.last for each asmt in the group.
            for asmt in asmts {
                self.update(asmt);
            }
            if let (Some(last), Some((maybe_first, maybe_last))) =
                (self.last.clone(), self.done_go.clone())
            {
                if maybe_last == last
                // making sure maybe_fist and maybe_last are the only 2 cells written to 
                && v.contains(&maybe_first)
                && v.contains(&maybe_last)
                // making sure that all reads of the first cell are from stable ports 
                && asmts.iter().all(|assign| {
                    if_name_stable_or_done(assign, &maybe_first)
                }) {
                    return Some((maybe_first, maybe_last));
                }
            }
        }
        None
    }

    // For a given asmt, if asmt is a.go = b.done, then we add (b,a) to self.go_done_map.
    // Also if asmt is group[done] = cell.done, sets self.last to Some(cell).
    fn update(&mut self, asmt: &ir::Assignment) {
        let src = asmt.src.borrow();
        let dst = asmt.dst.borrow();
        match (&src.parent, &dst.parent) {
            (
                ir::PortParent::Cell(src_cell),
                ir::PortParent::Cell(dst_cell),
            ) => {
                if src.attributes.has("done")
                    && dst.attributes.has("go")
                    && comp_or_non_comb(&src_cell.upgrade())
                    && comp_or_non_comb(&dst_cell.upgrade())
                {
                    self.done_go = Some((
                        src_cell.upgrade().clone_name(),
                        dst_cell.upgrade().clone_name(),
                    ));
                }
            }
            // src_cell's done writes to group's done
            (ir::PortParent::Cell(src_cell), ir::PortParent::Group(_)) => {
                if dst.name == "done"
                    && src.attributes.has("done")
                    && comp_or_non_comb(&src_cell.upgrade())
                {
                    self.last = Some(src_cell.upgrade().borrow().clone_name())
                }
            }
            // If we encounter anything else, then not of interest to us
            _ => (),
        }
    }
    //Returns whether the given assignment is a go-done assignment
    //i.e. cell1.go = cell2.done.
    pub fn is_go_done(asmt: &ir::Assignment) -> bool {
        let src = asmt.src.borrow();
        let dst = asmt.dst.borrow();
        match (&src.parent, &dst.parent) {
            (ir::PortParent::Cell(_), ir::PortParent::Cell(_)) => {
                src.attributes.has("done") && dst.attributes.has("go")
            }
            _ => false,
        }
    }

    //Returns whether the given assignment writes to the go assignment of cell
    //in the form cell.go = !cell.done? 1'd1.
    pub fn is_specific_go(asmt: &ir::Assignment, cell: &ir::Id) -> bool {
        //checks whether guard is cell.done
        let guard_is_done = |guard: &ir::Guard| -> bool {
            match guard {
                ir::Guard::Port(port) => {
                    port.borrow().attributes.has("done")
                        && port.borrow().get_parent_name() == cell
                }
                _ => false,
            }
        };

        //checks whether guard is !cell.done
        let guard_not_done = |guard: &ir::Guard| -> bool {
            match guard {
                ir::Guard::Not(g) => guard_is_done(&*g),
                _ => false,
            }
        };

        let dst = asmt.dst.borrow();
        // checks cell.go =
        dst.get_parent_name() == cell  && dst.attributes.has("go")
        // checks !cell.done ?
        && guard_not_done(&*asmt.guard)
        // checks 1'd1
        && asmt.src.borrow().is_constant(1, 1)
    }

    /// Returns group with made using builder with prefix. The assignments are
    /// asmts, plus a write to groups's done, based on done_src and done_guard.
    fn make_group(
        done_src: ir::RRC<ir::Port>,
        done_guard: ir::Guard,
        asmts: Vec<ir::Assignment>,
        builder: &mut ir::Builder,
        prefix: String,
    ) -> ir::RRC<ir::Group> {
        let group = builder.add_group(prefix);
        let mut group_asmts = asmts;
        let done_asmt = builder.build_assignment(
            group.borrow().get("done"),
            done_src,
            done_guard,
        );
        group_asmts.push(done_asmt);
        group.borrow_mut().assignments.append(&mut group_asmts);
        group
    }
}
