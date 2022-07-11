use std::rc::Rc;

use itertools::Itertools;

use crate::analysis::ReadWriteSet;
use crate::ir::RRC;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
};

/// Transform groups that are structurally invoking components into equivalent
/// [ir::Invoke] statements.
///
/// For a group to meet the requirements of this pass, it must
/// 1. Only use unguarded assignments
/// 2. Only assign to input ports of one non-combinational component
/// 3. Assign `1'd1` to the @go port of the component, and
/// 4. Depend directly on the @done port of the component for its done
///    condition.
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

/// Construct an [ir::Invoke] from an [ir::Group] that has been validated by this pass.
fn construct_invoke(
    assigns: &[ir::Assignment],
    comp: RRC<ir::Cell>,
    builder: &mut ir::Builder,
) -> ir::Control {
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut comb_assigns = Vec::new();

    let cell_is_parent = |port: &ir::Port| -> bool {
        if let ir::PortParent::Cell(cell_wref) = &port.parent {
            Rc::ptr_eq(&cell_wref.upgrade(), &comp)
        } else {
            false
        }
    };

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
        // If the cell's port is being used as a source, add the dst to
        // outputs
        if cell_is_parent(&assign.src.borrow())
            && assign.src != comp.borrow().get_with_attr("done")
        {
            let name = assign.src.borrow().name.clone();
            outputs.push((name, Rc::clone(&assign.dst)));
        }
        // If the cell's port is being used as a dest, add the source to
        // inputs
        if cell_is_parent(&assign.dst.borrow())
            && assign.dst != comp.borrow().get_with_attr("go")
        {
            let name = assign.dst.borrow().name.clone();
            inputs.push((name, Rc::clone(&assign.src)));
        }
        // If a combinational component's port is being used as a dest, add
        // it to comb_assigns
        else if comb_is_parent(&assign.dst.borrow()) {
            let asmt = assign.clone();
            comb_assigns.push(asmt);
        }
    }

    let comb_group = if comb_assigns.is_empty() {
        None
    } else {
        let comb_group_ref = builder.add_comb_group("comb_invoke_");
        comb_group_ref
            .borrow_mut()
            .assignments
            .append(&mut comb_assigns);
        Some(comb_group_ref)
    };

    ir::Control::Invoke(ir::Invoke {
        comp: comp,
        inputs: inputs,
        outputs: outputs,
        comb_group: comb_group,
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

        // Component must define a @go/@done interface
        let cell = writes.pop().unwrap();
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
            // All assignments should be unguarded.
            if !assign.guard.is_true() {
                return Ok(Action::Continue);
            }

            // @go port should have exactly one write and the src should be 1.
            if assign.dst == go_port {
                if go_multi_write {
                    return Ok(Action::Continue);
                }
                if !go_multi_write && assign.src.borrow().is_constant(1, 1) {
                    go_multi_write = true;
                }
            }
            // @done port should have exactly one read and the dst should be
            // group's done signal.
            if assign.src == done_port {
                if done_multi_write {
                    return Ok(Action::Continue);
                }
                if !done_multi_write && assign.dst == group.get("done") {
                    done_multi_write = true;
                }
            }
        }

        Ok(Action::change(construct_invoke(
            &group.assignments,
            cell,
            &mut builder,
        )))
    }
}
