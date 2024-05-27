use crate::traversal::{
    self, Action, ConstructVisitor, Named, VisResult, Visitor,
};
use calyx_ir::structure;
use calyx_ir::{self as ir, Attributes, LibrarySignatures};
use calyx_utils::{CalyxResult, Error};
use ir::{Assignment, RRC, WRC};
use itertools::Itertools;
use std::collections::HashMap;
use std::rc::Rc;

use super::dump_ports;

// given `cell_ref` returns the `go` port of the cell (if it only has one `go` port),
// or an error otherwise
fn get_go_port(cell_ref: ir::RRC<ir::Cell>) -> CalyxResult<ir::RRC<ir::Port>> {
    let cell = cell_ref.borrow();
    let name = cell.name();

    // Get the go port
    match cell.find_unique_with_attr(ir::NumAttr::Go) {
        Ok(Some(p)) => Ok(p),
        Ok(None) => Err(Error::malformed_control(format!(
            "Invoked component `{name}` does not define a @go signal. Cannot compile the invoke",
        ))),
        Err(_) => {
            Err(Error::malformed_control(format!(
                "Invoked component `{name}` defines multiple @go signals. Cannot compile the invoke",
            )))
        }
    }
}

// given inputs and outputs (of the invoke), and the `enable_assignments` (e.g., invoked_component.go = 1'd1)
// and a cell, builds the assignments for the corresponding group
fn build_assignments<T>(
    inputs: &mut Vec<(ir::Id, ir::RRC<ir::Port>)>,
    outputs: &mut Vec<(ir::Id, ir::RRC<ir::Port>)>,
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
        .collect()
}

#[derive(Default)]
/// Map for storing added ports for each ref cell
/// level of Hashmap represents:
/// HashMap<-component name-, Hashmap<(-ref cell name-,-port name-), port>>;
struct RefPortMap(HashMap<ir::Id, HashMap<ir::Canonical, RRC<ir::Port>>>);

impl RefPortMap {
    fn insert(
        &mut self,
        comp_name: ir::Id,
        ports: HashMap<ir::Canonical, RRC<ir::Port>>,
    ) {
        self.0.insert(comp_name, ports);
    }

    fn get(
        &self,
        comp_name: &ir::Id,
    ) -> Option<&HashMap<ir::Canonical, RRC<ir::Port>>> {
        self.0.get(comp_name)
    }

    /// Get all of the newly added ports associated with a component that had
    /// ref cells
    fn get_ports(&self, comp_name: &ir::Id) -> Option<Vec<RRC<ir::Port>>> {
        self.0.get(comp_name).map(|map| {
            map.values()
                .cloned()
                .sorted_by(|a, b| a.borrow().name.cmp(&b.borrow().name))
                .collect()
        })
    }
}

/// Compiles [`ir::Invoke`] statements into an [`ir::Enable`] that runs the
/// invoked component.
pub struct CompileInvoke {
    /// Mapping from component to the canonical port name of ref cell o
    port_names: RefPortMap,
    /// Mapping from the ports of cells that were removed to the new port on the
    /// component signature.
    removed: HashMap<ir::Canonical, ir::RRC<ir::Port>>,
    /// Ref cells in the component. We hold onto these so that our references don't get invalidated
    ref_cells: Vec<ir::RRC<ir::Cell>>,
}

impl ConstructVisitor for CompileInvoke {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized,
    {
        Ok(CompileInvoke {
            port_names: RefPortMap::default(),
            removed: HashMap::new(),
            ref_cells: Vec::new(),
        })
    }

    fn clear_data(&mut self) {
        self.removed.clear();
        self.ref_cells.clear()
    }
}

impl Named for CompileInvoke {
    fn name() -> &'static str {
        "compile-invoke"
    }

    fn description() -> &'static str {
        "Rewrites invoke statements to group enables"
    }
}

impl CompileInvoke {
    /// Given `ref_cells` of an invoke, returns `(inputs, outputs)` where
    /// inputs are the corresponding inputs to the `invoke` and
    /// outputs are the corresponding outputs to the `invoke`.
    ///
    /// Since this pass eliminates all ref cells in post order, we expect that
    /// invoked component already had all of its ref cells removed.
    fn ref_cells_to_ports<T>(
        &mut self,
        inv_cell: RRC<ir::Cell>,
        ref_cells: impl Iterator<Item = (ir::Id, ir::RRC<ir::Cell>)>,
    ) -> Vec<ir::Assignment<T>> {
        let inv_comp = inv_cell.borrow().type_name().unwrap();
        let mut assigns = Vec::new();
        for (ref_cell_name, cell) in ref_cells {
            log::debug!(
                "Removing ref cell `{}` with {} ports",
                ref_cell_name,
                cell.borrow().ports.len()
            );

            // Mapping from canonical names of the ports of the ref cell to the
            // new port defined on the signature of the component
            let Some(comp_ports) = self.port_names.get(&inv_comp) else {
                unreachable!("component `{}` invoked but not already visited by the pass", inv_comp)
            };

            // The type of the cell is the same as the ref cell so we can
            // iterate over its ports and generate bindings for the ref cell.
            for pr in &cell.borrow().ports {
                let port = pr.borrow();
                if port.has_attribute(ir::BoolAttr::Clk)
                    || port.has_attribute(ir::BoolAttr::Reset)
                {
                    continue;
                }

                let canon = ir::Canonical::new(ref_cell_name, port.name);
                let Some(comp_port) = comp_ports.get(&canon) else {
                    unreachable!("port `{}` not found in the signature of {}. Known ports are: {}",
                        canon,
                        inv_comp,
                        comp_ports.keys().map(|c| c.port.as_ref()).collect_vec().join(", ")
                    )
                };
                // Get the port on the new cell with the same name as ref_port
                let ref_port = inv_cell.borrow().get(comp_port.borrow().name);
                log::debug!("Port `{}` -> `{}`", canon, ref_port.borrow().name);

                let old_port = pr.borrow().canonical();
                // If the port has been removed already, get the new port from the component's signature
                let arg_port = if let Some(sig_pr) = self.removed.get(&old_port)
                {
                    log::debug!(
                        "Port `{}` has been removed. Using `{}`",
                        old_port,
                        sig_pr.borrow().name
                    );
                    Rc::clone(sig_pr)
                } else {
                    Rc::clone(pr)
                };

                match port.direction {
                    ir::Direction::Output => {
                        log::debug!(
                            "constructing: {} = {}",
                            ref_port.borrow().canonical(),
                            arg_port.borrow().canonical()
                        );
                        assigns.push(ir::Assignment::new(
                            ref_port.clone(),
                            arg_port,
                        ));
                    }
                    ir::Direction::Input => {
                        log::debug!(
                            "constructing: {} = {}",
                            arg_port.borrow().canonical(),
                            ref_port.borrow().canonical(),
                        );
                        assigns.push(ir::Assignment::new(
                            arg_port,
                            ref_port.clone(),
                        ));
                    }
                    _ => {
                        unreachable!("Cell should have inout ports");
                    }
                }
            }
        }
        assigns
    }
}

impl Visitor for CompileInvoke {
    fn iteration_order() -> crate::traversal::Order
    where
        Self: Sized,
    {
        traversal::Order::Post
    }

    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        log::debug!("Visiting `{}`", comp.name);
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
                        let new_port = ir::rrc(ir::Port {
                            name: p.borrow().name,
                            width: p.borrow().width,
                            direction: p.borrow().direction.reverse(),
                            parent: ir::PortParent::Cell(WRC::from(cell)),
                            attributes: Attributes::default(),
                        });
                        new_ports.push(new_port);
                    }
                }
            }
            cell.borrow_mut().ports.extend(new_ports);
        }

        let dump_ports::DumpResults { cells, rewrites } =
            dump_ports::dump_ports_to_signature(
                comp,
                |cell| cell.borrow().is_reference(),
                true,
            );

        // Hold onto the cells so they don't get dropped.
        self.ref_cells = cells;
        self.removed = rewrites;

        Ok(Action::Continue)
    }

    fn invoke(
        &mut self,
        s: &mut ir::Invoke,
        comp: &mut ir::Component,
        ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, ctx);
        let invoke_group = builder.add_group("invoke");
        // Assigns representing the ref cell connections
        invoke_group.borrow_mut().assignments.extend(
            self.ref_cells_to_ports(Rc::clone(&s.comp), s.ref_cells.drain(..)),
        );

        // comp.go = 1'd1;
        // invoke[done] = comp.done;
        structure!(builder;
            let one = constant(1, 1);
        );
        let cell = s.comp.borrow();
        let go_port = get_go_port(Rc::clone(&s.comp))?;
        let done_port = cell.find_unique_with_attr(ir::NumAttr::Done)?.unwrap();

        // Build assignemnts
        let go_assign = builder.build_assignment(
            go_port,
            one.borrow().get("out"),
            ir::Guard::True,
        );
        let done_assign = builder.build_assignment(
            invoke_group.borrow().get("done"),
            done_port,
            ir::Guard::True,
        );

        invoke_group
            .borrow_mut()
            .assignments
            .extend(vec![go_assign, done_assign]);

        // Generate argument assignments
        let cell = &*s.comp.borrow();
        let assigns = build_assignments(
            &mut s.inputs,
            &mut s.outputs,
            &mut builder,
            cell,
        );
        invoke_group.borrow_mut().assignments.extend(assigns);

        // Add assignments from the attached combinational group
        if let Some(cgr) = &s.comb_group {
            let cg = &*cgr.borrow();
            invoke_group
                .borrow_mut()
                .assignments
                .extend(cg.assignments.iter().cloned())
        }

        // Copy "promotable" annotation from the `invoke` statement if present
        if let Some(time) = s.attributes.get(ir::NumAttr::Promotable) {
            invoke_group
                .borrow_mut()
                .attributes
                .insert(ir::NumAttr::Promotable, time);
        }

        let mut en = ir::Enable {
            group: invoke_group,
            attributes: std::mem::take(&mut s.attributes),
        };
        if let Some(time) = s.attributes.get(ir::NumAttr::Promotable) {
            en.attributes.insert(ir::NumAttr::Promotable, time);
        }

        Ok(Action::change(ir::Control::Enable(en)))
    }

    fn static_invoke(
        &mut self,
        s: &mut ir::StaticInvoke,
        comp: &mut ir::Component,
        ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, ctx);

        let invoke_group = builder.add_static_group("static_invoke", s.latency);

        invoke_group.borrow_mut().assignments.extend(
            self.ref_cells_to_ports(Rc::clone(&s.comp), s.ref_cells.drain(..)),
        );

        // comp.go = 1'd1;
        structure!(builder;
            let one = constant(1, 1);
        );

        // Get the go port
        let go_port = get_go_port(Rc::clone(&s.comp))?;

        // Checks whether compe is a static<n> component or an @interval(n) component.
        let go_guard = if s
            .comp
            .borrow()
            .ports
            .iter()
            .any(|port| port.borrow().attributes.has(ir::NumAttr::Interval))
        {
            // For @interval(n) components, we do not guard the comp.go
            // We trigger the go signal for the entire interval.
            ir::Guard::True
        } else {
            // For static<n> components, we guard the comp.go with %[0:1]
            ir::Guard::Info(ir::StaticTiming::new((0, 1)))
        };

        // Build assignemnts
        let go_assign: ir::Assignment<ir::StaticTiming> = builder
            .build_assignment(go_port, one.borrow().get("out"), go_guard);
        invoke_group.borrow_mut().assignments.push(go_assign);

        // Generate argument assignments
        let cell = &*s.comp.borrow();
        let assigns = build_assignments(
            &mut s.inputs,
            &mut s.outputs,
            &mut builder,
            cell,
        );
        invoke_group.borrow_mut().assignments.extend(assigns);

        if let Some(cgr) = &s.comb_group {
            let cg = &*cgr.borrow();
            invoke_group.borrow_mut().assignments.extend(
                cg.assignments
                    .iter()
                    .cloned()
                    .map(Assignment::from)
                    .collect_vec(),
            );
        }

        let en = ir::StaticEnable {
            group: invoke_group,
            attributes: std::mem::take(&mut s.attributes),
        };

        Ok(Action::StaticChange(Box::new(ir::StaticControl::Enable(
            en,
        ))))
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let port_map = std::mem::take(&mut self.removed);
        // Add the newly added port to the global port map
        // Rewrite all of the ref cell ports
        let rw = ir::Rewriter {
            port_map,
            ..Default::default()
        };
        rw.rewrite(comp);
        self.port_names.insert(comp.name, rw.port_map);
        Ok(Action::Continue)
    }
}
