use crate::analysis::{AssignmentAnalysis, ReadWriteSet};
use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir as ir;
use ir::Nothing;
use std::collections::BTreeMap;

#[derive(Default)]
/// Transforms a group into a seq of 2 smaller groups, if possible.
/// Currently, in order for a group to be transformed must
/// 1) Group must write to exactly 2 cells -- let's call them cell1 and cell2
/// 2) cell1 and cell2 must be either non-combinational primitives or components
/// 3) Must have group[done] = cell2.done and cell2.go = cell1.done;
/// 4) All reads of cell1 must be a stable port or cell1.done.
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
        let groups: Vec<ir::RRC<ir::Group>> =
            comp.get_groups_mut().drain().collect();
        let mut builder = ir::Builder::new(comp, sigs);
        for g in groups.iter() {
            let mut g_ref = g.borrow_mut();
            let group_name = g_ref.name();
            let split_analysis: SplitAnalysis<Nothing> =
                SplitAnalysis::default();
            if let Some((outline1, outline2)) = split_analysis.get_split(
                &mut g_ref.assignments,
                group_name,
                &mut builder,
            ) {
                let g1 = outline1.make_group(
                    &mut builder,
                    format!("beg_spl_{}", g_ref.name().id),
                );
                let g2 = outline2.make_group(
                    &mut builder,
                    format!("end_spl_{}", g_ref.name().id),
                );
                let seq = ir::Control::seq(vec![
                    ir::Control::enable(g1),
                    ir::Control::enable(g2),
                ]);
                self.group_seq_map.insert(group_name, seq);
            }
        }

        // Add back the groups we drained at the beginning of this method, but
        // filter out the empty groups that were split into smaller groups
        comp.get_groups_mut().append(
            groups
                .into_iter()
                .filter(|group| !group.borrow().assignments.is_empty()),
        );

        // // do the same thing with static groups
        // let static_groups: Vec<ir::RRC<ir::StaticGroup>> =
        //     comp.get_static_groups_mut().drain().collect();
        // let mut builder = ir::Builder::new(comp, sigs);
        // for sg in static_groups.iter() {
        //     let split_analysis: SplitAnalysis<StaticTiming> =
        //         SplitAnalysis::default();
        //     if let Some((outline1, outline2)) = split_analysis.get_split(
        //         &mut sg.borrow_mut().assignments,
        //         sg.borrow().name(),
        //         &mut builder,
        //     ) {
        //         let g1 = outline1.make_group_static(
        //             &mut builder,
        //             format!("beg_spl_{}", sg.borrow().name().id),
        //         );
        //         let g2 = outline2.make_group_static(
        //             &mut builder,
        //             format!("end_spl{}", sg.borrow().name().id),
        //         );
        //         let seq = ir::Control::seq(vec![
        //             ir::Control::static_enable(g1),
        //             ir::Control::static_enable(g2),
        //         ]);
        //         self.group_seq_map.insert(sg.borrow().name(), seq);
        //     }
        // }

        // // Add back the groups we drained at the beginning of this method, but
        // // filter out the empty groups that were split into smaller groups
        // comp.get_static_groups_mut()
        //     .append(static_groups.into_iter().filter(|static_group| {
        //         !static_group.borrow().assignments.is_empty()
        //     }));

        Ok(Action::Continue)
    }

    fn enable(
        &mut self,
        s: &mut ir::Enable,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let group_name = s.group.borrow().name();
        match self.group_seq_map.get(&group_name) {
            None => Ok(Action::Continue),
            Some(seq) => Ok(Action::Change(Box::new(ir::Cloner::control(seq)))),
        }
    }
}

// For all port reads from name in assignment, returns whether all ports are either stable
// or done.
fn if_name_stable_or_done<T>(
    assign: &ir::Assignment<T>,
    name: &ir::Id,
) -> bool {
    let reads = ReadWriteSet::port_reads(assign);
    reads
        .filter(|port_ref| port_ref.borrow().get_parent_name() == name)
        .all(|port_ref| {
            let atts = &port_ref.borrow().attributes;
            atts.has(ir::BoolAttr::Stable) || atts.has(ir::NumAttr::Done)
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
fn writes_to_cell<T>(asmt: &ir::Assignment<T>) -> Option<ir::RRC<ir::Cell>> {
    std::iter::once(asmt).analysis().cell_writes().next()
}

///Primarily used to help determine the order cells are executed within
///the group, and if possible, to transform a group into a seq of two smaller groups
struct SplitAnalysis<T>
where
    T: Clone,
{
    /// Holds the go-done assignment, i.e. a.go = b.done
    go_done_asmt: Option<ir::Assignment<T>>,

    /// Holds the first "go" assignment, *if* it is in the form a.go = !a.done ? 1'd1
    first_go_asmt: Option<ir::Assignment<T>>,

    /// Holds the group[done] = done assignment;
    group_done_asmt: Option<ir::Assignment<T>>,

    /// Assignments that write to first cell, unless the assignment is already accounted by a different field
    fst_asmts: Vec<ir::Assignment<T>>,

    /// Assignments that write to second cell, unless the assignment is already accounted by a different field
    snd_asmts: Vec<ir::Assignment<T>>,

    /// Writes to combinational components
    comb_asmts: Vec<ir::Assignment<T>>,
}

impl<T> Default for SplitAnalysis<T>
where
    T: Clone,
{
    fn default() -> Self {
        SplitAnalysis {
            go_done_asmt: None,
            first_go_asmt: None,
            group_done_asmt: None,
            fst_asmts: Vec::default(),
            snd_asmts: Vec::default(),
            comb_asmts: Vec::default(),
        }
    }
}

impl<T> SplitAnalysis<T>
where
    T: Clone,
{
    /// Based on assigns, returns Some(seq), where seq = [group1,group2], which
    /// the groups that can be made by splitting assigns. If it is not possible to split
    /// assigns into two groups, then just regurn None.
    /// Criteria for being able to split assigns into two groups (this criteria
    /// is already specified in group2seq's description as well):
    /// 1) Group must write to exactly 2 cells -- let's call them cell1 and cell2
    /// 2) cell1 and cell2 must be either non-combinational primitives or components
    /// 3) Must have group[done] = cell2.done and cell2.go = cell1.done;
    /// 4) All reads of cell1 must be a stable port or cell1.done.
    pub fn get_split(
        mut self,
        assigns: &mut Vec<ir::Assignment<T>>,
        group_name: ir::Id,
        builder: &mut ir::Builder,
    ) -> Option<(GroupOutline<T>, GroupOutline<T>)> {
        let signal_on = builder.add_constant(1, 1);
        // Builds ordering. If it cannot build a valid linear ordering of length 2,
        // then returns None, and we stop.
        let (first, second) = SplitAnalysis::possible_split(assigns)?;

        // Sets the first_go_asmt, fst_asmts, snd_asmts group_done_asmt, go_done_asmt
        // fields for split_analysis
        self.organize_assignments(assigns, &first, &second);

        // If there is assignment in the form first.go = !first.done ? 1'd1,
        // turn this into first.go = 1'd1.
        if let Some(go_asmt) = self.first_go_asmt {
            let new_go_asmt = builder.build_assignment(
                go_asmt.dst,
                signal_on.borrow().get("out"),
                ir::Guard::True,
            );
            self.fst_asmts.push(new_go_asmt);
        }
        let comb_assigns_clones = self.comb_asmts.clone();
        // writes to comb components should be included in the first group
        self.fst_asmts.extend(comb_assigns_clones);

        let go_done = self.go_done_asmt.unwrap_or_else(|| {
            unreachable!("couldn't find a go-done assignment in {}", group_name)
        });

        // Pushing second.go = 1'd1 onto snd_asmts
        let cell_go = builder.build_assignment(
            go_done.dst,
            signal_on.borrow().get("out"),
            ir::Guard::True,
        );
        self.snd_asmts.push(cell_go);
        // writes to comb assigns should also be in the second group
        self.snd_asmts.extend(self.comb_asmts);

        let group_done = self.group_done_asmt.unwrap_or_else(|| {
            unreachable!(
                "Couldn't find a group[done] = _.done assignment in {}",
                group_name
            )
        });

        let g1_outline: GroupOutline<T> = GroupOutline {
            assignments: self.fst_asmts,
            done_guard: ir::Guard::True,
            done_src: go_done.src,
        };
        let g2_outline: GroupOutline<T> = GroupOutline {
            assignments: self.snd_asmts,
            done_guard: *group_done.guard,
            done_src: group_done.src,
        };
        Some((g1_outline, g2_outline))
    }

    // Goes through assignments, and properly fills in the fields go_done_asmt,
    // first_go_asmt, fst_asmts, snd_asmts, and group_done_asmt.
    fn organize_assignments(
        &mut self,
        assigns: &mut Vec<ir::Assignment<T>>,
        first_cell_name: &ir::Id,
        second_cell_name: &ir::Id,
    ) {
        for asmt in assigns.drain(..) {
            match writes_to_cell(&asmt) {
                Some(cell_ref) => {
                    let cell_name = cell_ref.borrow().name();
                    if Self::is_go_done(&asmt) {
                        self.go_done_asmt = Some(asmt);
                    } else if Self::is_specific_go(&asmt, first_cell_name) {
                        self.first_go_asmt = Some(asmt);
                    } else if cell_name == first_cell_name {
                        self.fst_asmts.push(asmt);
                    } else if cell_name == second_cell_name {
                        self.snd_asmts.push(asmt);
                    } else {
                        // assert that we're writing to a combinational component
                        assert!(cell_ref.borrow().is_comb_cell(), "writes to more than 2 stateful cells: {first_cell_name}, {second_cell_name}, {}", cell_ref.borrow().name());
                        self.comb_asmts.push(asmt);
                    }
                }
                None => self.group_done_asmt = Some(asmt),
            }
        }
    }
    // Builds ordering for self. If there is a possible ordering of asmts that
    // satisfy group2seq's criteria, then return the ordering in the form of
    // Some(cell1, cell2). Otherwise return None.
    pub fn possible_split(
        asmts: &[ir::Assignment<T>],
    ) -> Option<(ir::Id, ir::Id)> {
        let stateful_writes: Vec<ir::Id> = asmts
            .iter()
            .analysis()
            .cell_writes()
            .filter_map(|cell| {
                if cell.borrow().is_comb_cell() {
                    None
                } else {
                    Some(cell.borrow().name())
                }
            })
            .collect();

        if stateful_writes.len() == 2 {
            let (maybe_first, maybe_last, last) =
                Self::look_for_assigns(asmts)?;
            if maybe_last == last
                // making sure maybe_first and maybe_last are the only 2 cells written to
                && stateful_writes.contains(&maybe_first)
                && stateful_writes.contains(&maybe_last)
                // making sure that all reads of the first cell are from stable ports
                && asmts.iter().all(|assign| {
                    if_name_stable_or_done(assign, &maybe_first)
                })
            {
                return Some((maybe_first, maybe_last));
            }
        }
        None
    }
    // Searches thru asmts for an a.go = b.done, or a group[done] = c.done assignment.
    // If we can find examples of such assignments, returns Some(b,a,c).
    // Otherwise returns None.
    fn look_for_assigns(
        asmts: &[ir::Assignment<T>],
    ) -> Option<(ir::Id, ir::Id, ir::Id)> {
        let mut done_go: Option<(ir::Id, ir::Id)> = None;
        let mut last: Option<ir::Id> = None;
        for asmt in asmts {
            let src = asmt.src.borrow();
            let dst = asmt.dst.borrow();
            match (&src.parent, &dst.parent) {
                (
                    ir::PortParent::Cell(src_cell),
                    ir::PortParent::Cell(dst_cell),
                ) => {
                    // a.go = b.done case
                    if src.attributes.has(ir::NumAttr::Done)
                        && dst.attributes.has(ir::NumAttr::Go)
                        && comp_or_non_comb(&src_cell.upgrade())
                        && comp_or_non_comb(&dst_cell.upgrade())
                    {
                        done_go = Some((
                            src_cell.upgrade().borrow().name(),
                            dst_cell.upgrade().borrow().name(),
                        ));
                    }
                }
                (ir::PortParent::Cell(src_cell), ir::PortParent::Group(_)) => {
                    // group[done] = c.done case
                    if dst.name == "done"
                        && src.attributes.has(ir::NumAttr::Done)
                        && comp_or_non_comb(&src_cell.upgrade())
                    {
                        last = Some(src_cell.upgrade().borrow().name())
                    }
                }
                // If we encounter anything else, then not of interest to us
                _ => (),
            }
        }
        let (done, go) = done_go?;
        let last_val = last?;
        Some((done, go, last_val))
    }
    //Returns whether the given assignment is a go-done assignment
    //i.e. cell1.go = cell2.done.
    pub fn is_go_done(asmt: &ir::Assignment<T>) -> bool {
        let src = asmt.src.borrow();
        let dst = asmt.dst.borrow();
        match (&src.parent, &dst.parent) {
            (ir::PortParent::Cell(_), ir::PortParent::Cell(_)) => {
                src.attributes.has(ir::NumAttr::Done)
                    && dst.attributes.has(ir::NumAttr::Go)
            }
            _ => false,
        }
    }

    //Returns whether the given assignment writes to the go assignment of cell
    //in the form cell.go = !cell.done? 1'd1.
    pub fn is_specific_go(asmt: &ir::Assignment<T>, cell: &ir::Id) -> bool {
        let dst = asmt.dst.borrow();
        // checks cell.go =
        dst.get_parent_name() == cell  && dst.attributes.has(ir::NumAttr::Go)
        // checks !cell.done ?
        && asmt.guard.is_not_done(cell)
        // checks 1'd1
        && asmt.src.borrow().is_constant(1, 1)
    }
}

/// Template for a Generic Group (i.e., either regular or static):
/// Includes group's assignments, done guard, and done src.
/// Can't include the done assignment in this struct, since this struct is for *before*
/// we've actually created the group, so we can't refer to the group yet (and we
/// need to refer to the group to create its done port)
/// This is intentional, since if we were to create the group, then it would
/// no longer be generic (we would have to pick either group/static group)
struct GroupOutline<T> {
    assignments: Vec<ir::Assignment<T>>,
    done_guard: ir::Guard<T>,
    done_src: ir::RRC<ir::Port>,
}

impl GroupOutline<Nothing> {
    /// Returns group with made using builder with prefix. The assignments are
    /// self.assignments, plus a write to groups's done, based on done_src and done_guard.
    fn make_group(
        self,
        builder: &mut ir::Builder,
        prefix: String,
    ) -> ir::RRC<ir::Group> {
        let group = builder.add_group(prefix);
        let mut group_asmts = self.assignments;
        let done_asmt = builder.build_assignment(
            group.borrow().get("done"),
            self.done_src,
            self.done_guard,
        );
        group_asmts.push(done_asmt);
        group.borrow_mut().assignments.append(&mut group_asmts);
        group
    }
}

// impl GroupOutline<StaticTiming> {
//     /// Returns group with made using builder with prefix. The assignments are
//     /// self.assignments, plus a write to groups's done, based on done_src and done_guard.
//     fn make_group_static(
//         self,
//         builder: &mut ir::Builder,
//         prefix: String,
//     ) -> ir::RRC<ir::StaticGroup> {
//         panic!("not implemented");
//         let group = builder.add_static_group(prefix, 0);
//         let mut group_asmts = self.assignments;
//         let done_asmt = builder.build_assignment(
//             group.borrow().get(ir::NumAttr::Done),
//             self.done_src,
//             self.done_guard,
//         );
//         group_asmts.push(done_asmt);
//         group.borrow_mut().assignments.append(&mut group_asmts);
//         group
//     }
// }
