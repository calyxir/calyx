use crate::errors::{CalyxResult, Error, WithPos};
use crate::ir::traversal::ConstructVisitor;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{
    self, CellType, CloneName, Component, LibrarySignatures, RESERVED_NAMES,
};
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::collections::HashSet;

/// Pass to check if the program is well-formed.
///
/// Catches the following errors:
/// 1. Programs that don't use a defined group or combinational group.
/// 2. Groups that don't write to their done signal.
/// 3. Groups that write to another group's done signal.
pub struct WellFormed {
    /// Reserved names
    reserved_names: HashSet<String>,
    /// Names of the groups that have been used in the control.
    used_groups: HashSet<ir::Id>,
    /// Names of combinational groups used in the control.
    used_comb_groups: HashSet<ir::Id>,
    /// external cell types of components used in the control.
    external_cell_types: HashMap<ir::Id, LinkedHashMap<ir::Id, CellType>>,
}

impl ConstructVisitor for WellFormed {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized,
    {
        let reserved_names =
            RESERVED_NAMES.iter().map(|s| s.to_string()).collect();

        let mut e_c_t = HashMap::new();
        for comp in ctx.components.iter() {
            let mut cellmap: LinkedHashMap<ir::Id, CellType> =
                LinkedHashMap::new();
            for cell in comp.cells.iter() {
                if cell.borrow().is_external() {
                    if comp.name.eq(&"main") {
                        return Err(Error::malformed_structure(
                            "external cell not allowed for main component",
                        )
                        .with_pos(cell.borrow().name()));
                    }
                    cellmap.insert(
                        cell.clone_name(),
                        cell.borrow().prototype.clone(),
                    );
                }
            }
            e_c_t.insert(comp.name.clone(), cellmap);
        }

        let w_f = WellFormed {
            reserved_names,
            used_groups: HashSet::new(),
            used_comb_groups: HashSet::new(),
            external_cell_types: e_c_t,
        };

        Ok(w_f)
    }

    fn clear_data(&mut self) {
        // Data is shared between components
    }
}

impl Named for WellFormed {
    fn name() -> &'static str {
        "well-formed"
    }

    fn description() -> &'static str {
        "Check if the structure and control are well formed."
    }
}

/// Returns an error if the assignments are obviously conflicting. This happens when two
/// assignments assign to the same port unconditionally.
fn obvious_conflicts<'a, I>(assigns: I) -> CalyxResult<()>
where
    I: Iterator<Item = &'a ir::Assignment>,
{
    let dst_grps = assigns
        .filter(|a| a.guard.is_true())
        .map(|a| (a.dst.borrow().canonical(), a))
        .sorted_by(|(dst1, _), (dst2, _)| ir::Canonical::cmp(dst1, dst2))
        .group_by(|(dst, _)| dst.clone());

    for (_, group) in &dst_grps {
        let assigns = group.map(|(_, a)| a).collect_vec();
        if assigns.len() > 1 {
            let msg = assigns
                .into_iter()
                .map(|a| {
                    a.attributes
                        .copy_span()
                        .map(|s| s.show())
                        .unwrap_or_else(|| ir::Printer::assignment_to_str(a))
                })
                .join("");
            return Err(Error::malformed_structure(format!(
                "Obviously conflicting assignments found:\n{}",
                msg
            )));
        }
    }
    Ok(())
}

fn same_binding(
    binding_out: &SmallVec<[(ir::Id, u64); 5]>,
    binding_in: &SmallVec<[(ir::Id, u64); 5]>,
) -> CalyxResult<()> {
    if (*binding_out).len() != (*binding_in).len() {
        return Err(Error::malformed_control(
            "unmatching binding sizes".to_string(),
        ));
    }

    binding_out.iter().zip(binding_in.iter()).try_for_each(
        |((k_out, v_out), (k_in, v_in))| {
            if k_out.eq(k_in) && (v_out == v_in) {
                Ok(())
            } else {
                Err(Error::malformed_control(
                    "unmatching binding values for fed_in external cell"
                        .to_string(),
                )
                .with_pos(k_in))
            }
        },
    )
}

fn same_type(proto_out: &CellType, proto_in: &CellType) -> CalyxResult<()> {
    match (proto_out, proto_in) {
        (
            CellType::Primitive {
                name,
                param_binding,
                ..
            },
            CellType::Primitive {
                name: name_in,
                param_binding: param_binding_in,
                ..
            },
        ) => {
            if name_in.eq(name) {
                same_binding(param_binding, param_binding_in)
            } else {
                Err(Error::malformed_control(format!(
                    "type mismatch, expected {}, got {}",
                    name, name_in
                ))
                .with_pos(name))
            }
        }
        (
            CellType::Component { name },
            CellType::Component { name: name_in },
        ) => {
            if name.eq(name_in) {
                Ok(())
            } else {
                Err(Error::malformed_control("type mismatch: cell type not component or incorrect component name". to_string()).with_pos(name))
            }
        }
        _ => Err(Error::malformed_control(
            "type mismatch: unallowed type".to_string(),
        )),
    }
}

impl Visitor for WellFormed {
    fn start(
        &mut self,
        comp: &mut Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Check if any of the cells use a reserved name.
        for cell_ref in comp.cells.iter() {
            let cell = cell_ref.borrow();
            if self.reserved_names.contains(&cell.name().id) {
                return Err(Error::reserved_name(cell.clone_name())
                    .with_pos(cell.name()));
            }
            if cell.is_external() {
                if cell.is_primitive(Some("std_const")) {
                    return Err(Error::malformed_structure(
                        "constant not allowed for external cells".to_string(),
                    )
                    .with_pos(cell.name()));
                }
                if matches!(cell.prototype, CellType::ThisComponent) {
                    return Err(Error::malformed_structure(
                        "the current component not allowed for external cells"
                            .to_string(),
                    )
                    .with_pos(cell.name()));
                }
            }
        }

        // For each non-combinational group, check if there is at least one write to the done
        // signal of that group and that the write is to the group's done signal.
        comp.groups.iter().try_for_each(|group_ref| {
            let group = group_ref.borrow();
            let gname = group.name();
            // Find an assignment writing to this group's done condition.
            let done = group.assignments.iter().filter(|assign| {
                let dst = assign.dst.borrow();
                dst.is_hole()
                    && dst.name == "done"
            }).map(|assign| {
                let dst = assign.dst.borrow();
                if gname != &dst.get_parent_name() {
                    Err(Error::malformed_structure(
                            format!("Group `{}` refers to the done condition of another group (`{}`).",
                            group.name(),
                            dst.get_parent_name())).with_pos(&dst.attributes))
                } else {
                    Ok(())
                }
            }).collect::<CalyxResult<Vec<_>>>()?;
            if done.is_empty() {
                Err(Error::malformed_structure(format!(
                    "No writes to the `done' hole for group `{gname}'",
                )).with_pos(&group.attributes))
            } else {
                Ok(())
            }
        })?;

        // Check for obvious conflicting assignments
        for gr in comp.groups.iter() {
            obvious_conflicts(
                gr.borrow()
                    .assignments
                    .iter()
                    .chain(comp.continuous_assignments.iter()),
            )?;
        }
        for cgr in comp.comb_groups.iter() {
            obvious_conflicts(
                cgr.borrow()
                    .assignments
                    .iter()
                    .chain(comp.continuous_assignments.iter()),
            )?;
        }
        obvious_conflicts(comp.continuous_assignments.iter())?;

        Ok(Action::Continue)
    }

    fn enable(
        &mut self,
        s: &mut ir::Enable,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        self.used_groups.insert(s.group.clone_name());

        let group = s.group.borrow();
        let asgn = group.done_cond();
        let const_done_assign =
            asgn.guard.is_true() && asgn.src.borrow().is_constant(1, 1);

        if const_done_assign {
            return Err(Error::malformed_structure("Group with constant done condition is invalid. Use `comb group` instead to define a combinational group.").with_pos(&group.attributes));
        }

        // A group with "static"=0 annotation
        if group
            .attributes
            .get("static")
            .map(|v| *v == 0)
            .unwrap_or(false)
        {
            return Err(Error::malformed_structure("Group with annotation \"static\"=0 is invalid. Use `comb group` instead to define a combinational group or if the group's done condition is not constant, provide the correct \"static\" annotation.").with_pos(&group.attributes));
        }

        Ok(Action::Continue)
    }

    fn invoke(
        &mut self,
        s: &mut ir::Invoke,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if let Some(c) = &s.comb_group {
            self.used_comb_groups.insert(c.clone_name());
        }
        // Only refers to ports defined in the invoked instance.
        let cell = s.comp.borrow();
        let ports: HashSet<_> =
            cell.ports.iter().map(|p| p.borrow().name.clone()).collect();

        s.inputs
            .iter()
            .chain(s.outputs.iter())
            .try_for_each(|(port, _)| {
                if !ports.contains(port) {
                    Err(Error::malformed_structure(format!(
                        "`{}` does not have port named `{}`",
                        cell.name(),
                        port
                    ))
                    .with_pos(&s.attributes))
                } else {
                    Ok(())
                }
            })?;

        if let CellType::Component { name: id } = &cell.prototype {
            let cellmap = self.external_cell_types.get(id).unwrap();
            let mut mentioned_cells = HashSet::new();
            for (outcell, incell) in s.external_cells.iter() {
                if let Some(t) = cellmap.get(&(*outcell)) {
                    let proto = incell.borrow().prototype.clone();
                    same_type(t, &proto)?;
                    mentioned_cells.insert(&(*outcell));
                } else {
                    return Err(Error::malformed_control(format!(
                        "{} does not have external cell named {}",
                        id, *outcell
                    ))
                    .with_pos(outcell));
                }
            }
            for id in cellmap.keys() {
                if mentioned_cells.get(id).is_none() {
                    return Err(Error::malformed_control(format!(
                        "unmentioned external cell: {}",
                        id
                    ))
                    .with_pos(s.comp.borrow().name()));
                }
            }
        }

        Ok(Action::Continue)
    }

    fn finish_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Add cond group as a used port.
        if let Some(cond) = &s.cond {
            self.used_comb_groups.insert(cond.clone_name());
        }
        Ok(Action::Continue)
    }

    fn finish_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Add cond group as a used port.
        if let Some(cond) = &s.cond {
            self.used_comb_groups.insert(cond.clone_name());
        }
        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Go signals of groups mentioned in other groups are considered used
        comp.for_each_assignment(|assign| {
            assign.for_each_port(|pr| {
                let port = pr.borrow();
                if port.is_hole() && port.name == "go" {
                    self.used_groups.insert(port.get_parent_name());
                }
                None
            })
        });

        // Find unused groups
        let all_groups: HashSet<ir::Id> =
            comp.groups.iter().map(|g| g.clone_name()).collect();
        if let Some(group) =
            all_groups.difference(&self.used_groups).into_iter().next()
        {
            let gr = comp.find_group(&group).unwrap();
            let gr = gr.borrow();
            return Err(
                Error::unused(group.clone(), "group").with_pos(&gr.attributes)
            );
        };

        let all_comb_groups: HashSet<ir::Id> =
            comp.comb_groups.iter().map(|g| g.clone_name()).collect();
        if let Some(comb_group) = all_comb_groups
            .difference(&self.used_comb_groups)
            .into_iter()
            .next()
        {
            let cgr = comp.find_comb_group(&comb_group).unwrap();
            let cgr = cgr.borrow();
            return Err(Error::unused(
                comb_group.clone(),
                "combinational group",
            )
            .with_pos(&cgr.attributes));
        }
        Ok(Action::Continue)
    }
}
