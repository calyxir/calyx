use crate::errors::Error;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, CloneName, LibrarySignatures};
use std::collections::{HashMap, HashSet};

/// Pass to check for common errors such as missing assignments to `done` holes
/// of groups.
pub struct Papercut<'a> {
    /// Map from (primitive name) -> (signal, signal).
    /// Implies that when the first signal is driven for the primitive, the
    /// second must also be driven.
    /// For example, when driving the input port of a register, the `write_en`
    /// signal must also be driven.
    drive_together: HashMap<&'a str, Vec<(&'a str, &'a str)>>,
}

impl Default for Papercut<'_> {
    fn default() -> Self {
        let drive_together = [
            ("std_reg", vec![("in", "write_en")]),
            (
                "std_mem_d1",
                vec![("write_data", "write_en"), ("write_data", "addr0")],
            ),
            (
                "std_mem_d2",
                vec![
                    ("write_data", "write_en"),
                    ("write_data", "addr0"),
                    ("write_data", "addr1"),
                ],
            ),
            (
                "std_mem_d3",
                vec![
                    ("write_data", "write_en"),
                    ("write_data", "addr0"),
                    ("write_data", "addr1"),
                    ("write_data", "addr2"),
                ],
            ),
            ("std_mul_pipe", vec![("go", "left"), ("go", "right")]),
            ("std_mod_pipe", vec![("go", "left"), ("go", "right")]),
        ]
        .iter()
        .cloned()
        .collect();
        Papercut { drive_together }
    }
}

impl Named for Papercut<'_> {
    fn name() -> &'static str {
        "papercut"
    }

    fn description() -> &'static str {
        "Detect various common made mistakes"
    }
}

impl Visitor for Papercut<'_> {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult {
        // For each group, check if there is at least one write to the done
        // signal of that group.
        // Names of the groups whose `done` hole has been written to.
        let mut hole_writes = HashSet::new();
        for group in comp.groups.iter() {
            for assign_ref in &group.borrow().assignments {
                let assign = assign_ref.dst.borrow();
                if assign.is_hole() && assign.name == "done" {
                    if let ir::PortParent::Group(group_ref) = &assign.parent {
                        hole_writes.insert(group_ref.upgrade().clone_name());
                    }
                }
            }
        }

        let no_done_group = comp
            .groups
            .iter()
            .find(|g| !hole_writes.contains(&g.borrow().name()))
            .map(|g| g.clone_name());

        // If there is a group that hasn't been assigned to, throw an error.
        if let Some(g) = no_done_group {
            return Err(Error::Papercut(
                format!(
                    "No writes to the `done' hole for group `{}'",
                    g.to_string()
                ),
                g,
            ));
        }

        // For each component that's being driven in a group, make
        // sure all signals defined for that component's
        // `drive_together' are also driven.
        // For example, for a register, both the `.in' port and the
        // `.write_en' port need to be driven.

        for group in comp.groups.iter() {
            // 1. Build a map from (instance_name, type) to the signals being
            // driven.
            let mut drives: HashMap<(String, String), Vec<String>> =
                HashMap::new();

            // Get all the input ports driven for each component in this
            // group.
            for assign in &group.borrow().assignments {
                let dst = assign.dst.borrow();
                if let ir::PortParent::Cell(cell_wref) = &dst.parent {
                    let cell_ref = cell_wref.upgrade();
                    let cell = cell_ref.borrow();
                    // If this is a primitive cell, collect the driver
                    if let ir::CellType::Primitive { name, .. } =
                        &cell.prototype
                    {
                        drives
                            .entry((cell.name().id.clone(), name.id.clone()))
                            .or_insert_with(Vec::new)
                            .push(dst.name.id.clone())
                    }
                }
            }

            // 2. Check if this matches the `drive_together' specification.
            for ((inst, comp_type), signals) in drives {
                if let Some(spec) = self.drive_together.get(comp_type.as_str())
                {
                    for (first, second) in spec {
                        // If the first signal is driven, the second must also be
                        // driven.
                        if signals.contains(&first.to_string())
                            && !signals.contains(&second.to_string())
                        {
                            let msg = format!(
                        "Required signal not driven inside the group.\nWhen driving the signal `{}.{}' the signal `{}.{}' must also be driven. The primitive type `{}' requires this invariant.",
                        inst,
                        first,
                        inst,
                        second,
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

        Ok(Action::Stop)
    }
}
