use crate::analysis;
use crate::errors::{CalyxResult, Error};
use crate::ir::traversal::{
    Action, ConstructVisitor, Named, VisResult, Visitor,
};
use crate::ir::{self, CloneName, LibrarySignatures};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

/// Tuple containing (port, set of ports).
/// When the first port is read from, all of the ports in the set must be written to.
type ReadTogether = (ir::Id, HashSet<ir::Id>);

/// Pass to check for common errors such as missing assignments to `done` holes
/// of groups.
pub struct Papercut {
    /// Map from (primitive name) -> Vec<(set of ports)>
    /// When any of the ports in a set is driven, all ports in that set must
    /// be driven.
    /// For example, when driving the `in` port of a register, the `write_en`
    /// signal must also be driven.
    write_together: HashMap<ir::Id, Vec<HashSet<ir::Id>>>,

    /// Map from (primitive name) -> Vec<(port, set of ports)>
    /// When the `port` in the tuple is being read from, all the ports in the
    /// set must be driven.
    read_together: HashMap<ir::Id, Vec<ReadTogether>>,
}

/// Construct @write_together specs from the primitive definitions.
fn write_together_specs<'a>(
    primitives: impl Iterator<Item = &'a ir::Primitive>,
) -> HashMap<ir::Id, Vec<HashSet<ir::Id>>> {
    let mut write_together = HashMap::new();
    for prim in primitives {
        let writes: Vec<HashSet<ir::Id>> = prim
            .find_all_with_attr("write_together")
            .into_iter()
            .map(|pd| {
                (
                    pd.attributes.get("write_together").unwrap(),
                    pd.name.clone(),
                )
            })
            .into_group_map()
            .into_values()
            .map(|writes| writes.into_iter().collect::<HashSet<_>>())
            .collect();
        if !writes.is_empty() {
            write_together.insert(prim.name.clone(), writes);
        }
    }
    write_together
}

/// Construct @read_together specs from the primitive definitions.
fn read_together_specs<'a>(
    primitives: impl Iterator<Item = &'a ir::Primitive>,
) -> CalyxResult<HashMap<ir::Id, Vec<ReadTogether>>> {
    let mut read_together = HashMap::new();
    for prim in primitives {
        let reads: Vec<ReadTogether> = prim
                .find_all_with_attr("read_together")
                .into_iter()
                .map(|pd| (pd.attributes.get("read_together").unwrap(), pd))
                .into_group_map()
                .into_values()
                .map(|ports| {
                    let (outputs, inputs): (Vec<_>, Vec<_>) =
                        ports.into_iter().partition(|&port| {
                            matches!(port.direction, ir::Direction::Output)
                        });
                    // There should only be one port in the read_together specification.
                    if outputs.len() != 1 {
                        return Err(Error::Papercut(format!("Invalid @read_together specification for primitive `{}`. Each specification groups is only allowed to have one output port specified.", prim.name), prim.name.clone()))
                    }
                    assert!(outputs.len() == 1);
                    Ok((
                        outputs[0].name.clone(),
                        inputs
                            .into_iter()
                            .map(|port| port.name.clone())
                            .collect::<HashSet<_>>(),
                    ))
                })
                .collect::<CalyxResult<_>>()?;
        if !reads.is_empty() {
            read_together.insert(prim.name.clone(), reads);
        }
    }
    Ok(read_together)
}

impl ConstructVisitor for Papercut {
    fn from(ctx: &ir::Context) -> CalyxResult<Self> {
        let write_together = write_together_specs(ctx.lib.sigs.values());
        let read_together = read_together_specs(ctx.lib.sigs.values())?;
        Ok(Papercut {
            write_together,
            read_together,
        })
    }

    fn clear_data(&mut self) {
        /* All data is shared between components. */
    }
}

impl Named for Papercut {
    fn name() -> &'static str {
        "papercut"
    }

    fn description() -> &'static str {
        "Detect various common made mistakes"
    }
}

/// Extract information about a port.
fn port_information(
    port_ref: ir::RRC<ir::Port>,
) -> Option<((ir::Id, ir::Id), ir::Id)> {
    let port = port_ref.borrow();
    if let ir::PortParent::Cell(cell_wref) = &port.parent {
        let cell_ref = cell_wref.upgrade();
        let cell = cell_ref.borrow();
        if let ir::CellType::Primitive { name, .. } = &cell.prototype {
            return Some((
                (cell.name().clone(), name.clone()),
                port.name.clone(),
            ));
        }
    }
    None
}

impl Visitor for Papercut {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult {
        // If the control program is empty, check that the `done` signal
        // has been assigned to.
        if let ir::Control::Empty(..) = *comp.control.borrow() {
            let done_use =
                comp.continuous_assignments.iter().find(|assign_ref| {
                    let assign = assign_ref.dst.borrow();
                    // If at least one assignment used the `done` port, then
                    // we're good.
                    assign.name == "done" && !assign.is_hole()
                });
            if done_use.is_none() {
                return Err(Error::Papercut(format!("Component `{}` has an empty control program and does not assign to the `done` port. Without an assignment to the `done`, the component cannot return control flow.", comp.name.clone()), comp.name.clone()));
            }
        }

        // For each component that's being driven in a group, make
        // sure all signals defined for that component's
        // `write_together' and `read_together' are also driven.
        // For example, for a register, both the `.in' port and the
        // `.write_en' port need to be driven.
        for group_ref in comp.groups.iter() {
            let group = group_ref.borrow();
            // Build a map from (instance name, primitive name) to the signals being
            // read from and written to.
            let all_writes =
                analysis::ReadWriteSet::port_write_set(&group.assignments)
                    .filter_map(port_information)
                    .into_grouping_map()
                    .collect::<HashSet<_>>();
            let all_reads =
                analysis::ReadWriteSet::port_read_set(&group.assignments)
                    .filter_map(port_information)
                    .into_grouping_map()
                    .collect::<HashSet<_>>();

            for ((inst, comp_type), reads) in all_reads {
                if let Some(spec) = self.read_together.get(&comp_type) {
                    let empty = HashSet::new();
                    let writes = all_writes
                        .get(&(inst.clone(), comp_type.clone()))
                        .unwrap_or(&empty);
                    for (read, required) in spec {
                        if reads.contains(read)
                            && matches!(
                                required.difference(writes).next(),
                                Some(_)
                            )
                        {
                            let missing = required
                                .difference(writes)
                                .sorted()
                                .map(|port| {
                                    format!("{}.{}", inst.clone(), port)
                                })
                                .join(", ");
                            let msg =
                                format!("Required signal not driven inside the group.\
                                        \nWhen read the port `{}.{}', the ports [{}] must be written to.\
                                        \nThe primitive type `{}' requires this invariant.",
                                        inst,
                                        read,
                                        missing,
                                        comp_type);
                            return Err(Error::Papercut(
                                msg,
                                group.clone_name(),
                            ));
                        }
                    }
                }
            }
            // Check if this matches the `write_together' and `read_together' specification.
            for ((inst, comp_type), writes) in all_writes {
                if let Some(spec) = self.write_together.get(&comp_type) {
                    for required in spec {
                        // It should either be the case that:
                        // 1. `writes` contains no writes that overlap with `required`
                        //     In which case `required - writes` == `required`.
                        // 2. `writes` contains writes that overlap with `required`
                        //     In which case `required - writes == {}`
                        let mut diff = required - &writes;
                        if !diff.is_empty() && diff != *required {
                            let first = writes.iter().sorted().next().unwrap();
                            let missing = diff
                                .drain()
                                .sorted()
                                .map(|port| format!("{}.{}", inst, port))
                                .join(", ");
                            let msg =
                                format!("Required signal not driven inside the group.\
                                        \nWhen writing to the port `{}.{}', the ports [{}] must also be written to.\
                                        \nThe primitive type `{}' requires this invariant.",
                                        inst,
                                        first,
                                        missing,
                                        comp_type);
                            return Err(Error::Papercut(
                                msg,
                                group.clone_name(),
                            ));
                        }
                    }
                }
            }
        }

        Ok(Action::Continue)
    }

    fn start_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult {
        if s.cond.is_none() {
            let port = s.port.borrow();
            if let ir::PortParent::Cell(cell_wref) = &port.parent {
                let cell_ref = cell_wref.upgrade();
                let cell = cell_ref.borrow();
                if let ir::CellType::Primitive { is_comb, name: prim_name, .. } = &cell.prototype {
                    if *is_comb {
                        let msg = format!("Port `{}.{}` is an output port on combinational primitive `{}` and will always output 0. Add a `with` statement to the `while` statement to ensure it has a valid value during execution.", cell.name(), port.name, prim_name);
                        return Err(Error::Papercut(msg, cell.name().clone()));
                    }
                }
            }
        }
        Ok(Action::Continue)
    }

    fn start_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult {
        if s.cond.is_none() {
            let port = s.port.borrow();
            if let ir::PortParent::Cell(cell_wref) = &port.parent {
                let cell_ref = cell_wref.upgrade();
                let cell = cell_ref.borrow();
                if let ir::CellType::Primitive { is_comb, name: prim_name, .. } = &cell.prototype {
                    if *is_comb {
                        let msg = format!("Port `{}.{}` is an output port on combinational primitive `{}` and will always output 0. Add a `with` statement to the `if` statement to ensure it has a valid value during execution.", cell.name(), port.name, prim_name);
                        return Err(Error::Papercut(msg, cell.name().clone()));
                    }
                }
            }
        }
        Ok(Action::Continue)
    }
}
