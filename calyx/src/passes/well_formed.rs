use crate::errors::{CalyxResult, Error, WithPos};
use crate::ir::traversal::ConstructVisitor;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{
    self, CellType, CloneName, Component, LibrarySignatures, RESERVED_NAMES,
};
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;
use std::collections::HashMap;
use std::collections::HashSet;

/// Pass to check if the program is well-formed.
///
/// Catches the following errors:
/// 1. Programs that don't use a defined group or combinational group.
/// 2. Groups that don't write to their done signal.
/// 3. Groups that write to another group's done signal.
/// 4. Ref cells that have unallowed types.
/// 5. Invoking components with unmentioned ref cells.
/// 6. Invoking components with wrong ref cell name.
/// 7. Invoking components with impatible fed-in cell type for ref cells.
pub struct WellFormed {
    /// Reserved names
    reserved_names: HashSet<String>,
    /// Names of the groups that have been used in the control.
    used_groups: HashSet<ir::Id>,
    /// Names of combinational groups used in the control.
    used_comb_groups: HashSet<ir::Id>,
    /// ref cell types of components used in the control.
    ref_cell_types: HashMap<ir::Id, LinkedHashMap<ir::Id, CellType>>,
}

impl ConstructVisitor for WellFormed {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized,
    {
        let reserved_names =
            RESERVED_NAMES.iter().map(|s| s.to_string()).collect();

        for prim in ctx.lib.signatures() {
            if prim.attributes.has("static") {
                return Err(Error::malformed_structure(format!("Primitive `{}`: Defining @static attributes on components is deprecated. Place the @static attribute on the port marked as @go", prim.name)));
            }
        }

        let mut ref_cell_types = HashMap::new();
        for comp in ctx.components.iter() {
            // Defining @static on the component is meaningless
            if comp.attributes.has("static") {
                return Err(Error::malformed_structure(format!("Component `{}`: Defining @static attributes on components is deprecated. Place the @static attribute on the port marked as @go", comp.name)));
            }

            // Main component cannot use `ref` cells
            if comp.name == ctx.entrypoint {
                for cell in comp.cells.iter() {
                    if cell.borrow().is_reference() {
                        return Err(Error::malformed_structure(
                            "ref cells are not allowed for main component",
                        )
                        .with_pos(cell.borrow().name()));
                    }
                }
            }

            // Non-main components cannot use @external attribute
            let cellmap: LinkedHashMap<ir::Id, CellType> = comp
                .cells
                .iter()
                .filter_map(|cr| {
                    let cell = cr.borrow();
                    // Make sure @external cells are not defined in non-entrypoint components
                    if cell.attributes.has("external")
                        && comp.name != ctx.entrypoint
                    {
                        Some(Err(Error::malformed_structure("Cell cannot be marked `@external` in non-entrypoint component").with_pos(&cell.attributes)))
                    } else if cell.is_reference() {
                        Some(Ok((cell.clone_name(), cell.prototype.clone())))
                    } else {
                        None
                    }
                })
                .collect::<CalyxResult<_>>()?;
            ref_cell_types.insert(comp.name.clone(), cellmap);
        }

        let w_f = WellFormed {
            reserved_names,
            used_groups: HashSet::new(),
            used_comb_groups: HashSet::new(),
            ref_cell_types,
        };

        Ok(w_f)
    }

    fn clear_data(&mut self) {
        self.used_groups = HashSet::default();
        self.used_comb_groups = HashSet::default();
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

fn same_type(proto_out: &CellType, proto_in: &CellType) -> CalyxResult<()> {
    if proto_out != proto_in {
        Err(Error::malformed_control(format!(
            "Unexpected type for ref cell. Expected `{}`, received `{}`",
            proto_out.surface_name().unwrap(),
            proto_in.surface_name().unwrap(),
        )))
    } else {
        Ok(())
    }
}

impl Visitor for WellFormed {
    fn start(
        &mut self,
        comp: &mut Component,
        _ctx: &LibrarySignatures,
        comps: &[ir::Component],
    ) -> VisResult {
        for cell_ref in comp.cells.iter() {
            let cell = cell_ref.borrow();
            // Check if any of the cells use a reserved name.
            if self.reserved_names.contains(&cell.name().id) {
                return Err(Error::reserved_name(cell.clone_name())
                    .with_pos(cell.name()));
            }
            // Check if a `ref` cell is invalid
            if cell.is_reference() {
                if cell.is_primitive(Some("std_const")) {
                    return Err(Error::malformed_structure(
                        "constant not allowed for ref cells".to_string(),
                    )
                    .with_pos(cell.name()));
                }
                if matches!(cell.prototype, CellType::ThisComponent) {
                    unreachable!(
                        "the current component not allowed for ref cells"
                    );
                }
            }
        }

        // If the component is combinational, make sure all cells are also combinational,
        // there are no group or comb group definitions, and the control program is empty
        if comp.is_comb {
            if !matches!(&*comp.control.borrow(), ir::Control::Empty(..)) {
                return Err(Error::malformed_structure(format!("Component `{}` is marked combinational but has a non-empty control program", comp.name)));
            }

            if !comp.groups.is_empty() {
                let group = comp.groups.iter().next().unwrap().borrow();
                return Err(Error::malformed_structure(format!("Component `{}` is marked combinational but contains a group `{}`", comp.name, group.clone_name())).with_pos(&group.attributes));
            }

            if !comp.comb_groups.is_empty() {
                let group = comp.comb_groups.iter().next().unwrap().borrow();
                return Err(Error::malformed_structure(format!("Component `{}` is marked combinational but contains a group `{}`", comp.name, group.clone_name())).with_pos(&group.attributes));
            }

            for cell_ref in comp.cells.iter() {
                let cell = cell_ref.borrow();
                let is_comb = match &cell.prototype {
                    CellType::Primitive { is_comb, .. } => is_comb.to_owned(),
                    CellType::Constant { .. } => true,
                    CellType::Component { name } => {
                        let comp_idx =
                            comps.iter().position(|x| x.name == name).unwrap();
                        let comp = comps
                            .get(comp_idx)
                            .expect("Found cell that does not exist");
                        comp.is_comb
                    }
                    _ => false,
                };
                if !is_comb {
                    return Err(Error::malformed_structure(format!("Component `{}` is marked combinational but contains non-combinational cell `{}`", comp.name, cell.name())).with_pos(&cell.attributes));
                }
            }
        }

        // For each non-combinational group, check if there is at least one write to the done
        // signal of that group and that the write is to the group's done signal.
        for gr in comp.groups.iter() {
            let group = gr.borrow();
            let gname = group.name();
            let mut has_done = false;
            // Find an assignment writing to this group's done condition.
            for assign in &group.assignments {
                let dst = assign.dst.borrow();
                if dst.is_hole() && dst.name == "done" {
                    // Group has multiple done conditions
                    if has_done {
                        return Err(Error::malformed_structure(format!(
                            "Group `{}` has multiple done conditions",
                            gname
                        ))
                        .with_pos(&assign.attributes));
                    } else {
                        has_done = true;
                    }
                    // Group uses another group's done condition
                    if gname != &dst.get_parent_name() {
                        return Err(Error::malformed_structure(
                            format!("Group `{}` refers to the done condition of another group (`{}`).",
                            gname,
                            dst.get_parent_name())).with_pos(&dst.attributes));
                    }
                }
            }

            // Group does not have a done condition
            if !has_done {
                return Err(Error::malformed_structure(format!(
                    "No writes to the `done' hole for group `{gname}'",
                ))
                .with_pos(&group.attributes));
            }
        }

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
            let cellmap = &self.ref_cell_types[id];
            let mut mentioned_cells = HashSet::new();
            for (outcell, incell) in s.ref_cells.iter() {
                if let Some(t) = cellmap.get(outcell) {
                    let proto = incell.borrow().prototype.clone();
                    same_type(t, &proto)
                        .map_err(|err| err.with_pos(&s.attributes))?;
                    mentioned_cells.insert(outcell.clone());
                } else {
                    return Err(Error::malformed_control(format!(
                        "{} does not have ref cell named {}",
                        id, outcell
                    ))
                    .with_pos(outcell));
                }
            }
            for id in cellmap.keys() {
                if mentioned_cells.get(id).is_none() {
                    return Err(Error::malformed_control(format!(
                        "unmentioned ref cell: {}",
                        id
                    ))
                    .with_pos(&s.attributes));
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
