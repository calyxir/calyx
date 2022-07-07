use crate::analysis::ReadWriteSet;
use crate::ir;
use std::collections::HashMap;

#[derive(Default)]
///Primarily used to help determine the order cells are executed within
///the group
pub struct OrderAnalysis {
    ///Map w/ entries (b,a) for all assignments of form a.go = b.done,
    go_done_map: HashMap<ir::Id, ir::Id>,
    ///For group[done] = a.done or group[done] = a.done ? 1'd1,
    last: Option<ir::Id>,
}

impl OrderAnalysis {
    //Returns true if the cell is a component or a non-combinational primitive
    fn is_stateful(&self, cell: &ir::RRC<ir::Cell>) -> bool {
        match &cell.borrow().prototype {
            ir::CellType::Primitive { is_comb, .. } => !*is_comb,
            ir::CellType::Component { .. } => true,
            _ => false,
        }
    }
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
    //For a given asmt, if asmt is a.go = b.done, then we add (b,a) to self.go_done_map.
    //If we find that b is already a key in self.go_done_map, we return false to signal
    //that the same done signal is triggering two different go's.
    //Also if asmt is group[done] = cell.done, sets self.last to Some(cell).
    fn update(&mut self, asmt: &ir::Assignment) -> bool {
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
                    match self
                        .go_done_map
                        .get(src_cell.upgrade().borrow().name())
                    {
                        None => {
                            self.go_done_map.insert(
                                src_cell.upgrade().borrow().name().clone(),
                                dst_cell.upgrade().borrow().name().clone(),
                            );
                        }
                        Some(name) => {
                            if name != dst_cell.upgrade().borrow().name() {
                                return false;
                            }
                        }
                    }
                }
            }
            // src_cell's done writes to group's done
            (ir::PortParent::Cell(src_cell), ir::PortParent::Group(_)) => {
                if dst.name == "done" {
                    //checking for a.done
                    if matches!(*asmt.guard, ir::Guard::True)
                        && src.name == "done"
                    {
                        self.last =
                            Some(src_cell.upgrade().borrow().name().clone())
                    } else {
                        //checking for a.done ? 1'd1
                        if src.is_constant(1, 1) {
                            if let ir::Guard::Port(port) = &*asmt.guard {
                                if port.borrow().name == "done" {
                                    self.last = Some(get_parent_name(port))
                                }
                            }
                        }
                    }
                }
            }
            // If we encounter anything else, then not of interest to us
            _ => (),
        }
        true
    }
    //Given the name of the cell, returns the name of the predecessor cell based on go_done_map
    //If it has no predecessor according to go_done_map, return None.
    fn get_pred(&self, name: &ir::Id) -> Option<ir::Id> {
        if let Some((go, _)) =
            self.go_done_map.iter().find(|(_, done)| *done == name)
        {
            Some(go.clone())
        } else {
            None
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
    ) -> Option<Vec<ir::Id>> {
        //Update self.go_done_map and self.last for each asmt in the group.
        //The only time self.update() returns
        //false is when it discovers that group has one cell's done port triggering
        //multiple different cell's go ports.
        for asmt in asmts {
            if !self.update(asmt) {
                return None;
            }
        }
        //Build ordering of cells, based on self.go_done_map and self.last.
        let mut ordering: Vec<ir::Id> = Vec::new();
        if let Some(last_cell) = self.last.clone() {
            ordering.push(last_cell.clone());
            let mut cur_pred = last_cell;
            while let Some(new_pred) = self.get_pred(&cur_pred) {
                ordering.insert(0, new_pred.clone());
                cur_pred = new_pred.clone();
            }
            let all_stateful_cells = ReadWriteSet::write_set(asmts.iter())
                .filter(|cell| self.is_stateful(cell));
            if ordering.len() == all_stateful_cells.count() {
                Some(ordering)
            } else {
                None
            }
        } else {
            None
        }
    }
}

//Gets the name of the port parent
fn get_parent_name(port: &ir::RRC<ir::Port>) -> ir::Id {
    match &port.borrow().parent {
        ir::PortParent::Cell(cell) => cell.upgrade().borrow().name().clone(),
        ir::PortParent::Group(group) => group.upgrade().borrow().name().clone(),
    }
}
