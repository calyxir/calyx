use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use itertools::Itertools;

use crate::ir::{self, CloneName, RRC};

/// Contains a mapping from name of groups to the ports read by the control
/// program.
/// The vector of ports is guaranteed to only contain unique ports.
pub struct ControlPorts {
    used_ports: HashMap<ir::Id, Vec<RRC<ir::Port>>>,
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
}

/// Helper method to construct a [ControlPorts] instance.
fn construct(
    con: &ir::Control,
    used_ports: &mut HashMap<ir::Id, Vec<RRC<ir::Port>>>,
) {
    match con {
        ir::Control::Enable(_) | ir::Control::Empty(_) => {}
        ir::Control::Invoke(ir::Invoke {
            comb_group, inputs, ..
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

            construct(tbranch, used_ports);
            construct(fbranch, used_ports);
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
            construct(body, used_ports);
        }
        ir::Control::Seq(ir::Seq { stmts, .. })
        | ir::Control::Par(ir::Par { stmts, .. }) => {
            stmts.iter().for_each(|con| construct(con, used_ports));
        }
    }
}

impl From<&ir::Control> for ControlPorts {
    fn from(con: &ir::Control) -> Self {
        let mut used_ports = HashMap::default();
        construct(con, &mut used_ports);
        // Deduplicate all vectors
        used_ports.values_mut().for_each(|v| {
            *v = v.drain(..).unique_by(|p| p.borrow().canonical()).collect()
        });
        ControlPorts { used_ports }
    }
}
