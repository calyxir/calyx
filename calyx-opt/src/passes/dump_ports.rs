use crate::passes::compile_ref::RefPortMap;
use calyx_ir::{self as ir, RRC, WRC};
use itertools::Itertools;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

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
    removed: &mut HashMap<ir::Canonical, RRC<ir::Port>>,
) -> Vec<RRC<ir::Cell>> {
    let comp_name = component.name;
    let (ext_cells, cells): (Vec<_>, Vec<_>) =
        component.cells.drain().partition(cell_filter);
    component.cells.append(cells.into_iter());

    for cell_ref in &ext_cells {
        let cell = cell_ref.borrow();
        log::debug!("`{}' is matches predictate", cell.name());

        // If we do not eliminate the @clk and @reset ports, we may
        // get signals conflicting the original @clk and @reset signals of
        // the component, see https://github.com/cucapra/calyx/issues/1034
        let ports_inline = cell
            .ports
            .iter()
            .filter(|pr| {
                let p = pr.borrow();
                if remove_signals {
                    !p.attributes.has(ir::BoolAttr::Clk)
                        && !p.attributes.has(ir::BoolAttr::Reset)
                } else {
                    true
                }
            })
            .map(Rc::clone)
            .collect_vec();

        for port_ref in ports_inline {
            let canon = port_ref.borrow().canonical();
            let port = port_ref.borrow();
            let new_port = Rc::new(RefCell::new(ir::Port {
                name: component.generate_name(format_port_name(&canon)),
                width: port.width,
                direction: port.direction.clone(),
                parent: ir::PortParent::Cell(WRC::from(&component.signature)),
                attributes: ir::Attributes::default(),
            }));
            component
                .signature
                .borrow_mut()
                .ports
                .push(Rc::clone(&new_port));

            // Record the port as removed
            removed.insert(canon.clone(), Rc::clone(&new_port));

            // Record the port to add to cells
            port_names
                .entry(comp_name)
                .or_default()
                .insert(canon, Rc::clone(&new_port));
        }
    }
    ext_cells
}
