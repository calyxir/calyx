use std::collections::HashMap;

use crate::ir::{self, RRC, WRC};

/// Formats name of a port given the id of the cell and the port
pub(super) fn format_port_name(comp: &ir::Id, port: &ir::Id) -> ir::Id {
    format!("{}_{}", comp.id, port.id).into()
}

/// Remove all the cells matching the given criterion(f evaluates to true) from
/// the component and inline all the ports of the removed cells to the component
/// signature
pub(super) fn dump_ports_to_signature(
    component: &mut ir::Component,
    f: fn(&RRC<ir::Cell>) -> bool,
    port_names: &mut HashMap<ir::Id, HashMap<ir::Id, HashMap<ir::Id, ir::Id>>>,
) {
    let comp_name = component.name.clone();
    let (ext_cells, cells): (Vec<_>, Vec<_>) =
        component.cells.drain().partition(f);

    component.cells.append(cells.into_iter());

    for cell_ref in ext_cells {
        let mut cell = cell_ref.borrow_mut();
        let name = cell.name().clone();
        let (ports_inline, _): (Vec<_>, Vec<_>) =
            cell.ports.drain(..).partition(|p_ref| {
                let p = p_ref.borrow();
                p.attributes.get("clk").is_none()
                    && p.attributes.get("reset").is_none()
            });
        // If we do not eliminate the @clk and @reset ports, we may
        // get signals conflicting the original @clk and @reset signals of
        // the component, see https://github.com/cucapra/calyx/issues/1034
        for port_ref in ports_inline.into_iter() {
            let port_name = port_ref.borrow().name.clone();
            // Change the name and the parent of this port.
            port_names
                .entry(comp_name.clone())
                .or_default()
                .entry(name.clone())
                .or_default()
                .insert(
                    port_name.clone(),
                    component
                        .generate_name(format_port_name(&name, &port_name)),
                );
            port_ref.borrow_mut().name =
                port_names[&comp_name][&name][&port_name].clone();
            // Point to the signature cell as its parent
            port_ref.borrow_mut().parent =
                ir::PortParent::Cell(WRC::from(&component.signature));
            // Remove any attributes from this cell port.
            port_ref.borrow_mut().attributes = ir::Attributes::default();
            component.signature.borrow_mut().ports.push(port_ref);
        }
    }
}
