use calyx_ir::{self as ir, RRC};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

type PortMap = HashMap<ir::Id, Vec<RRC<ir::Port>>>;
type Binding = Vec<(ir::Id, RRC<ir::Port>)>;
type InvokeMap = HashMap<ir::Id, Vec<Binding>>;

/// Contains a mapping from name of [ir::CombGroup] to the ports read by the control program
/// as well as the mapping from invoke statements to the port mappings.
/// The vector of ports is guaranteed to only contain unique ports.
pub struct ControlPorts<const INVOKE_MAP: bool> {
    // Map name of combinational group to the ports read by the control program.
    cg_to_port: PortMap,
    // Mapping from name of invoke instance to the port bindings.
    invoke_map: InvokeMap,
}

impl<const INVOKE_MAP: bool> ControlPorts<INVOKE_MAP> {
    /// Return a reference to the port reads associated with the group.
    pub fn get(&self, group: &ir::Id) -> Option<&Vec<RRC<ir::Port>>> {
        self.cg_to_port.get(group)
    }

    /// Remove the port reads associated with the group.
    pub fn remove(&mut self, group: &ir::Id) -> Option<Vec<RRC<ir::Port>>> {
        self.cg_to_port.remove(group)
    }

    /// Get all bindings for an instance
    pub fn get_bindings(&self, instance: &ir::Id) -> Option<&Vec<Binding>> {
        if INVOKE_MAP {
            self.invoke_map.get(instance)
        } else {
            panic!("ControlPorts instance built without invoke_map")
        }
    }

    /// Return the entire invoke binding map.
    pub fn get_all_bindings(self) -> InvokeMap {
        if INVOKE_MAP {
            self.invoke_map
        } else {
            panic!("ControlPorts instance built without invoke_map")
        }
    }
}

impl<const INVOKE_MAP: bool> ControlPorts<INVOKE_MAP> {
    fn construct_static(&mut self, scon: &ir::StaticControl) {
        match scon {
            ir::StaticControl::Enable(_) => (),
            ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
                self.construct_static(body)
            }
        }
    }

    fn construct(&mut self, con: &ir::Control) {
        match con {
            ir::Control::Enable(_)
            | ir::Control::Empty(_)
            | ir::Control::StaticEnable(_) => {}
            ir::Control::Invoke(ir::Invoke {
                comp,
                comb_group,
                inputs,
                outputs,
                ..
            }) => {
                if let Some(c) = comb_group {
                    let cells = super::ReadWriteSet::uses(
                        c.borrow().assignments.iter(),
                    )
                    .map(|cell| cell.borrow().name())
                    .collect::<HashSet<_>>();
                    // Only add ports that come from cells used in this comb group.
                    let ports = inputs
                        .iter()
                        .map(|(_, port)| Rc::clone(port))
                        .filter(|port| {
                            cells.contains(&port.borrow().get_parent_name())
                        });
                    self.cg_to_port
                        .entry(c.borrow().name())
                        .or_default()
                        .extend(ports);
                }
                if INVOKE_MAP {
                    let name = comp.borrow().name();
                    let bindings = inputs
                        .iter()
                        .chain(outputs.iter())
                        .cloned()
                        .collect_vec();
                    self.invoke_map.entry(name).or_default().push(bindings);
                }
            }
            ir::Control::If(ir::If {
                cond,
                port,
                tbranch,
                fbranch,
                ..
            }) => {
                if let Some(c) = cond {
                    self.cg_to_port
                        .entry(c.borrow().name())
                        .or_default()
                        .push(Rc::clone(port));
                }

                self.construct(tbranch);
                self.construct(fbranch);
            }
            ir::Control::While(ir::While {
                cond, port, body, ..
            }) => {
                if let Some(c) = cond {
                    self.cg_to_port
                        .entry(c.borrow().name())
                        .or_default()
                        .push(Rc::clone(port));
                }
                self.construct(body);
            }
            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => {
                stmts.iter().for_each(|con| self.construct(con));
            }
            ir::Control::Static(sc) => self.construct_static(sc),
        }
    }
}

impl<const INVOKE_MAP: bool> From<&ir::Control> for ControlPorts<INVOKE_MAP> {
    fn from(con: &ir::Control) -> Self {
        let mut cp = ControlPorts {
            cg_to_port: HashMap::new(),
            invoke_map: HashMap::new(),
        };
        cp.construct(con);
        // Deduplicate all group port reads
        cp.cg_to_port.values_mut().for_each(|v| {
            *v = v.drain(..).unique_by(|p| p.borrow().canonical()).collect()
        });
        // Deduplicate all invoke bindings if map was constructed
        if INVOKE_MAP {
            cp.invoke_map.values_mut().for_each(|v| {
                *v = v
                    .drain(..)
                    .unique_by(|binding| {
                        binding
                            .clone()
                            .into_iter()
                            .map(|(p, v)| (p, v.borrow().canonical()))
                            .collect_vec()
                    })
                    .collect()
            });
        }
        cp
    }
}
