use calyx_ir::{self as ir, RRC, WRC};
use ir::rewriter;
use itertools::Itertools;
use std::rc::Rc;

#[derive(Default)]
/// Results generated from the process of dumping out ports.
pub struct DumpResults {
    /// The cells that were removed from the component.
    pub cells: Vec<RRC<ir::Cell>>,
    /// Rewrites from (cell, port) to the new port.
    /// Usually consumed by an [`ir::rewriter::Rewriter`].
    pub rewrites: rewriter::PortRewriteMap,
}

/// Formats name of a port given the id of the cell and the port
pub(super) fn format_port_name(canon: &ir::Canonical) -> ir::Id {
    format!("{}_{}", canon.cell, canon.port).into()
}

/// Remove all the cells matching the given criterion (f evaluates to true) from
/// the component and inline all the ports of the removed cells to the component
/// signature.
///
/// If `remove_clk_and_reset` is true, does not inline ports marked with @clk and @reset.
pub(super) fn dump_ports_to_signature<F>(
    component: &mut ir::Component,
    cell_filter: F,
    remove_clk_and_reset: bool,
) -> DumpResults
where
    F: Fn(&RRC<ir::Cell>) -> bool,
{
    let mut removed = rewriter::PortRewriteMap::default();

    let (ext_cells, cells): (Vec<_>, Vec<_>) =
        component.cells.drain().partition(cell_filter);
    component.cells.append(cells.into_iter());

    for cell_ref in &ext_cells {
        let cell = cell_ref.borrow();
        log::debug!("cell `{}' removed", cell.name());
        // We need this information because we might want to attach the `@data`
        // attribute to some of the ports.
        let is_data_cell = cell.attributes.has(ir::BoolAttr::Data);

        // If we do not eliminate the @clk and @reset ports, we may
        // get signals conflicting the original @clk and @reset signals of
        // the component, see https://github.com/calyxir/calyx/issues/1034
        let ports_inline = cell
            .ports
            .iter()
            .filter(|pr| {
                if remove_clk_and_reset {
                    let p = pr.borrow();
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
            // We might want to insert the @data attribute for optimization purposes.
            // But to do this, we have to make sure that the cell is marked @data
            // as well.
            let new_port_attrs =
                if is_data_cell & port.attributes.has(ir::BoolAttr::Data) {
                    let mut attrs = ir::Attributes::default();
                    attrs.insert(ir::BoolAttr::Data, 1);
                    attrs
                } else {
                    ir::Attributes::default()
                };

            let new_port = ir::rrc(ir::Port {
                name: component.generate_name(format_port_name(&canon)),
                width: port.width,
                direction: port.direction.clone(),
                parent: ir::PortParent::Cell(WRC::from(&component.signature)),
                attributes: new_port_attrs,
            });
            component
                .signature
                .borrow_mut()
                .ports
                .push(Rc::clone(&new_port));

            // Record the port as removed
            removed.insert(canon.clone(), Rc::clone(&new_port));
        }
    }
    DumpResults {
        cells: ext_cells,
        rewrites: removed,
    }
}
