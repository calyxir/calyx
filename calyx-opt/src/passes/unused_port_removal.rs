use crate::traversal::{
    Action, ConstructVisitor, Named, Order, VisResult, Visitor,
};
use calyx_ir::{
    Assignment, BoolAttr, Builder, Cell, CellType, Component, Context, Id,
    LibrarySignatures, Nothing, Port, PortParent, RRC,
};
use calyx_utils::CalyxResult;

use std::collections::{HashMap, HashSet};

pub struct UnusedPortRemoval {
    used_ports: HashMap<Id, HashSet<Id>>,
}

impl Named for UnusedPortRemoval {
    fn name() -> &'static str {
        "unused-port-removal"
    }

    fn description() -> &'static str {
        "Remove unused ports"
    }
}

impl ConstructVisitor for UnusedPortRemoval {
    fn from(_ctx: &Context) -> CalyxResult<Self>
    where
        Self: Sized,
    {
        // create and return an instance of an
        // UnusedPortRemoval pass over a single component (?)
        let u_p_r = UnusedPortRemoval {
            used_ports: HashMap::new(),
        };

        Ok(u_p_r)
    }

    // what is this for? clear data after visiting every component?
    fn clear_data(&mut self) {}
}

impl Visitor for UnusedPortRemoval {
    fn iteration_order() -> Order {
        Order::Pre
    }

    fn start(
        &mut self,
        comp: &mut Component,
        sigs: &LibrarySignatures,
        _comps: &[Component],
    ) -> VisResult {
        // By the time we get to analyze the current component, all the ports of this
        // component that have been used are the only ones that are ever going to be used,
        // so we can compare against the complete set of ports defined in the component
        // signature to figure out which ones are not used by any external component <-- verify claim

        // get a list of all ports instantiated in the component signature
        let all_ports: HashSet<Id> = comp
            .signature
            .borrow()
            .ports()
            .iter()
            .map(|port| port.borrow().name)
            .collect();

        // know these these signature-instantiated ports are a super set of ports
        // that are instantiated by other components, all of which we have access
        // to based on our pre-order traversal :)
        let unused_ports: HashSet<Id> = match self.used_ports.get(&comp.name) {
            None => all_ports,
            Some(used_set) => all_ports
                .difference(used_set)
                .map(|item: &Id| *item)
                .collect(),
        };

        // runt -i tests/passes/unused-port-removal/simple -d
        // if port from signature is an unused port, add an attribute @internal
        if comp.name != "main" {
            for port in comp.signature.borrow_mut().ports.iter_mut() {
                let mut port_ref = port.borrow_mut();
                let name = port_ref.name;
                match unused_ports.contains(&name) {
                    false => (),
                    true => {
                        port_ref.attributes.insert(BoolAttr::Internal, 1);
                    }
                }
            }

            // if either the source or a destination of an assignment are unused,
            // drop that assignment (meaning we don't care about those guards)
            comp.continuous_assignments.retain(|assign| {
                let source_not_used =
                    unused_ports.contains(&assign.src.borrow().name);
                let destination_not_used =
                    unused_ports.contains(&assign.dst.borrow().name);
                !(source_not_used || destination_not_used)
            });

            // initialize builder to construct constant cells

            // get widths of the unused ports within the guards of each assignment
            let mut port_widths: Vec<u64> = Vec::new();
            comp.for_each_assignment(|assign| {
                for port in (assign.guard).as_mut().all_ports().iter() {
                    let name = port.borrow().name;

                    match unused_ports.get(&name) {
                        None => (),
                        Some(..) => {
                            (&mut port_widths).push(port.borrow().width)
                        }
                    }
                }
            });

            // initialize map from port widths to cells
            let mut width_to_cell: HashMap<u64, RRC<Cell>> = HashMap::new();

            // from list of ports unused in guard, fill in hash mapping widths to Id's of
            // new instatiated constant cells in component
            let mut builder = Builder::new(comp, sigs);
            for port_width in port_widths.iter() {
                match width_to_cell.get(port_width) {
                    Some(..) => (),
                    None => {
                        let low_const_cell =
                            builder.add_constant(0, port_width.to_owned());
                        width_to_cell
                            .insert(port_width.to_owned(), low_const_cell);
                    }
                }
            }

            // now, we're simply left with assignments that assign to both used source
            // and destination ports; for assignments, it's possible that the guard of an assignment
            // uses an unused port; if this is the case, replace with n'b0 signal.
            comp.for_each_assignment(|assign| {
                // deference to get rid of Box pointer
                let guard = (assign.guard).as_mut();
                guard.collapse_unused(&mut width_to_cell, &unused_ports);
                // guard.collapse_unused_mut(comp, sigs, &unused_ports);
            })
        }

        // insert a mapping from each of this component's children components to
        // the ports that each child uses
        comp.iter_assignments(|assign: &Assignment<Nothing>| {
            assign.iter_ports(|port: &RRC<Port>| {
                match port.borrow().parent {
                    // only care about ports belonging to cells, not groups/static groups
                    PortParent::Cell(_) => {
                        match port.borrow().cell_parent().borrow().prototype {
                            // only care about non-primitives (i.e. components and not registers, etc.)
                            CellType::Component { name: comp_name } => {
                                self.used_ports
                                    .entry(comp_name)
                                    .or_default()
                                    .insert(port.borrow().name);
                            }
                            _ => (),
                        }
                    }
                    _ => (),
                }
            });
        });
        Ok(Action::Continue)
    }
}
