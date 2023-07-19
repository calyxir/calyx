use calyx_ir as ir;
use calyx_utils::CalyxResult;
use calyx_utils::Error;
use itertools::Itertools;
// given `cell_ref` returns the `go` port of the cell (if it only has one `go` port),
// or an error otherwise
pub(super) fn get_go_port(cell_ref: ir::RRC<ir::Cell>) -> CalyxResult<ir::RRC<ir::Port>> {
  let cell = cell_ref.borrow();

  let name = cell.name();

  // Get the go port
  let mut go_ports = cell.find_all_with_attr(ir::NumAttr::Go).collect_vec();
  if go_ports.len() > 1 {
      return Err(Error::malformed_control(format!("Invoked component `{name}` defines multiple @go signals. Cannot compile the invoke")));
  } else if go_ports.is_empty() {
      return Err(Error::malformed_control(format!("Invoked component `{name}` does not define a @go signal. Cannot compile the invoke")));
  }

  Ok(go_ports.pop().unwrap())
}

// given inputs and outputs (of the invoke), and the `enable_assignments` (e.g., invoked_component.go = 1'd1)
// and a cell, builds the assignments for the corresponding group
fn build_assignments<T>(
  inputs: &mut Vec<(ir::Id, ir::RRC<ir::Port>)>,
  outputs: &mut Vec<(ir::Id, ir::RRC<ir::Port>)>,
  mut enable_assignments: Vec<ir::Assignment<T>>,
  builder: &mut ir::Builder,
  cell: &ir::Cell,
) -> Vec<ir::Assignment<T>> {
  inputs
      .drain(..)
      .map(|(inp, p)| {
          builder.build_assignment(cell.get(inp), p, ir::Guard::True)
      })
      .chain(outputs.drain(..).map(|(out, p)| {
          builder.build_assignment(p, cell.get(out), ir::Guard::True)
      }))
      .chain(enable_assignments.drain(..))
      .collect()
}

