use crate::analysis::ReadWriteSet;
use crate::ir::{self, CloneName};

#[derive(Default)]
///Primarily used to help determine the order cells are executed within
///the group
pub struct OrderAnalysis {
    ///Holds, (b,a) for assignment of form a.go = b.done,
    done_go: Option<(ir::Id, ir::Id)>,
    ///Holds a for assignment of form group[done] = a.done
    last: Option<ir::Id>,
}

// If assignment reads from name, returns whether source port is either stable
// or done. If assignment's source is something else, returns true.
fn if_name_stable_or_done(assign: &ir::Assignment, name: &ir::Id) -> bool {
    let mut reads = ReadWriteSet::port_reads(assign);
    reads.all(|port_ref| {
        let port = port_ref.borrow();
        if port.get_parent_name() == name {
            port.attributes.has("stable") || port.attributes.has("done")
        } else {
            true
        }
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

impl OrderAnalysis {
    //Returns whether the given assignment is a go done assignment from two cells
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

    /// Wheters whether amt writes to a component or non-combinational primitive,
    /// *or* writes to the group's done port.
    pub fn is_expected_write(asmt: &ir::Assignment) -> bool {
        match &asmt.dst.borrow().parent {
            ir::PortParent::Cell(cell) => comp_or_non_comb(&cell.upgrade()),
            ir::PortParent::Group(_) => asmt.dst.borrow().name == "done",
        }
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
                if src.attributes.has("done") && dst.attributes.has("go") {
                    self.done_go = Some((
                        src_cell.upgrade().borrow().clone_name(),
                        dst_cell.upgrade().borrow().clone_name(),
                    ));
                }
            }
            // src_cell's done writes to group's done
            (ir::PortParent::Cell(src_cell), ir::PortParent::Group(_)) => {
                if dst.name == "done" && src.attributes.has("done") {
                    self.last = Some(src_cell.upgrade().borrow().clone_name())
                }
            }
            // If we encounter anything else, then not of interest to us
            _ => (),
        }
    }

    // Builds ordering for self. Returns true if this is a complete, valid,
    // linear ordering in which all reads from the fist cell are from a
    // stable port, false otherwise.
    pub fn get_ordering(
        &mut self,
        asmts: &Vec<ir::Assignment>,
    ) -> Option<(ir::Id, ir::Id)> {
        //Update self.go_done_map and self.last for each asmt in the group.
        for asmt in asmts {
            self.update(asmt);
        }
        //Build ordering of cells, based on self.done_go and self.last.
        if let (Some(last), Some((maybe_first, maybe_last))) =
            (self.last.clone(), self.done_go.clone())
        {
            let all_stateful_writes =
                ReadWriteSet::write_set(asmts.iter()).filter(comp_or_non_comb);
            if maybe_last == last
                // making sure maybe_fist and maybe_last are the only 2 cells written to 
                && all_stateful_writes.count() == 2
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
}
