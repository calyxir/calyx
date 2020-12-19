use crate::frontend::library::ast as lib;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, WRC};

#[derive(Default)]
/// Externalize input/output ports for "external" cells.
/// The ports of these cells are exposed through the ports of the parent
/// component.
///
/// For example:
/// ```
/// component main() -> () {
///     cells {
///         // Inputs: addr0, write_data, write_en
///         // Outputs: read_data, done
///         m1 = prim std_mem_d1_ext(32, 10, 4);
///     }
///     wires {
///         m1.addr0 = 1'd1;
///         x.in = m1.read_data;
///     }
/// }
/// ```
/// is transformed into:
/// ```
/// component main(
///     m1_read_data: 32,
///     m1_done: 1
/// ) -> (m1_add0: 4, m1_write_data: 32, m1_write_en: 1) {
///     cells {
///         // m1 removed.
///     }
///     wires {
///         m1_add0 = 1'd1;
///         x.in = m1_read_data;
///     }
/// }
/// ```
pub struct Externalize;

impl Named for Externalize {
    fn name() -> &'static str {
        "externalize"
    }

    fn description() -> &'static str {
        "Externalize the interfaces of _ext memories"
    }
}

/// Is this primitive and external
fn is_external_cell(name: &ir::Id) -> bool {
    name.as_ref().starts_with("std_mem") && name.as_ref().ends_with("ext")
}

/// Generate a string given the name of the component and the port.
fn format_port_name(comp: &ir::Id, port: &ir::Id) -> ir::Id {
    format!("{}_{}", comp.id, port.id).into()
}

impl Visitor for Externalize {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _ctx: &lib::LibrarySignatures,
    ) -> VisResult {
        // Extract external cells.
        let (ext_cells, cells): (Vec<_>, Vec<_>) =
            comp.cells.drain(..).into_iter().partition(|cr| {
                let cell = cr.borrow();
                if let ir::CellType::Primitive { name, .. } = &cell.prototype {
                    return is_external_cell(name);
                }
                false
            });

        // Re-add non-external cells.
        comp.cells = cells;

        // Detach the port from the component's cell and attach it to the
        // component's signature.
        // By doing this, we don't need to change the assignments since they
        // refer to this port. All we have done is change the port's parent
        // which automatically changes the assignments.
        for cell_ref in ext_cells {
            let mut cell = cell_ref.borrow_mut();
            let name = cell.name.clone();
            for port_ref in cell.ports.drain(..) {
                let port_name = port_ref.borrow().name.clone();
                // Change the name and the parent of this port.
                port_ref.borrow_mut().name =
                    format_port_name(&name, &port_name);
                // Flip the direction of the port.
                let new_dir = port_ref.borrow().direction.reverse();
                port_ref.borrow_mut().direction = new_dir;
                // Point to the signature cell as its parent
                port_ref.borrow_mut().parent =
                    ir::PortParent::Cell(WRC::from(&comp.signature));
                comp.signature.borrow_mut().ports.push(port_ref);
            }
        }

        // Stop traversal, we don't need to traverse over control ast
        Ok(Action::Stop)
    }
}
