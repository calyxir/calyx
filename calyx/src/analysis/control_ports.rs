use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use itertools::Itertools;

use crate::ir::{self, CloneName, RRC};

type PortMap = HashMap<ir::Id, Vec<RRC<ir::Port>>>;
type Binding = Vec<(ir::Id, RRC<ir::Port>)>;
type InvokeMap = HashMap<ir::Id, Vec<Binding>>;

/// Contains a mapping from name of [ir::CombGroup] to the ports read by the control program
/// as well as the mapping from invoke statements to the port mappings.
/// The vector of ports is guaranteed to only contain unique ports.
pub struct ControlPorts {
    // Map name of combinational group to the ports read by the control program.
    used_ports: PortMap,
    // Mapping from name of invoke instance to the port bindings.
    invoke_map: InvokeMap,
}

impl ControlPorts {
    /// Return a reference to the port reads associated with the group.
    pub fn get(&self, group: &ir::Id) -> Option<&Vec<RRC<ir::Port>>> {
        self.used_ports.get(group)
    }

    /// Remove the port reads associated with the group.
    pub fn remove(&mut self, group: &ir::Id) -> Option<Vec<RRC<ir::Port>>> {
        self.used_ports.remove(group)
    }

    /// Get all bindings for an instance
    pub fn get_bindings(&self, instance: &ir::Id) -> Option<&Vec<Binding>> {
        self.invoke_map.get(instance)
    }
}

/// Helper method to construct a [ControlPorts] instance.
fn construct(
    con: &ir::Control,
    used_ports: &mut PortMap,
    invoke_map: &mut InvokeMap,
) {
    match con {
        ir::Control::Enable(_) | ir::Control::Empty(_) => {}
        ir::Control::Invoke(ir::Invoke {
            comp,
            comb_group,
            inputs,
            outputs,
            ..
        }) => {
            if let Some(c) = comb_group {
                let cells = super::ReadWriteSet::uses(&c.borrow().assignments)
                    .into_iter()
                    .map(|cell| cell.clone_name())
                    .collect::<HashSet<_>>();
                // Only add ports that come from cells used in this comb group.
                let ports =
                    inputs.iter().map(|(_, port)| Rc::clone(port)).filter(
                        |port| cells.contains(&port.borrow().get_parent_name()),
                    );
                used_ports
                    .entry(c.borrow().name().clone())
                    .or_default()
                    .extend(ports);
            }
            let name = comp.borrow().clone_name();
            let bindings =
                inputs.iter().chain(outputs.iter()).cloned().collect_vec();
            invoke_map.entry(name).or_default().push(bindings);
        }
        ir::Control::If(ir::If {
            cond,
            port,
            tbranch,
            fbranch,
            ..
        }) => {
            if let Some(c) = cond {
                used_ports
                    .entry(c.borrow().name().clone())
                    .or_default()
                    .push(Rc::clone(port));
            }

            construct(tbranch, used_ports, invoke_map);
            construct(fbranch, used_ports, invoke_map);
        }
        ir::Control::While(ir::While {
            cond, port, body, ..
        }) => {
            if let Some(c) = cond {
                used_ports
                    .entry(c.borrow().name().clone())
                    .or_default()
                    .push(Rc::clone(port));
            }
            construct(body, used_ports, invoke_map);
        }
        ir::Control::Seq(ir::Seq { stmts, .. })
        | ir::Control::Par(ir::Par { stmts, .. }) => {
            stmts
                .iter()
                .for_each(|con| construct(con, used_ports, invoke_map));
        }
    }
}

impl From<&ir::Control> for ControlPorts {
    fn from(con: &ir::Control) -> Self {
        let mut used_ports = HashMap::new();
        let mut invoke_map = HashMap::new();
        construct(con, &mut used_ports, &mut invoke_map);
        // Deduplicate all group port reads
        used_ports.values_mut().for_each(|v| {
            *v = v.drain(..).unique_by(|p| p.borrow().canonical()).collect()
        });
        // Deduplicate all invoke bindings
        invoke_map.values_mut().for_each(|v| {
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
        ControlPorts {
            used_ports,
            invoke_map,
        }
    }
}
