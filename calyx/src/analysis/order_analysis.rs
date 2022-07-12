use crate::analysis::ReadWriteSet;
use crate::ir;

#[derive(Default)]
///Primarily used to help determine the order cells are executed within
///the group
pub struct OrderAnalysis {
    ///Holds, (b,a) for assignment of form a.go = b.done,
    done_go: Option<(ir::Id, ir::Id)>,
    ///Holds a for assignment of form group[done] = a.done
    last: Option<ir::Id>,
}

impl OrderAnalysis {
    //Returns whether the given assignment is a go done assignment from two cells of interest
    //i.e. cell1.go = cell2.done.
    pub fn is_go_done(&self, asmt: &ir::Assignment) -> bool {
        let src = asmt.src.borrow();
        let dst = asmt.dst.borrow();
        match (&src.parent, &dst.parent) {
            (
                ir::PortParent::Cell(src_cell),
                ir::PortParent::Cell(dst_cell),
            ) => {
                //the first two checks may be unnecessary
                self.is_stateful(&src_cell.upgrade())
                    && self.is_stateful(&dst_cell.upgrade())
                    && src.name == "done"
                    && dst.attributes.has("go")
            }
            _ => false,
        }
    }
    //Returns whether the given assignment writes to the go assignment of cell
    //in the form cell.go = !cell.done? 1'd1.
    pub fn writes_to_go(&self, asmt: &ir::Assignment, cell: &ir::Id) -> bool {
        let guard_is_done = |guard: &ir::Guard| -> bool {
            match guard {
                ir::Guard::Port(port) => {
                    port.borrow().name == "done"
                        && match &port.borrow().parent {
                            ir::PortParent::Cell(cell_wref) => {
                                cell_wref.upgrade().borrow().name() == cell
                            }
                            _ => false,
                        }
                }
                _ => false,
            }
        };

        let guard_not_done = |guard: &ir::Guard| -> bool {
            match guard {
                ir::Guard::Not(g) => guard_is_done(&*g),
                _ => false,
            }
        };

        let dst = asmt.dst.borrow();
        match &dst.parent {
            ir::PortParent::Cell(dst_cell) => {
                //the first two checks may be unnecessary
                dst_cell.upgrade().borrow().name() == cell
                    && guard_not_done(&*asmt.guard)
                    && asmt.src.borrow().is_constant(1, 1)
                    && self.is_stateful(&dst_cell.upgrade())
                    && dst.attributes.has("go")
            }
            _ => false,
        }
    }
    //Returns true if the cell is a component or a non-combinational primitive
    fn is_stateful(&self, cell: &ir::RRC<ir::Cell>) -> bool {
        match &cell.borrow().prototype {
            ir::CellType::Primitive { is_comb, .. } => !*is_comb,
            ir::CellType::Component { .. } => true,
            _ => false,
        }
    }
    //For a given asmt, if asmt is a.go = b.done, then we add (b,a) to self.go_done_map.
    //If we find that b is already a key in self.go_done_map, we return false to signal
    //that the same done signal is triggering two different go's.
    //Also if asmt is group[done] = cell.done, sets self.last to Some(cell).
    fn update(&mut self, asmt: &ir::Assignment) {
        let src = asmt.src.borrow();
        let dst = asmt.dst.borrow();
        match (&src.parent, &dst.parent) {
            (
                ir::PortParent::Cell(src_cell),
                ir::PortParent::Cell(dst_cell),
            ) => {
                //first two checks may be unnecessary
                if self.is_stateful(&src_cell.upgrade())
                    && self.is_stateful(&dst_cell.upgrade())
                    && src.name == "done"
                    && dst.attributes.has("go")
                {
                    match self.done_go {
                        None => {
                            self.done_go = Some((
                                src_cell.upgrade().borrow().name().clone(),
                                dst_cell.upgrade().borrow().name().clone(),
                            ));
                        }
                        Some(_) => (),
                    }
                }
            }
            // src_cell's done writes to group's done
            (ir::PortParent::Cell(src_cell), ir::PortParent::Group(_)) => {
                if dst.name == "done" {
                    //checking for a.done
                    if src.name == "done" {
                        self.last =
                            Some(src_cell.upgrade().borrow().name().clone())
                    }
                }
            }
            // If we encounter anything else, then not of interest to us
            _ => (),
        }
    }

    //The assignment must write to a stateful component, *or* be a write
    //to the group's done port.
    //In order to perform the transformation to the group, all assignments in the group
    //must return true on this method.
    pub fn is_orderable_assignment(&self, asmt: &ir::Assignment) -> bool {
        match &asmt.dst.borrow().parent {
            ir::PortParent::Cell(cell) => self.is_stateful(&cell.upgrade()),
            ir::PortParent::Group(_) => asmt.dst.borrow().name == "done",
        }
    }

    //builds ordering for self. Returns true if this is a complete, valid, linear ordering, false otherwise
    pub fn get_ordering(
        &mut self,
        asmts: &Vec<ir::Assignment>,
    ) -> Option<(ir::Id, ir::Id)> {
        //Update self.go_done_map and self.last for each asmt in the group.
        for asmt in asmts {
            self.update(asmt);
        }
        //Build ordering of cells, based on self.done_go and self.last.
        if let (Some(last), Some((done, go))) =
            (self.last.clone(), self.done_go.clone())
        {
            let all_stateful_writes = ReadWriteSet::write_set(asmts.iter())
                .filter(|cell| self.is_stateful(cell));
            if go == last && all_stateful_writes.count() == 2 {
                Some((done, go))
            } else {
                None
            }
        } else {
            None
        }
    }
}
