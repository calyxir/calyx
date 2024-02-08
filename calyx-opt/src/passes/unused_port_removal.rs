use crate::traversal::{
    Action, ConstructVisitor, Named, Order, VisResult, Visitor,
};
use calyx_ir::{
    Assignment, BoolAttr, CellType, Component, Context, Id, LibrarySignatures,
    Nothing, Port, PortParent, RRC,
};
use calyx_utils::CalyxResult;

use std::collections::{HashMap, HashSet};

// Infers @internal annotations for component ports

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

    // before any control logic is processed, add attribute @internal
    // to every unused port
    fn start(
        &mut self,
        comp: &mut Component,
        _sigs: &LibrarySignatures,
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
                match unused_ports.get(&name) {
                    None => (),
                    Some(_) => {
                        port_ref.attributes.insert(BoolAttr::Internal, 1);
                    }
                }
            }
        }

        //
        // comp.for_each_assignment(|assign: &mut Assignment<Nothing>| {});

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
