use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{
    self as ir, CellType, Component, GetAttributes, LibrarySignatures,
    RESERVED_NAMES,
};
use calyx_utils::{CalyxResult, Error, WithPos};
use ir::Nothing;
use ir::StaticTiming;
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;
use std::collections::HashMap;
use std::collections::HashSet;

// given a port and a vec of components `comps`,
// returns true if the port's parent is a static primitive
// otherwise returns false
fn port_is_static_prim(port: &ir::Port) -> bool {
    // if port parent is hole then obviously not static
    let parent_cell = match &port.parent {
        ir::PortParent::Cell(cell_wref) => cell_wref.upgrade(),
        ir::PortParent::Group(_) | ir::PortParent::StaticGroup(_) => {
            return false
        }
    };
    // if celltype is this component/constant, then obviously not static
    // if primitive, then we can quickly check whether it is static
    // if component, then we have to go throuch `comps` to see whether its static
    // for some reason, need to store result in variable, otherwise it gives a
    // lifetime error
    let res = match parent_cell.borrow().prototype {
        ir::CellType::Primitive { latency, .. } => latency.is_some(),
        ir::CellType::Component { .. }
        | ir::CellType::ThisComponent
        | ir::CellType::Constant { .. } => false,
    };
    res
}

#[derive(Default)]
struct ActiveAssignments {
    // Set of currently active assignments
    assigns: Vec<ir::Assignment<Nothing>>,
    // Stack representing the number of assignments added at each level
    num_assigns: Vec<usize>,
}
impl ActiveAssignments {
    /// Push a set of assignments to the stack.
    pub fn push(&mut self, assign: &[ir::Assignment<Nothing>]) {
        let prev_size = self.assigns.len();
        self.assigns.extend(assign.iter().cloned());
        // Number of assignments added at this level
        self.num_assigns.push(self.assigns.len() - prev_size);
    }

    /// Pop the last set of assignments from the stack.
    pub fn pop(&mut self) {
        let num_assigns = self.num_assigns.pop().unwrap();
        self.assigns.truncate(self.assigns.len() - num_assigns);
    }

    pub fn iter(&self) -> impl Iterator<Item = &ir::Assignment<Nothing>> {
        self.assigns.iter()
    }
}

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
    reserved_names: HashSet<ir::Id>,
    /// Names of the groups that have been used in the control.
    used_groups: HashSet<ir::Id>,
    /// Names of combinational groups used in the control.
    used_comb_groups: HashSet<ir::Id>,
    /// ref cell types of components used in the control.
    ref_cell_types: HashMap<ir::Id, LinkedHashMap<ir::Id, CellType>>,
    /// Stack of currently active combinational groups
    active_comb: ActiveAssignments,
}

impl ConstructVisitor for WellFormed {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized,
    {
        let reserved_names =
            RESERVED_NAMES.iter().map(|s| ir::Id::from(*s)).collect();

        let mut ref_cell_types = HashMap::new();
        for comp in ctx.components.iter() {
            // Non-main components cannot use @external attribute
            let cellmap: LinkedHashMap<ir::Id, CellType> = comp
                .cells
                .iter()
                .filter_map(|cr| {
                    let cell = cr.borrow();
                    // Make sure @external cells are not defined in non-entrypoint components
                    if cell.attributes.has(ir::BoolAttr::External)
                        && comp.name != ctx.entrypoint
                    {
                        Some(Err(Error::malformed_structure("Cell cannot be marked `@external` in non-entrypoint component").with_pos(&cell.attributes)))
                    } else if cell.is_reference() {
                        Some(Ok((cell.name(), cell.prototype.clone())))
                    } else {
                        None
                    }
                })
                .collect::<CalyxResult<_>>()?;
            ref_cell_types.insert(comp.name, cellmap);
        }

        let w_f = WellFormed {
            reserved_names,
            used_groups: HashSet::new(),
            used_comb_groups: HashSet::new(),
            ref_cell_types,
            active_comb: ActiveAssignments::default(),
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
/// Because there are two types of assignments, we take in `assigns1` and `assigns2`.
/// Regardless, we check for conflicts across (assigns1.chained(assigns2)).
fn obvious_conflicts<'a, I1, I2>(assigns1: I1, assigns2: I2) -> CalyxResult<()>
where
    I1: Iterator<Item = &'a ir::Assignment<Nothing>>,
    I2: Iterator<Item = &'a ir::Assignment<StaticTiming>>,
{
    let dsts1 = assigns1.filter(|a| a.guard.is_true()).map(|a| {
        (
            a.dst.borrow().canonical(),
            a.attributes
                .copy_span()
                .into_option()
                .map(|s| s.show())
                .unwrap_or_else(|| ir::Printer::assignment_to_str(a)),
        )
    });
    let dsts2 = assigns2.filter(|a| a.guard.is_true()).map(|a| {
        (
            a.dst.borrow().canonical(),
            a.attributes
                .copy_span()
                .into_option()
                .map(|s| s.show())
                .unwrap_or_else(|| ir::Printer::assignment_to_str(a)),
        )
    });
    let dsts = dsts1.chain(dsts2);
    let dst_grps = dsts
        .sorted_by(|(dst1, _), (dst2, _)| ir::Canonical::cmp(dst1, dst2))
        .group_by(|(dst, _)| dst.clone());

    for (_, group) in &dst_grps {
        let assigns = group.map(|(_, a)| a).collect_vec();
        if assigns.len() > 1 {
            let msg = assigns.into_iter().join("");
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
            if self.reserved_names.contains(&cell.name()) {
                return Err(Error::reserved_name(cell.name())
                    .with_pos(cell.get_attributes()));
            }
            // Check if a `ref` cell is invalid
            if cell.is_reference() {
                if cell.is_primitive(Some("std_const")) {
                    return Err(Error::malformed_structure(
                        "constant not allowed for ref cells".to_string(),
                    )
                    .with_pos(cell.get_attributes()));
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

            if !comp.get_groups().is_empty() {
                let group = comp.get_groups().iter().next().unwrap().borrow();
                return Err(Error::malformed_structure(format!("Component `{}` is marked combinational but contains a group `{}`", comp.name, group.name())).with_pos(&group.attributes));
            }

            if !comp.get_static_groups().is_empty() {
                let group =
                    comp.get_static_groups().iter().next().unwrap().borrow();
                return Err(Error::malformed_structure(format!("Component `{}` is marked combinational but contains a group `{}`", comp.name, group.name())).with_pos(&group.attributes));
            }

            if !comp.comb_groups.is_empty() {
                let group = comp.comb_groups.iter().next().unwrap().borrow();
                return Err(Error::malformed_structure(format!("Component `{}` is marked combinational but contains a group `{}`", comp.name, group.name())).with_pos(&group.attributes));
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
        // in ast_to_ir, we should have already checked that static components have static_control_body
        if comp.latency.is_some() {
            assert!(
                matches!(&*comp.control.borrow(), &ir::Control::Static(_)),
                "static component {} does not have static control. This should have been checked in ast_to_ir",
                comp.name
            );
        }

        // Checking that @interval annotations are placed correctly.
        // There are two options for @interval annotations:
        // 1. You have written only continuous assignments (this is similar
        // to primitives written in Verilog).
        // 2. You are using static<n> control.
        let comp_sig = &comp.signature.borrow();
        let go_ports =
            comp_sig.find_all_with_attr(ir::NumAttr::Go).collect_vec();
        if go_ports.iter().any(|go_port| {
            go_port.borrow().attributes.has(ir::NumAttr::Interval)
        }) {
            match &*comp.control.borrow() {
                ir::Control::Static(_) | ir::Control::Empty(_) => (),
                _ => return Err(Error::malformed_structure(
                    format!("component {} has dynamic control but has @interval annotations", comp.name),
                    )
                    .with_pos(&comp.attributes)),
            };
            if !comp.control.borrow().is_empty() {
                // Getting "reference value" should be the same for all go ports and
                // the control.
                let reference_val = match go_ports[0]
                    .borrow()
                    .attributes
                    .get(ir::NumAttr::Interval)
                {
                    Some(val) => val,
                    None => {
                        return Err(Error::malformed_structure(
                        "@interval(n) attribute on all @go ports since there is static<n> control",
                        )
                        .with_pos(&comp.attributes))
                    }
                };
                // Checking go ports.
                for go_port in &go_ports {
                    let go_port_val = match go_port
                        .borrow()
                        .attributes
                        .get(ir::NumAttr::Interval)
                    {
                        Some(val) => val,
                        None => {
                            return Err(Error::malformed_structure(format!(
                                "@go port expected @interval({reference_val}) attribute on all ports \
                                since the component has static<n> control",
                            ))
                            .with_pos(&comp.attributes))
                        }
                    };
                    if go_port_val != reference_val {
                        return Err(Error::malformed_structure(format!(
                            "@go port expected @interval {reference_val}, got @interval {go_port_val}",
                        ))
                        .with_pos(&go_port.borrow().attributes));
                    }
                    // Checking control latency
                    match comp.control.borrow().get_latency() {
                        None => {
                            unreachable!("already checked control is static")
                        }
                        Some(control_latency) => {
                            if control_latency != reference_val {
                                return Err(Error::malformed_structure(format!(
                                    "component {} expected @interval {reference_val}, got @interval {control_latency}", comp.name,
                                ))
                                .with_pos(&comp.attributes));
                            }
                        }
                    }
                }
            }
        }

        // For each non-combinational group, check if there is at least one write to the done
        // signal of that group and that the write is to the group's done signal.
        for gr in comp.get_groups().iter() {
            let group = gr.borrow();
            let gname = group.name();
            let mut has_done = false;
            // Find an assignment writing to this group's done condition.
            for assign in &group.assignments {
                let dst = assign.dst.borrow();
                if port_is_static_prim(&dst) {
                    return Err(Error::malformed_structure(format!(
                        "Static cell `{}` written to in non-static group",
                        dst.get_parent_name()
                    ))
                    .with_pos(&assign.attributes));
                }
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
                    if gname != dst.get_parent_name() {
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

        // Don't need to check done condition for static groups. Instead, just
        // checking that the static timing intervals are well formed, and
        // that don't write to static components
        for gr in comp.get_static_groups().iter() {
            let group = gr.borrow();
            let group_latency = group.get_latency();
            // Check that for each interval %[beg, end], end > beg.
            for assign in &group.assignments {
                assign.guard.check_for_each_info(
                    &mut |static_timing: &StaticTiming| {
                        if static_timing.get_interval().0
                            >= static_timing.get_interval().1
                        {
                            Err(Error::malformed_structure(format!(
                                "Static Timing Guard has improper interval: `{}`",
                                static_timing.to_string()
                            ))
                            .with_pos(&assign.attributes))
                        } else if static_timing.get_interval().1 > group_latency {
                            Err(Error::malformed_structure(format!(
                                "Static Timing Guard has interval `{}`, which is out of bounds since its static group has latency {}",
                                static_timing.to_string(),
                                group_latency
                            ))
                            .with_pos(&assign.attributes))
                        } else {
                            Ok(())
                        }
                    },
                )?;
            }
        }

        // Check for obvious conflicting assignments in the continuous assignments
        obvious_conflicts(
            comp.continuous_assignments.iter(),
            std::iter::empty::<&ir::Assignment<StaticTiming>>(),
        )?;
        // Check for obvious conflicting assignments between the continuous assignments and the groups
        for cgr in comp.comb_groups.iter() {
            for assign in &cgr.borrow().assignments {
                let dst = assign.dst.borrow();
                if port_is_static_prim(&dst) {
                    return Err(Error::malformed_structure(format!(
                        "Static cell `{}` written to in non-static group",
                        dst.get_parent_name()
                    ))
                    .with_pos(&assign.attributes));
                }
            }
            obvious_conflicts(
                cgr.borrow()
                    .assignments
                    .iter()
                    .chain(comp.continuous_assignments.iter()),
                std::iter::empty::<&ir::Assignment<StaticTiming>>(),
            )?;
        }

        Ok(Action::Continue)
    }

    fn static_enable(
        &mut self,
        s: &mut ir::StaticEnable,
        comp: &mut Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        self.used_groups.insert(s.group.borrow().name());

        let group = s.group.borrow();

        // check for obvious conflicts within static groups and continuous/comb group assigns
        obvious_conflicts(
            comp.continuous_assignments
                .iter()
                .chain(self.active_comb.iter()),
            group.assignments.iter(),
        )
        .map_err(|err| {
            let msg = s
                .attributes
                .copy_span()
                .into_option()
                .map(|s| s.format("Assigments activated by group enable"));
            err.with_post_msg(msg)
        })?;

        Ok(Action::Continue)
    }

    fn enable(
        &mut self,
        s: &mut ir::Enable,
        comp: &mut Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        self.used_groups.insert(s.group.borrow().name());

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
            .get(ir::NumAttr::Promotable)
            .map(|v| v == 0)
            .unwrap_or(false)
        {
            return Err(Error::malformed_structure("Group with annotation \"promotable\"=0 is invalid. Use `comb group` instead to define a combinational group or if the group's done condition is not constant, provide the correct \"static\" annotation.").with_pos(&group.attributes));
        }

        // Check if the group has obviously conflicting assignments with the continuous assignments and the active combinational groups
        obvious_conflicts(
            group
                .assignments
                .iter()
                .chain(comp.continuous_assignments.iter())
                .chain(self.active_comb.iter()),
            std::iter::empty::<&ir::Assignment<StaticTiming>>(),
        )
        .map_err(|err| {
            let msg = s
                .attributes
                .copy_span()
                .into_option()
                .map(|s| s.format("Assigments activated by group enable"));
            err.with_post_msg(msg)
        })?;

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
            self.used_comb_groups.insert(c.borrow().name());
        }
        // Only refers to ports defined in the invoked instance.
        let cell = s.comp.borrow();

        if let CellType::Component { name: id } = &cell.prototype {
            let cellmap = &self.ref_cell_types[id];
            let mut mentioned_cells = HashSet::new();
            for (outcell, incell) in s.ref_cells.iter() {
                if let Some(t) = cellmap.get(outcell) {
                    let proto = incell.borrow().prototype.clone();
                    same_type(t, &proto)
                        .map_err(|err| err.with_pos(&s.attributes))?;
                    mentioned_cells.insert(outcell);
                } else {
                    return Err(Error::malformed_control(format!(
                        "{} does not have ref cell named {}",
                        id, outcell
                    )));
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

    fn static_invoke(
        &mut self,
        s: &mut ir::StaticInvoke,
        _comp: &mut Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Only refers to ports defined in the invoked instance.
        let cell = s.comp.borrow();

        if let CellType::Component { name: id } = &cell.prototype {
            let cellmap = &self.ref_cell_types[id];
            let mut mentioned_cells = HashSet::new();
            for (outcell, incell) in s.ref_cells.iter() {
                if let Some(t) = cellmap.get(outcell) {
                    let proto = incell.borrow().prototype.clone();
                    same_type(t, &proto)
                        .map_err(|err| err.with_pos(&s.attributes))?;
                    mentioned_cells.insert(outcell);
                } else {
                    return Err(Error::malformed_control(format!(
                        "{} does not have ref cell named {}",
                        id, outcell
                    )));
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

    fn start_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if let Some(cgr) = &s.cond {
            let cg = cgr.borrow();
            let assigns = &cg.assignments;
            // Check if the combinational group conflicts with the active combinational groups
            obvious_conflicts(
                assigns.iter().chain(self.active_comb.iter()),
                std::iter::empty::<&ir::Assignment<StaticTiming>>(),
            )
            .map_err(|err| {
                let msg = s.attributes.copy_span().format(format!(
                    "Assignments from `{}' are actived here",
                    cg.name()
                ));
                err.with_post_msg(Some(msg))
            })?;
            // Push the combinational group to the stack of active groups
            self.active_comb.push(assigns);
        } else if !s.port.borrow().has_attribute(ir::BoolAttr::Stable) {
            let msg = s.attributes.copy_span().format(format!(
                    "If statement has no comb group and its condition port {} is unstable",
                    s.port.borrow().canonical()
                ));
            log::warn!("{msg}");
        }
        Ok(Action::Continue)
    }

    fn start_static_if(
        &mut self,
        s: &mut ir::StaticIf,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if !s.port.borrow().has_attribute(ir::BoolAttr::Stable) {
            let msg = s.attributes.copy_span().format(format!(
                "static if statement's condition port {} is unstable",
                s.port.borrow().canonical()
            ));
            log::warn!("{msg}");
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
            self.used_comb_groups.insert(cond.borrow().name());
            // Remove assignments from this combinational group
            self.active_comb.pop();
        }
        Ok(Action::Continue)
    }

    fn start_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if let Some(cgr) = &s.cond {
            let cg = cgr.borrow();
            let assigns = &cg.assignments;
            // Check if the combinational group conflicts with the active combinational groups
            obvious_conflicts(
                assigns.iter().chain(self.active_comb.iter()),
                std::iter::empty::<&ir::Assignment<StaticTiming>>(),
            )
            .map_err(|err| {
                let msg = s.attributes.copy_span().format(format!(
                    "Assignments from `{}' are actived here",
                    cg.name()
                ));
                err.with_post_msg(Some(msg))
            })?;
            // Push the combinational group to the stack of active groups
            self.active_comb.push(assigns);
        } else if !s.port.borrow().has_attribute(ir::BoolAttr::Stable) {
            let msg = s.attributes.copy_span().format(format!(
                    "While loop has no comb group and its condition port {} is unstable",
                    s.port.borrow().canonical()
                ));
            log::warn!("{msg}");
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
            self.used_comb_groups.insert(cond.borrow().name());
            // Remove assignments from this combinational group
            self.active_comb.pop();
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
        comp.for_each_static_assignment(|assign| {
            assign.for_each_port(|pr| {
                let port = pr.borrow();
                if port.is_hole() && port.name == "go" {
                    self.used_groups.insert(port.get_parent_name());
                }
                None
            })
        });

        // Find unused groups
        let mut all_groups: HashSet<ir::Id> = comp
            .get_groups()
            .iter()
            .map(|g| g.borrow().name())
            .collect();
        let static_groups: HashSet<ir::Id> = comp
            .get_static_groups()
            .iter()
            .map(|g| g.borrow().name())
            .collect();
        all_groups.extend(static_groups);

        if let Some(group) = all_groups.difference(&self.used_groups).next() {
            match comp.find_group(*group) {
                Some(gr) => {
                    let gr = gr.borrow();
                    return Err(
                        Error::unused(*group, "group").with_pos(&gr.attributes)
                    );
                }
                None => {
                    let gr = comp.find_static_group(*group).unwrap();
                    let gr = gr.borrow();
                    return Err(
                        Error::unused(*group, "group").with_pos(&gr.attributes)
                    );
                }
            }
        };

        let all_comb_groups: HashSet<ir::Id> =
            comp.comb_groups.iter().map(|g| g.borrow().name()).collect();
        if let Some(comb_group) =
            all_comb_groups.difference(&self.used_comb_groups).next()
        {
            let cgr = comp.find_comb_group(*comb_group).unwrap();
            let cgr = cgr.borrow();
            return Err(Error::unused(*comb_group, "combinational group")
                .with_pos(&cgr.attributes));
        }
        Ok(Action::Continue)
    }
}
