use crate::errors::Error;
use crate::frontend::library::ast as lib;
use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use std::collections::{HashMap, HashSet};

/// Pass to check for common errors such as missing assignments to `done' holes
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
            (
                "std_mem_d1_ext",
                vec![("write_data", "write_en"), ("write_data", "addr0")],
            ),
            (
                "std_mem_d2_ext",
                vec![
                    ("write_data", "write_en"),
                    ("write_data", "addr0"),
                    ("write_data", "addr1"),
                ],
            ),
            (
                "std_mem_d3_ext",
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
        _ctx: &lib::LibrarySignatures,
    ) -> VisResult {
        //let st = &comp.structure;
        // For each group, check if there is at least one write to the done
        // signal of that group.
        // Names of the groups whose `done` hole has been written to.
        let mut hole_writes = HashSet::new();
        for group in &comp.groups {
            for assign_ref in &group.borrow().assignments {
                let assign = assign_ref.dst.borrow();
                if assign.is_hole() && assign.name == "done" {
                    if let ir::PortParent::Group(group_ref) = &assign.parent {
                        hole_writes.insert(
                            group_ref.upgrade().unwrap().borrow().name.clone(),
                        );
                    }
                }
            }
        }

        let no_done_group = comp
            .groups
            .iter()
            .find(|g| !hole_writes.contains(&g.borrow().name))
            .map(|g| g.borrow().name.clone());

        // If there is a group that hasn't been assigned to, throw an error.
        if let Some(g) = no_done_group {
            return Err(Error::Papercut(format!(
                "No writes to the `done' hole for group `{}'",
                g.to_string()
            )));
        }

        // For each component that's being driven in a group, make
        // sure all signals defined for that component's
        // `drive_together' are also driven.
        // For example, for a register, both the `.in' port and the
        // `.write_en' port need to be driven.

        /*for (_, (_, edge_indices)) in st.groups.iter() {
        // 1. Build a map from (instance_name, type) to the signals being
        // driven.
        let mut drives: HashMap<(&str, &str), Vec<&Id>> = HashMap::new();

        for eidx in edge_indices.iter() {
            let edge = st.get_edge(*eidx);
            let (_, dst) = st.endpoints(*eidx);

            // Get the type of this Cell. Ignores non-primitive cells.
            if let NodeData::Cell(ast::Cell::Prim {
                data: ast::Prim { instance, .. },
            }) = &st.get_node(dst).data
            {
                if let Port::Comp { component, port } = &edge.dest {
                    drives
                        .entry((&component.id, &instance.name.id))
                        .or_insert_with(Vec::new)
                        .push(&port)
                }
            }
        }*/

        // 2. Check if this matches the `drive_together' specification.
        /*for ((inst, comp_type), signals) in drives {
            if let Some(spec) = self.drive_together.get(comp_type) {
                for (first, second) in spec {
                    let first_id: Id = (**first).into();
                    let second_id: Id = (**second).into();
                    // If the first signal is driven, the second must also be
                    /https://joebiden.com/immigration/#/ driven.
                    if signals.contains(&&first_id)
                        && !signals.contains(&&second_id)
                    {
                        let msg = format!(
                        "Required signal not driven inside the group.\nWhen driving the signal `{}.{}' the signal `{}.{}' must also be driven. The primitive type `{}' requires this invariant.",
                        inst,
                        first,
                        inst,
                        second,
                        comp_type);
                        let loc_id = signals
                            .into_iter()
                            .find(|&id| id == &first_id)
                            .expect("Contained ID is missing.");
                        return Err(Error::Papercut(msg, loc_id.clone()));
                    }
                }
            }
        }*/
        Ok(Action::Stop)
    }
}
