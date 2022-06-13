use super::dump_ports;
use crate::errors::CalyxResult;
use crate::ir::traversal::ConstructVisitor;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, LibrarySignatures, RRC};
use std::collections::HashMap;

/// Externalize input/output ports for cells marked with the `@external(1)` attribute.
/// The ports of these cells are exposed through the ports of the parent
/// component.
///
/// For example:
/// ```
/// component main() -> () {
///     cells {
///         // Inputs: addr0, write_data, write_en
///         // Outputs: read_data, done
///         @external(1) m1 = prim std_mem_d1(32, 10, 4);
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
pub struct Externalize {
    port_names: HashMap<ir::Id, HashMap<ir::Id, HashMap<ir::Id, ir::Id>>>,
}

impl ConstructVisitor for Externalize {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized,
    {
        let externalize = Externalize {
            port_names: HashMap::new(),
        };
        Ok(externalize)
    }

    fn clear_data(&mut self) {
        //data is shared between components
    }
}

impl Named for Externalize {
    fn name() -> &'static str {
        "externalize"
    }

    fn description() -> &'static str {
        "Externalize the interfaces of cells marked with `@external(1)`"
    }
}

fn has_external_attribute(cr: &RRC<ir::Cell>) -> bool {
    let cell = cr.borrow();
    cell.get_attribute("external").is_some()
}

impl Visitor for Externalize {
    fn require_postorder() -> bool {
        true
    }

    fn start(
        &mut self,
        comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        dump_ports::dump_ports_to_signature(
            comp,
            has_external_attribute,
            &mut self.port_names,
        );

        Ok(Action::Continue)
    }
}
