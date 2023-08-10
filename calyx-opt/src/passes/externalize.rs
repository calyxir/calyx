use super::dump_ports;
use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{self as ir, LibrarySignatures, RRC};
use calyx_utils::CalyxResult;
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
pub struct Externalize;

impl ConstructVisitor for Externalize {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized,
    {
        let externalize = Externalize;
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
    cell.get_attribute(ir::BoolAttr::External).is_some()
}

impl Visitor for Externalize {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut port_names = HashMap::new();
        let mut renamed = HashMap::new();
        let cells = dump_ports::dump_ports_to_signature(
            comp,
            has_external_attribute,
            false,
            &mut port_names,
            &mut renamed,
        );

        let cell_map = HashMap::default();
        let rw = ir::Rewriter::new(&cell_map, &renamed);
        comp.for_each_assignment(|assign| {
            rw.rewrite_assign(assign);
        });
        comp.for_each_static_assignment(|assign| {
            rw.rewrite_assign(assign);
        });
        // Don't allow cells to be dropped before this because otherwise rewriting will fail
        drop(cells);

        Ok(Action::Stop)
    }
}
