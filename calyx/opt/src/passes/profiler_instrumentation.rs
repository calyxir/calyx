use core::panic;
use std::collections::{HashMap, HashSet};

use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{self as ir, BoolAttr, Guard, Id, Nothing, NumAttr};
use calyx_utils::CalyxResult;

/// Adds probe wires to each group (includes static groups and comb groups) to detect when a group is active.
/// Used by the profiler.
pub struct ProfilerInstrumentation {}

/// Mapping group names to constructs (groups/primitives/cells) that the group enabled,
/// along with the guard that was involved in the assignment.
type CallsFromGroupMap<T> = HashMap<Id, Vec<(Id, ir::Guard<T>)>>;

impl Named for ProfilerInstrumentation {
    fn name() -> &'static str {
        "profiler-instrumentation"
    }

    fn description() -> &'static str {
        "Add instrumentation for profiling"
    }

    fn opts() -> Vec<crate::traversal::PassOpt> {
        vec![]
    }
}

impl ConstructVisitor for ProfilerInstrumentation {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        Ok(ProfilerInstrumentation {})
    }

    fn clear_data(&mut self) {}
}

/// Creates probe cells and assignments pertaining to standard groups.
fn group(comp: &mut ir::Component, sigs: &ir::LibrarySignatures) {
    // groups to groups that they enabled
    let mut structural_enable_map: HashMap<Id, Vec<(Id, ir::Guard<Nothing>)>> =
        HashMap::new();
    // groups to cells (from non-primitive components) that they invoked
    let mut cell_invoke_map: HashMap<Id, Vec<Id>> = HashMap::new();
    // groups to primitives that they invoked
    let mut primitive_invoke_map: HashMap<Id, Vec<(Id, ir::Guard<Nothing>)>> =
        HashMap::new();
    let group_names = comp
        .groups
        .iter()
        .map(|group| group.borrow().name())
        .collect::<Vec<_>>();

    // Dynamic groups: iterate and check for structural enables, cell invokes, and primitive enables
    for group_ref in comp.groups.iter() {
        let group = &group_ref.borrow();
        // set to prevent adding multiple probes for a combinational primitive enabled by the group
        let mut comb_primitives_covered = HashSet::new();
        let mut primitive_vec: Vec<(Id, ir::Guard<Nothing>)> = Vec::new();
        for assignment_ref in group.assignments.iter() {
            let dst_borrow = assignment_ref.dst.borrow();
            if let ir::PortParent::Group(parent_group_ref) = &dst_borrow.parent
            {
                if dst_borrow.name == "go" {
                    // found an invocation of go
                    let invoked_group_name =
                        parent_group_ref.upgrade().borrow().name();
                    let guard = *(assignment_ref.guard.clone());
                    match structural_enable_map.get_mut(&invoked_group_name) {
                        Some(vec_ref) => vec_ref.push((group.name(), guard)),
                        None => {
                            structural_enable_map.insert(
                                invoked_group_name,
                                vec![(group.name(), guard)],
                            );
                        }
                    }
                }
            }
            if let ir::PortParent::Cell(cell_ref) = &dst_borrow.parent {
                match cell_ref.upgrade().borrow().prototype.clone() {
                    calyx_ir::CellType::Primitive {
                        name: _,
                        param_binding: _,
                        is_comb,
                        latency: _,
                    } => {
                        let cell_name = cell_ref.upgrade().borrow().name();
                        if is_comb {
                            // collecting primitives for area utilization; we want to avoid adding the same primitive twice!
                            if comb_primitives_covered.insert(cell_name) {
                                primitive_vec.push((cell_name, Guard::True));
                            }
                        } else if dst_borrow.has_attribute(NumAttr::Go) {
                            // non-combinational primitives
                            let guard = Guard::and(
                                *(assignment_ref.guard.clone()),
                                Guard::port(ir::rrc(
                                    assignment_ref.src.borrow().clone(),
                                )),
                            );
                            primitive_vec.push((cell_name, guard));
                        }
                    }
                    calyx_ir::CellType::Component { name: _ } => {
                        if dst_borrow.has_attribute(NumAttr::Go) {
                            let cell_name = cell_ref.upgrade().borrow().name();
                            match cell_invoke_map.get_mut(&group.name()) {
                                Some(vec_ref) => {
                                    vec_ref.push(cell_name);
                                }
                                None => {
                                    cell_invoke_map
                                        .insert(group.name(), vec![cell_name]);
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
        primitive_invoke_map.insert(group_ref.borrow().name(), primitive_vec);
    }

    // create probe cells and assignments
    let group_name_assign_and_cell = create_assignments(
        comp,
        sigs,
        &group_names,
        Some(structural_enable_map),
        Some(cell_invoke_map),
        Some(primitive_invoke_map),
    );

    // Add created assignments to each group and their corresponding probe cells
    for group in comp.groups.iter() {
        for (group_name, asgn, cell) in group_name_assign_and_cell.iter() {
            if group.borrow().name() == group_name {
                group.borrow_mut().assignments.push(asgn.clone());
                comp.cells.add(cell.to_owned());
            }
        }
    }
}

/// Creates probe cells and assignments pertaining to combinational groups.
fn combinational_group(comp: &mut ir::Component, sigs: &ir::LibrarySignatures) {
    // NOTE: combinational groups cannot structurally enable other groups

    // groups to cells (from non-primitive components) that they invoked
    let mut cell_invoke_map: HashMap<Id, Vec<Id>> = HashMap::new();
    // groups to primitives that they invoked
    let mut primitive_invoke_map: HashMap<Id, Vec<(Id, ir::Guard<Nothing>)>> =
        HashMap::new();

    let group_names = comp
        .comb_groups
        .iter()
        .map(|group| group.borrow().name())
        .collect::<Vec<_>>();

    for group_ref in comp.comb_groups.iter() {
        let group = &group_ref.borrow();
        let mut comb_primitives_covered = HashSet::new();
        let mut comb_cells_covered = HashSet::new();

        for assignment_ref in group.assignments.iter() {
            let dst_borrow = assignment_ref.dst.borrow();
            if let ir::PortParent::Cell(cell_ref) = &dst_borrow.parent {
                match cell_ref.upgrade().borrow().prototype.clone() {
                    calyx_ir::CellType::Primitive {
                        name: _,
                        param_binding: _,
                        is_comb,
                        latency: _,
                    } => {
                        let cell_name = cell_ref.upgrade().borrow().name();
                        if is_comb {
                            // collecting primitives for area utilization; we want to avoid adding the same primitive twice!
                            if comb_primitives_covered.insert(cell_name) {
                                match primitive_invoke_map
                                    .get_mut(&group.name())
                                {
                                    Some(vec_ref) => {
                                        vec_ref.push((cell_name, Guard::True));
                                    }
                                    None => {
                                        primitive_invoke_map.insert(
                                            group.name(),
                                            vec![(cell_name, Guard::True)],
                                        );
                                    }
                                }
                            }
                        } else if dst_borrow.has_attribute(NumAttr::Go) {
                            panic!(
                                "Non-combinational primitive {} invoked inside of combinational group {}!",
                                dst_borrow.canonical(),
                                group.name()
                            )
                        }
                    }
                    calyx_ir::CellType::Component { name: _ } => {
                        let cell_name = cell_ref.upgrade().borrow().name();
                        if dst_borrow.name == "go" {
                            panic!(
                                "Non-combinational cell {} invoked inside of combinational group {}!",
                                cell_name,
                                group.name()
                            );
                        } else if comb_cells_covered.insert(cell_name) {
                            match cell_invoke_map.get_mut(&group.name()) {
                                Some(vec_ref) => {
                                    vec_ref.push(cell_name);
                                }
                                None => {
                                    cell_invoke_map
                                        .insert(group.name(), vec![cell_name]);
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    let group_name_asgn_and_cell = create_assignments(
        comp,
        sigs,
        &group_names,
        None, // assuming no structural enables within comb groups
        Some(cell_invoke_map),
        Some(primitive_invoke_map),
    );

    // Comb: Add created assignments to each group
    for comb_group in comp.comb_groups.iter() {
        for (comb_group_name, asgn, cell) in group_name_asgn_and_cell.iter() {
            if comb_group.borrow().name() == comb_group_name {
                comb_group.borrow_mut().assignments.push(asgn.clone());
                comp.cells.add(cell.to_owned());
            }
        }
    }
}

/// Creates probe cells and assignments pertaining to static groups.
fn static_group(comp: &mut ir::Component, sigs: &ir::LibrarySignatures) {
    let group_names = comp
        .static_groups
        .iter()
        .map(|group| group.borrow().name())
        .collect::<Vec<_>>();

    // TODO: create probes for structural enables, cell invokes, and primitive invokes

    let group_name_assign_and_cell =
        create_assignments(comp, sigs, &group_names, None, None, None);

    // Add created assignments to each group
    for static_group in comp.static_groups.iter() {
        for (static_group_name, asgn, cell) in group_name_assign_and_cell.iter()
        {
            if static_group.borrow().name() == static_group_name {
                static_group.borrow_mut().assignments.push(asgn.clone());
                comp.cells.add(cell.to_owned());
            }
        }
    }
}

/// Creates all probe cells and assignments for a certain kind of .
/// Returns a Vec where each element is (GROUP, ASGN, CELL) where
/// GROUP is the group to write the assignment in,
/// ASGN is the probe assignment to insert into the group,
/// CELL is the generated probe wire to add to cells
fn create_assignments<T: Clone>(
    comp: &mut ir::Component,
    sigs: &ir::LibrarySignatures,
    group_names: &[Id],
    structural_enable_map_opt: Option<CallsFromGroupMap<T>>,
    cell_invoke_map_opt: Option<HashMap<Id, Vec<Id>>>,
    primitive_invoke_map_opt: Option<CallsFromGroupMap<T>>,
) -> Vec<(
    Id,
    calyx_ir::Assignment<T>,
    std::rc::Rc<std::cell::RefCell<calyx_ir::Cell>>,
)> {
    let delimiter = "___";
    let comp_name = comp.name;
    // build probe and assignments for every group (dynamic and static) + all structural invokes
    let mut builder = ir::Builder::new(comp, sigs);
    let one = builder.add_constant(1, 1);

    // (group name, assignment to insert, probe cell to insert) for each probe we want to insert
    // we assume that each probe cell will only have one assignment.
    let mut group_name_assign_and_cell = Vec::new();

    // probe and assignments for group enable (this group is currently active)
    for group_name in group_names.iter() {
        // store group and component name (differentiate between groups of the same name under different components)
        let name =
            format!("{}{}{}_group_probe", group_name, delimiter, comp_name);
        let probe_cell = builder.add_primitive(name, "std_wire", &[1]);
        let probe_asgn: ir::Assignment<T> = builder.build_assignment(
            probe_cell.borrow().get("in"),
            one.borrow().get("out"),
            Guard::True,
        );
        // the probes should be @control because they should have value 0 whenever the corresponding group is not active.
        probe_cell.borrow_mut().add_attribute(BoolAttr::Control, 1);
        probe_cell
            .borrow_mut()
            .add_attribute(BoolAttr::Protected, 1);
        group_name_assign_and_cell.push((*group_name, probe_asgn, probe_cell));
    }

    if let Some(sem) = structural_enable_map_opt {
        // probe and assignments for structural enables (this group is structurally enabling a child group)
        for (invoked_group_name, parent_groups) in sem.iter() {
            for (parent_group, guard) in parent_groups.iter() {
                let probe_cell_name = format!(
                    "{}{}{}{}{}_se_probe",
                    invoked_group_name,
                    delimiter,
                    parent_group,
                    delimiter,
                    comp_name
                );
                let probe_cell =
                    builder.add_primitive(probe_cell_name, "std_wire", &[1]);
                probe_cell.borrow_mut().add_attribute(BoolAttr::Control, 1);
                probe_cell
                    .borrow_mut()
                    .add_attribute(BoolAttr::Protected, 1);
                let probe_asgn: ir::Assignment<T> = builder.build_assignment(
                    probe_cell.borrow().get("in"),
                    one.borrow().get("out"),
                    guard.clone(),
                );
                group_name_assign_and_cell.push((
                    *parent_group,
                    probe_asgn,
                    probe_cell,
                ));
            }
        }
    }

    if let Some(cell_invoke_map) = cell_invoke_map_opt {
        // probe cell and assignments for structural cell invocations (the group is structurally invoking a cell.)
        for (invoker_group, invoked_cells) in cell_invoke_map.iter() {
            for invoked_cell in invoked_cells {
                let probe_cell_name = format!(
                    "{}{}{}{}{}_cell_probe",
                    invoked_cell,
                    delimiter,
                    invoker_group,
                    delimiter,
                    comp_name
                );
                let probe_cell =
                    builder.add_primitive(probe_cell_name, "std_wire", &[1]);
                probe_cell.borrow_mut().add_attribute(BoolAttr::Control, 1);
                probe_cell
                    .borrow_mut()
                    .add_attribute(BoolAttr::Protected, 1);
                // NOTE: this probe is active for the duration of the whole group. Hence, it may be active even when the cell itself is inactive.
                let probe_asgn: ir::Assignment<T> = builder.build_assignment(
                    probe_cell.borrow().get("in"),
                    one.borrow().get("out"),
                    Guard::True,
                );
                group_name_assign_and_cell.push((
                    *invoker_group,
                    probe_asgn,
                    probe_cell,
                ));
            }
        }
    }

    if let Some(primitive_invoke_map) = primitive_invoke_map_opt {
        // probe and assignments for primitive invocations (this group is activating a primitive)
        for (group, primitive_invs) in primitive_invoke_map.iter() {
            for (primitive_cell_name, guard) in primitive_invs.iter() {
                let probe_cell_name = format!(
                    "{}{}{}{}{}_primitive_probe",
                    primitive_cell_name, delimiter, group, delimiter, comp_name
                );
                let probe_cell =
                    builder.add_primitive(probe_cell_name, "std_wire", &[1]);
                probe_cell.borrow_mut().add_attribute(BoolAttr::Control, 1);
                probe_cell
                    .borrow_mut()
                    .add_attribute(BoolAttr::Protected, 1);
                let probe_asgn: ir::Assignment<T> = builder.build_assignment(
                    probe_cell.borrow().get("in"),
                    one.borrow().get("out"),
                    guard.clone(),
                );
                group_name_assign_and_cell
                    .push((*group, probe_asgn, probe_cell));
            }
        }
    }

    group_name_assign_and_cell
}

impl Visitor for ProfilerInstrumentation {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        group(comp, sigs);
        combinational_group(comp, sigs);
        static_group(comp, sigs);
        Ok(Action::Continue)
    }
}
