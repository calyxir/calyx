use crate::analysis::ReadWriteSet;
use crate::ir::RRC;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
};
use itertools::Itertools;
use std::rc::Rc;

/// Transform groups that are structurally invoking components into equivalent
/// [ir::Invoke] statements.
///
/// For a group to meet the requirements of this pass, it must
/// 1. Only write to one non-combinational component
/// 2. That component is never read from and written to in the same assignemnt
/// 3. Assign component.go = 1'd1
/// 4. Assign group[done] = component.done
#[derive(Default)]
pub struct GroupToInvoke;

impl Named for GroupToInvoke {
    fn name() -> &'static str {
        "group2invoke"
    }

    fn description() -> &'static str {
        "covert groups that structurally invoke one component into invoke statements"
    }
}

// Returns true if port's parent is cell. We can do this by checking if they
// have the same name.
fn cell_is_parent(port: &ir::Port, cell: &ir::RRC<ir::Cell>) -> bool {
    port.get_parent_name() == cell.borrow().name()
}

/// Construct an [ir::Invoke] from an [ir::Group] that has been validated by this pass.
fn construct_invoke(
    assigns: &[ir::Assignment],
    comp: RRC<ir::Cell>,
    builder: &mut ir::Builder,
) -> ir::Control {
    let mut inputs = Vec::new();
    let mut comb_assigns = Vec::new();

    // Check if port's parent is a combinational primitive
    let comb_is_parent = |port: &ir::Port| -> bool {
        if let ir::PortParent::Cell(cell_wref) = &port.parent {
            match cell_wref.upgrade().borrow().prototype {
                ir::CellType::Primitive { is_comb, .. } => is_comb,
                _ => false,
            }
        } else {
            false
        }
    };

    for assign in assigns {
        // We know that all assignments in this group should write to either a)
        // a combinational component or b) comp or c) the group's done port

        // If a combinational component's port is being used as a dest, add
        // it to comb_assigns
        if comb_is_parent(&assign.dst.borrow()) {
            comb_assigns.push(assign.clone());
        }
        // If the cell's port is being used as a dest, add the source to
        // inputs. we can ignore the cell.go assignment, since that is not
        // part of the `invoke`.
        else if cell_is_parent(&assign.dst.borrow(), &comp)
            && assign.dst != comp.borrow().get_with_attr("go")
        {
            let name = assign.dst.borrow().name.clone();
            if assign.guard.is_true() {
                inputs.push((name, Rc::clone(&assign.src)));
            } else {
                // comp has a guarded assignment, need a wire
                let width = assign.dst.borrow().width;
                let wire =
                    builder.add_primitive("std_wire", "std_wire", &[width]);
                let wire_in_rrc = wire.borrow().get("in");
                let asmt = builder.build_assignment(
                    wire_in_rrc,
                    Rc::clone(&assign.src),
                    *assign.guard.clone(),
                );
                comb_assigns.push(asmt);
                let wire_out_rrc = wire.borrow().get("out");
                inputs.push((name, wire_out_rrc));
            }
        }
    }

    let comb_group = if comb_assigns.is_empty() {
        None
    } else {
        let comb_group_ref = builder.add_comb_group("comb_invoke");
        comb_group_ref
            .borrow_mut()
            .assignments
            .append(&mut comb_assigns);
        Some(comb_group_ref)
    };

    ir::Control::Invoke(ir::Invoke {
        comp,
        inputs,
        outputs: Vec::new(),
        comb_group,
        attributes: ir::Attributes::default(),
        ref_cells: Vec::new(),
    })
}

impl Visitor for GroupToInvoke {
    fn enable(
        &mut self,
        s: &mut ir::Enable,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, sigs);

        let group = s.group.borrow();

        // There should be exactly one non-combinational component being written to in the
        // group.
        let mut writes: Vec<ir::RRC<ir::Cell>> =
            ReadWriteSet::write_set(group.assignments.iter())
                .filter(|cell| match cell.borrow().prototype {
                    ir::CellType::Primitive { is_comb, .. } => !is_comb,
                    _ => true,
                })
                .collect_vec();
        if writes.len() != 1 {
            return Ok(Action::Continue);
        }

        // If component is ThisComponent, Reference, or External, don't turn into invoke
        let cell = writes.pop().unwrap();
        if matches!(cell.borrow().prototype, ir::CellType::ThisComponent)
            || cell.borrow().is_reference()
            || matches!(cell.borrow().get_attribute("external"), Some(_))
        {
            return Ok(Action::Continue);
        }

        // Component must define a @go/@done interface
        let maybe_go_port = cell.borrow().find_with_attr("go");
        let maybe_done_port = cell.borrow().find_with_attr("done");
        if maybe_go_port.is_none() || maybe_done_port.is_none() {
            return Ok(Action::Continue);
        }

        let go_port = maybe_go_port.unwrap();
        let mut go_multi_write = false;
        let done_port = maybe_done_port.unwrap();
        let mut done_multi_write = false;
        for assign in &group.assignments {
            // If reading and writing to cell in same assignment, then don't transform
            if cell_is_parent(&assign.dst.borrow(), &cell)
                && ReadWriteSet::port_reads(assign)
                    .any(|port| cell_is_parent(&port.borrow(), &cell))
            {
                return Ok(Action::Continue);
            }

            // @go port should have exactly one write and the src should be 1.
            if assign.dst == go_port {
                if go_multi_write {
                    return Ok(Action::Continue);
                }
                if !go_multi_write && assign.src.borrow().is_constant(1, 1) {
                    //guard must be true
                    if assign.guard.is_true() {
                        go_multi_write = true;
                    } else {
                        //if go port's guard is not true, then continue
                        return Ok(Action::Continue);
                    }
                }
            }
            // @done port should have exactly one read and the dst should be
            // group's done signal.
            if assign.src == done_port {
                if done_multi_write {
                    return Ok(Action::Continue);
                }
                if !done_multi_write && assign.dst == group.get("done") {
                    //Guard must be true
                    if assign.guard.is_true() {
                        done_multi_write = true;
                    } else {
                        //If done port's guard is not true, then Continue
                        return Ok(Action::Continue);
                    }
                }
            }
        }
        // Making sure we saw at least one read of the done port
        if !done_multi_write {
            return Ok(Action::Continue);
        }

        Ok(Action::change(construct_invoke(
            &group.assignments,
            cell,
            &mut builder,
        )))
    }
}
