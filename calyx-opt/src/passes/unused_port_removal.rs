use crate::traversal::{
    Action, ConstructVisitor, Named, Order, VisResult, Visitor,
};
use calyx_ir::{
    Assignment, CellType, Component, Context, Id, LibrarySignatures, Nothing,
    Port, PortParent, RRC,
};
use calyx_utils::CalyxResult;

use std::collections::{HashMap, HashSet};

// Infers @internal annotations for component ports

pub struct UnusedPortRemoval {
    unused_ports: HashMap<Id, HashSet<Id>>,
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
            unused_ports: HashMap::new(),
            used_ports: HashMap::new(),
        };

        Ok(u_p_r)
    }

    // what is this for?
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
        // by the time we get to analyze the current component, all the ports of this
        // component that have been used are the only ones that are ever going to be used,
        // so we can compare against the complete set of ports defined in the component
        // signature to figure out which ones are not used by any external component <-- verify claim
        // for sig_port in comp.signature.borrow().ports().iter() {
        //     match self.used_ports.get(&comp.name) {
        //         // if the component name has no bindings, then it's a primitive; disregard
        //         None => (),

        //         // if the component name is bound to a hashset, then determine whether
        //         // the port from the signature is inside the hashset or not
        //         Some(set) => {
        //             match set.get(&sig_port.borrow().name) {
        //                 // if the signature port is not in the hashset, then it is unused
        //                 None => {
        //                     self.unused_ports
        //                         .entry(comp.name)
        //                         .or_default()
        //                         .insert(sig_port.borrow().name);

        //                     // how do you get a mutable reference to the port from
        //                     // the signature so we can add an attribute to it?
        //                 }

        //                 // if the signature port is in the set, then it is used
        //                 Some(_) => (),
        //             }
        //         }
        //     }
        // }

        let all_ports: HashSet<Id> = comp
            .signature
            .borrow()
            .ports()
            .iter()
            .map(|port| port.borrow().name)
            .collect();

        let unused_ports: HashSet<Id> = match self.used_ports.get(&comp.name) {
            None => panic!("bruh"),
            Some(used_set) => {
                all_ports.difference(used_set).map(|item: &Id| *item)
            }
        }
        .collect();

        // remove assignments that assign to dead ports
        // comp.continuous_assignments
        //     .retain(|&elt| elt.iter_ports(|port| {}));

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
