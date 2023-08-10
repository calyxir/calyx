use super::dump_ports;
use crate::traversal::{
    Action, ConstructVisitor, Named, Order, VisResult, Visitor,
};
use calyx_ir::{self as ir, Attributes, LibrarySignatures, RRC, WRC};
use calyx_utils::CalyxResult;
use itertools::Itertools;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

type PortMap = Vec<(ir::Id, ir::RRC<ir::Port>)>;

/// Map for storing added ports for each ref cell
/// level of Hashmap represents:
/// HashMap<-component name-, Hashmap<(-ref cell name-,-port name-), port>>;
pub(super) type RefPortMap =
    HashMap<ir::Id, HashMap<ir::Canonical, RRC<ir::Port>>>;

trait GetPorts {
    fn get_ports(&self, comp_name: &ir::Id) -> Option<Vec<RRC<ir::Port>>>;
}

impl GetPorts for RefPortMap {
    fn get_ports(&self, comp_name: &ir::Id) -> Option<Vec<RRC<ir::Port>>> {
        if self.contains_key(comp_name) {
            let mut ret = Vec::new();
            for (_, p) in self[comp_name].iter() {
                ret.push(Rc::clone(p));
            }
            Some(ret)
        } else {
            None
        }
    }
}

/// Pass to eliminate `ref` cells from the program.
/// 1. Remove all the cells marked with the 'ref' keyword
/// 2. Inline all the ports of the ref cells to the component signature
/// 3. Remove all the ref cell mappings from the invoke statement
/// 4. Inline all the mappings of ports to the invoke signature
pub struct CompileRef {
    port_names: RefPortMap,
    /// Mapping from the ports of cells that were removed to the new port on the
    /// component signature.
    removed: HashMap<ir::Canonical, ir::RRC<ir::Port>>,
}

impl ConstructVisitor for CompileRef {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized,
    {
        Ok(CompileRef {
            port_names: HashMap::new(),
            removed: HashMap::new(),
        })
    }

    fn clear_data(&mut self) {
        self.removed.clear()
    }
}

fn is_external_cell(cr: &RRC<ir::Cell>) -> bool {
    cr.borrow().is_reference()
}

impl Named for CompileRef {
    fn name() -> &'static str {
        "compile-ref"
    }

    fn description() -> &'static str {
        "Inline the ports of reference cells to component signature and the invoke signature"
    }
}

impl CompileRef {
    /// Given `ref_cells` of an invoke, returns `(inputs, outputs)` where
    /// inputs are the corresponding inputs to the `invoke` and
    /// outputs are the corresponding outputs to the `invoke`.
    ///
    /// Since this pass eliminates all ref cells in post order, we expect that
    /// invoked component already had all of its ref cells removed.
    fn ref_cells_to_ports(
        &mut self,
        inv_comp: ir::Id,
        ref_cells: Vec<(ir::Id, ir::RRC<ir::Cell>)>,
    ) -> (PortMap, PortMap) {
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        for (ref_cell_name, cell) in ref_cells {
            log::debug!(
                "Removing ref cell `{}` with {} ports",
                ref_cell_name,
                cell.borrow().ports.len()
            );
            let Some(comp_ports) = self.port_names.get(&inv_comp) else {
                unreachable!("component `{}` invoked but not already visited by the pass", inv_comp)
            };
            // The type of the cell is the same as the ref cell so we can
            // iterate over its ports and generate bindings for the ref cell.
            for pr in &cell.borrow().ports {
                let port = pr.borrow();
                if !port.attributes.has(ir::BoolAttr::Clk)
                    && !port.attributes.has(ir::BoolAttr::Reset)
                {
                    log::debug!("Adding port `{}`", port.name);
                    let canon = ir::Canonical(ref_cell_name, port.name);
                    let Some(ref_port) = comp_ports.get(&canon) else {
                        unreachable!("port `{}` not found. Known ports are: {}",
                            canon,
                            comp_ports.keys().map(|c| c.1.as_ref()).collect_vec().join(", ")
                        )
                    };
                    let port_name = ref_port.borrow().name;
                    let old_port = pr.borrow().canonical();
                    // If the port has been removed already, get the new port from the component's signature
                    let port_bind =
                        if let Some(sig_pr) = self.removed.get(&old_port) {
                            log::debug!(
                                "Port `{}` has been removed. Using `{}`",
                                old_port,
                                sig_pr.borrow().name
                            );
                            (port_name, Rc::clone(sig_pr))
                        } else {
                            (port_name, Rc::clone(pr))
                        };

                    match port.direction {
                        ir::Direction::Input => {
                            outputs.push(port_bind);
                        }
                        ir::Direction::Output => {
                            inputs.push(port_bind);
                        }
                        _ => {
                            unreachable!("Cell should have inout ports");
                        }
                    }
                }
            }
        }
        (inputs, outputs)
    }
}

impl Visitor for CompileRef {
    fn iteration_order() -> Order {
        Order::Post
    }

    fn start(
        &mut self,
        comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        log::debug!("compile-ref: {}", comp.name);
        dump_ports::dump_ports_to_signature(
            comp,
            is_external_cell,
            true,
            &mut self.port_names,
            &mut self.removed,
        );

        // For all subcomponents that had a `ref` cell in them, we need to
        // update their cell to have the new ports added from inlining the
        // signatures of all the ref cells.
        for cell in comp.cells.iter() {
            let mut new_ports: Vec<RRC<ir::Port>> = Vec::new();
            if let Some(name) = cell.borrow().type_name() {
                if let Some(vec) = self.port_names.get_ports(&name) {
                    log::debug!(
                        "Updating ports of cell `{}' (type `{name}')",
                        cell.borrow().name()
                    );
                    for p in vec.iter() {
                        let new_port = Rc::new(RefCell::new(ir::Port {
                            name: p.borrow().name,
                            width: p.borrow().width,
                            direction: p.borrow().direction.reverse(),
                            parent: ir::PortParent::Cell(WRC::from(cell)),
                            attributes: Attributes::default(),
                        }));
                        new_ports.push(new_port);
                    }
                }
            }
            cell.borrow_mut().ports.extend(new_ports);
        }
        Ok(Action::Continue)
    }

    fn invoke(
        &mut self,
        s: &mut ir::Invoke,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let comp_name = s.comp.borrow().type_name().unwrap();
        let ref_cells = std::mem::take(&mut s.ref_cells);
        let (mut inputs, mut outputs) =
            self.ref_cells_to_ports(comp_name, ref_cells);
        s.inputs.append(&mut inputs);
        s.outputs.append(&mut outputs);
        Ok(Action::Continue)
    }
    fn static_invoke(
        &mut self,
        s: &mut ir::StaticInvoke,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let comp_name = s.comp.borrow().type_name().unwrap();
        let ref_cells = std::mem::take(&mut s.ref_cells);
        let (mut inputs, mut outputs) =
            self.ref_cells_to_ports(comp_name, ref_cells);
        s.inputs.append(&mut inputs);
        s.outputs.append(&mut outputs);
        Ok(Action::Continue)
    }
}
