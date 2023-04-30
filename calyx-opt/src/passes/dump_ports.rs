use crate::passes::compile_ref::RefPortMap;
use calyx_ir::{self as ir, RRC, WRC};
use itertools::Itertools;
use std::rc::Rc;

/// Formats name of a port given the id of the cell and the port
pub(super) fn format_port_name(canon: &ir::Canonical) -> ir::Id {
    format!("{}_{}", canon.0, canon.1).into()
}

/// Remove all the cells matching the given criterion (f evaluates to true) from
/// the component and inline all the ports of the removed cells to the component
/// signature.
///
/// If remove_signals is true, does not inline ports marked with @clk and @reset.
pub(super) fn dump_ports_to_signature(
    component: &mut ir::Component,
    cell_filter: fn(&RRC<ir::Cell>) -> bool,
    remove_signals: bool,
    port_names: &mut RefPortMap,
) {
    let comp_name = component.name;
    let (ext_cells, cells): (Vec<_>, Vec<_>) =
        component.cells.drain().partition(cell_filter);
    component.cells.append(cells.into_iter());

    for cell_ref in ext_cells {
        let mut cell = cell_ref.borrow_mut();

        // If we do not eliminate the @clk and @reset ports, we may
        // get signals conflicting the original @clk and @reset signals of
        // the component, see https://github.com/cucapra/calyx/issues/1034
        let ports_inline = cell
            .ports
            .drain(..)
            .filter(|pr| {
                let p = pr.borrow();
                if remove_signals {
                    p.attributes.get(ir::Attribute::Clk).is_none()
                        && p.attributes.get(ir::Attribute::Reset).is_none()
                } else {
                    true
                }
            })
            .collect_vec();
        // Explicitly drop `cell` otherwise call to `canonical` will panic
        drop(cell);

        for port_ref in ports_inline {
            let canon = port_ref.borrow().canonical();
            let port = &mut port_ref.borrow_mut();
            // Change the name and the parent of this port.
            port.name = component.generate_name(format_port_name(&canon));
            // Point to the signature cell as its parent
            port.parent = ir::PortParent::Cell(WRC::from(&component.signature));
            // Remove any attributes from this cell port.
            port.attributes = ir::Attributes::default();
            component
                .signature
                .borrow_mut()
                .ports
                .push(Rc::clone(&port_ref));
            // Record the port to add to cells
            port_names
                .entry(comp_name)
                .or_default()
                .insert(canon, Rc::clone(&port_ref));
        }
    }
}
