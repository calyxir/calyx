use core::panic;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ops::Add,
};

use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, VisResult, Visitor,
};
use calyx_ir::{self as ir, BoolAttr, Guard, Id, Nothing, NumAttr};
use calyx_utils::{CalyxResult, OutputFile};
use serde::Serialize;

#[derive(PartialEq, Eq, Hash, Clone, Serialize)]
struct StatsEntry {
    group_probe: u32,
    structural_enable_probe: u32,
    cell_probe: u32,
    primitive_probe: u32,
}

impl Add for StatsEntry {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            group_probe: self.group_probe + other.group_probe,
            structural_enable_probe: self.structural_enable_probe
                + other.structural_enable_probe,
            cell_probe: self.cell_probe + other.cell_probe,
            primitive_probe: self.primitive_probe + other.primitive_probe,
        }
    }
}

/// Adds probe wires to each group (includes static groups and comb groups) to detect when a group is active.
/// Used by the profiler.
pub struct ProfilerInstrumentation {
    probe_stats: BTreeMap<String, StatsEntry>,
    emit_probe_stats: Option<OutputFile>,
    invoke_comb_groups_to_stats: HashMap<Id, Option<StatsEntry>>,
}

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
        vec![PassOpt::new(
            "emit-probe-stats",
            "emit json file of shared cells",
            ParseVal::OutStream(OutputFile::Null),
            PassOpt::parse_outstream,
        )]
    }
}

impl ConstructVisitor for ProfilerInstrumentation {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        let opts = Self::get_opts(ctx);

        Ok(ProfilerInstrumentation {
            probe_stats: BTreeMap::new(),
            emit_probe_stats: opts["emit-probe-stats"].not_null_outstream(),
            invoke_comb_groups_to_stats: HashMap::new(),
        })
    }

    fn clear_data(&mut self) {}
}

fn count_helper<T>(map_opt: Option<CallsFromGroupMap<T>>) -> u32 {
    match map_opt {
        Some(map) => map
            .values()
            .fold(0, |acc, vec_ref| acc + vec_ref.len() as u32),
        None => 0,
    }
}

fn count<T>(
    num_groups: u32,
    structural_enable_map_opt: Option<CallsFromGroupMap<T>>,
    cell_invoke_map_opt: Option<CallsFromGroupMap<T>>,
    primitive_map_opt: Option<CallsFromGroupMap<T>>,
) -> StatsEntry {
    let num_structural_enables = count_helper(structural_enable_map_opt);
    let num_cell_invokes = count_helper(cell_invoke_map_opt);
    let num_primitive_invokes = count_helper(primitive_map_opt);

    StatsEntry {
        group_probe: num_groups,
        structural_enable_probe: num_structural_enables,
        cell_probe: num_cell_invokes,
        primitive_probe: num_primitive_invokes,
    }
}

/// Creates probe cells and assignments pertaining to standard groups.
fn group(
    comp: &mut ir::Component,
    sigs: &ir::LibrarySignatures,
    collect_stats: bool,
) -> Option<StatsEntry> {
    // groups to groups that they enabled
    let mut structural_enable_map: CallsFromGroupMap<Nothing> = HashMap::new();
    // groups to cells (from non-primitive components) that they invoked
    let mut cell_invoke_map: CallsFromGroupMap<Nothing> = HashMap::new();
    // groups to primitives that they invoked
    let mut primitive_invoke_map: CallsFromGroupMap<Nothing> = HashMap::new();
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
                && dst_borrow.name == "go"
            {
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
                            let guard = *(assignment_ref.guard.clone());
                            match cell_invoke_map.get_mut(&group.name()) {
                                Some(vec_ref) => {
                                    vec_ref.push((cell_name, guard));
                                }
                                None => {
                                    cell_invoke_map.insert(
                                        group.name(),
                                        vec![(cell_name, guard)],
                                    );
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
    let group_name_assign_and_cell = create_probes_and_assignments(
        comp,
        sigs,
        &group_names,
        Some(&structural_enable_map),
        Some(&cell_invoke_map),
        Some(&primitive_invoke_map),
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

    if collect_stats {
        Some(count(
            group_names.len() as u32,
            Some(structural_enable_map),
            Some(cell_invoke_map),
            Some(primitive_invoke_map),
        ))
    } else {
        None
    }
}

/// Creates probe cells and assignments pertaining to combinational groups.
/// `covered` is the set of comb groups that were attached to invokes
/// and already got instrumentation probes covered, so should be ignored
/// by this function.
fn combinational_group(
    comp: &mut ir::Component,
    sigs: &ir::LibrarySignatures,
    collect_stats: bool,
    covered: &HashSet<Id>,
) -> Option<StatsEntry> {
    // NOTE: combinational groups cannot structurally enable other groups

    // groups to cells (from non-primitive components) that they invoked
    let mut cell_invoke_map: CallsFromGroupMap<Nothing> = HashMap::new();
    // groups to primitives that they invoked
    let mut primitive_invoke_map: CallsFromGroupMap<Nothing> = HashMap::new();

    let group_names = comp
        .comb_groups
        .iter()
        // filter out any comb groups that are in covered
        .filter(|group| !covered.contains(&group.borrow().name()))
        .map(|group| group.borrow().name())
        .collect::<Vec<_>>();

    for group_ref in comp
        .comb_groups
        .iter()
        .filter(|group| !covered.contains(&group.borrow().name()))
    {
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
                            let guard = *(assignment_ref.guard.clone());
                            match cell_invoke_map.get_mut(&group.name()) {
                                Some(vec_ref) => {
                                    vec_ref.push((cell_name, guard));
                                }
                                None => {
                                    cell_invoke_map.insert(
                                        group.name(),
                                        vec![(cell_name, guard)],
                                    );
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    let group_name_asgn_and_cell = create_probes_and_assignments(
        comp,
        sigs,
        &group_names,
        None, // assuming no structural enables within comb groups
        Some(&cell_invoke_map),
        Some(&primitive_invoke_map),
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

    if collect_stats {
        Some(count(
            group_names.len() as u32,
            None,
            Some(cell_invoke_map),
            Some(primitive_invoke_map),
        ))
    } else {
        None
    }
}

/// Creates probe cells and assignments pertaining to static groups.
fn static_group(
    comp: &mut ir::Component,
    sigs: &ir::LibrarySignatures,
    collect_stats: bool,
) -> Option<StatsEntry> {
    let group_names = comp
        .static_groups
        .iter()
        .map(|group| group.borrow().name())
        .collect::<Vec<_>>();

    // groups to groups that they enabled
    let mut structural_enable_map: CallsFromGroupMap<ir::StaticTiming> =
        HashMap::new();
    // groups to cells (from non-primitive components) that they invoked
    let mut cell_invoke_map: CallsFromGroupMap<ir::StaticTiming> =
        HashMap::new();
    // groups to primitives that they invoked
    let mut primitive_invoke_map: CallsFromGroupMap<ir::StaticTiming> =
        HashMap::new();

    for group_ref in comp.static_groups.iter() {
        let group = &group_ref.borrow();
        // set to prevent adding multiple probes for a combinational primitive enabled by the group
        let mut comb_primitives_covered = HashSet::new();
        let mut primitive_vec: Vec<(Id, ir::Guard<ir::StaticTiming>)> =
            Vec::new();
        for assignment_ref in group.assignments.iter() {
            let dst_borrow = assignment_ref.dst.borrow();
            if let ir::PortParent::Group(parent_group_ref) = &dst_borrow.parent
                && dst_borrow.name == "go"
            {
                // found an invocation of go
                let invoked_group_name =
                    parent_group_ref.upgrade().borrow().name();
                let guard = *(assignment_ref.guard).clone();
                structural_enable_map
                    .entry(invoked_group_name)
                    .or_default()
                    .push((group.name(), guard));
            }
            if let ir::PortParent::Cell(cell_ref) = &dst_borrow.parent {
                match cell_ref.upgrade().borrow().prototype.clone() {
                    calyx_ir::CellType::Primitive { is_comb, .. } => {
                        let cell_name = cell_ref.upgrade().borrow().name();
                        if is_comb {
                            // collecting primitives for area utilization; we want to avoid adding the same primitive twice!
                            if comb_primitives_covered.insert(cell_name) {
                                primitive_vec.push((cell_name, Guard::True));
                            }
                        } else if dst_borrow.has_attribute(NumAttr::Go) {
                            // non-combinational primitives
                            let guard = Guard::and(
                                *(assignment_ref.guard).clone(),
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
                            let guard = *(assignment_ref.guard.clone());
                            cell_invoke_map
                                .entry(group.name())
                                .or_default()
                                .push((cell_name, guard));
                        }
                    }
                    _ => (),
                }
            }
        }
        primitive_invoke_map.insert(group_ref.borrow().name(), primitive_vec);
    }

    let group_name_assign_and_cell = create_probes_and_assignments(
        comp,
        sigs,
        &group_names,
        Some(&structural_enable_map),
        Some(&cell_invoke_map),
        Some(&primitive_invoke_map),
    );

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

    if collect_stats {
        Some(count(
            group_names.len() as u32,
            Some(structural_enable_map),
            Some(cell_invoke_map),
            Some(primitive_invoke_map),
        ))
    } else {
        None
    }
}

/// Creates all probe cells and assignments for a certain kind of .
/// Returns a Vec where each element is (GROUP, ASGN, CELL) where
/// GROUP is the group to write the assignment in,
/// ASGN is the probe assignment to insert into the group,
/// CELL is the generated probe wire to add to cells
fn create_probes_and_assignments<T: Clone>(
    comp: &mut ir::Component,
    sigs: &ir::LibrarySignatures,
    group_names: &[Id],
    structural_enable_map_opt: Option<&CallsFromGroupMap<T>>,
    cell_invoke_map_opt: Option<&CallsFromGroupMap<T>>,
    primitive_invoke_map_opt: Option<&CallsFromGroupMap<T>>,
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
        let name = format!("{group_name}{delimiter}{comp_name}_group_probe");
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
                    "{invoked_group_name}{delimiter}{parent_group}{delimiter}{comp_name}_se_probe"
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
            for (invoked_cell, guard) in invoked_cells {
                let probe_cell_name = format!(
                    "{invoked_cell}{delimiter}{invoker_group}{delimiter}{comp_name}_cell_probe"
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
                    guard.clone(),
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
                    "{primitive_cell_name}{delimiter}{group}{delimiter}{comp_name}_primitive_probe"
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

/// Creates probes for continuous assignments outside of groups. For every cell
/// or primitive involved in a continuous assignment, this function will generate
/// "contprimitive" and "contcell" wires as probes.
fn continuous_assignments(
    comp: &mut ir::Component,
    sigs: &ir::LibrarySignatures,
    collect_stats: bool,
) -> Option<StatsEntry> {
    // vector of cells (non-primitives) invoked
    let mut cell_invoke_vec: Vec<(Id, ir::Guard<Nothing>)> = Vec::new();
    // vector of primitives invoked
    let mut primitive_invoke_vec: Vec<(Id, ir::Guard<Nothing>)> = Vec::new();

    // set to prevent adding multiple probes for a combinational primitive
    let mut comb_primitives_covered = HashSet::new();
    let mut comb_cells_covered = HashSet::new();
    for assignment_ref in comp.continuous_assignments.iter() {
        let dst_borrow = assignment_ref.dst.borrow();
        let guard = *(assignment_ref.guard).clone();
        if let ir::PortParent::Cell(cell_ref) = &dst_borrow.parent {
            match cell_ref.upgrade().borrow().prototype.clone() {
                calyx_ir::CellType::Primitive { .. } => {
                    let cell_name = cell_ref.upgrade().borrow().name();
                    // collecting primitives for area utilization; we want to avoid adding the same primitive twice!
                    if comb_primitives_covered.insert(cell_name) {
                        primitive_invoke_vec.push((cell_name, guard));
                    }
                }
                calyx_ir::CellType::Component { .. } => {
                    let cell_name = cell_ref.upgrade().borrow().name();
                    if comb_cells_covered.insert(cell_name) {
                        cell_invoke_vec.push((cell_name, guard));
                    }
                }
                _ => (),
            }
        }
    }

    // add probes for primitives in continuous assignment
    let delimiter = "___";
    let comp_name = comp.name;
    let mut builder = ir::Builder::new(comp, sigs);
    let one = builder.add_constant(1, 1);
    let mut assign_and_cell = Vec::new();
    for (primitive_cell_name, guard) in primitive_invoke_vec.iter() {
        let probe_cell_name = format!(
            "{primitive_cell_name}{delimiter}{comp_name}_contprimitive_probe"
        );
        let probe_cell =
            builder.add_primitive(probe_cell_name, "std_wire", &[1]);
        probe_cell.borrow_mut().add_attribute(BoolAttr::Control, 1);
        probe_cell
            .borrow_mut()
            .add_attribute(BoolAttr::Protected, 1);
        let probe_asgn: ir::Assignment<Nothing> = builder.build_assignment(
            probe_cell.borrow().get("in"),
            one.borrow().get("out"),
            guard.clone(),
        );
        assign_and_cell.push((probe_asgn, probe_cell));
    }
    // add probes for cells (non-primitives) in continuous assignment
    for (cell_name, guard) in cell_invoke_vec.iter() {
        let probe_cell_name =
            format!("{cell_name}{delimiter}{comp_name}_contcell_probe");
        let probe_cell =
            builder.add_primitive(probe_cell_name, "std_wire", &[1]);
        probe_cell.borrow_mut().add_attribute(BoolAttr::Control, 1);
        probe_cell
            .borrow_mut()
            .add_attribute(BoolAttr::Protected, 1);
        let probe_asgn: ir::Assignment<Nothing> = builder.build_assignment(
            probe_cell.borrow().get("in"),
            one.borrow().get("out"),
            guard.clone(),
        );
        assign_and_cell.push((probe_asgn, probe_cell));
    }

    // Add created assignments to continuous assignments
    for (asgn, cell) in assign_and_cell.iter() {
        comp.continuous_assignments.push(asgn.clone());
        comp.cells.add(cell.to_owned());
    }

    if collect_stats {
        Some(StatsEntry {
            group_probe: 0,
            structural_enable_probe: 0,
            cell_probe: cell_invoke_vec.len() as u32,
            primitive_probe: primitive_invoke_vec.len() as u32,
        })
    } else {
        None
    }
}

fn populate_stats(
    component_name: Id,
    stats_map: &mut BTreeMap<String, StatsEntry>,
    stats_list: Vec<Option<StatsEntry>>,
) {
    let this_comp_stats_list = stats_list.iter().fold(
        StatsEntry {
            group_probe: 0,
            structural_enable_probe: 0,
            cell_probe: 0,
            primitive_probe: 0,
        },
        |s, g_s_opt| match g_s_opt {
            Some(g_s) => s + g_s.clone(),
            None => s,
        },
    );
    stats_map.insert(component_name.to_string(), this_comp_stats_list);
}

impl Visitor for ProfilerInstrumentation {
    fn invoke(
        &mut self,
        s: &mut calyx_ir::Invoke,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        let cell_name = s.comp.borrow().name();
        // for invokes, we instrument the comb group
        let mut comb_group = match &s.comb_group {
            Some(s) => s.borrow_mut(),
            None => {
                panic!(
                    "Invokes should come with a comb group. Please run `uniquefy_enables` before running this pass!"
                )
            }
        };
        let comb_group_name = comb_group.name();

        // To avoid code cloning, we will reuse create_probes_and_assignments by passing in
        // one-key maps (where the key is the name of the comb group) for cell_invoke_map_opt and primitive_invoke_map_opt
        let mut cell_invoke_map: CallsFromGroupMap<Nothing> = HashMap::new();
        cell_invoke_map.insert(comb_group_name, vec![(cell_name, Guard::True)]);

        // scanning to see if there are primitive uses (this can happen if the comb group was user defined)
        let mut primitive_name_set = HashSet::new();
        let mut primitives_invoked_vec = vec![];
        for assignment_ref in comb_group.assignments.iter() {
            let dst_borrow = assignment_ref.dst.borrow();
            if let ir::PortParent::Cell(cell_ref) = &dst_borrow.parent
                && let calyx_ir::CellType::Primitive { name, .. } =
                    cell_ref.upgrade().borrow().prototype.clone()
            {
                if primitive_name_set.insert(name) {
                    primitives_invoked_vec
                        .push((name, *(assignment_ref.guard.clone())));
                }
            }
        }
        let mut primitive_invoke_map: CallsFromGroupMap<Nothing> =
            HashMap::new();
        primitive_invoke_map.insert(comb_group_name, primitives_invoked_vec);

        let group_name_asgn_and_cell = create_probes_and_assignments(
            comp,
            sigs,
            &[comb_group_name],
            None,
            Some(&cell_invoke_map),
            Some(&primitive_invoke_map),
        );

        // insert created assignments back into comb group
        for (_comb_group_name, asgn, cell) in group_name_asgn_and_cell {
            comb_group.assignments.push(asgn.clone());
            comp.cells.add(cell.to_owned());
        }

        // collect statistics
        let stats = if self.emit_probe_stats.is_some() {
            Some(count(
                1,
                None,
                Some(cell_invoke_map),
                Some(primitive_invoke_map),
            ))
        } else {
            None
        };
        self.invoke_comb_groups_to_stats
            .insert(comb_group_name, stats);

        Ok(Action::Continue)
    }

    fn static_invoke(
        &mut self,
        s: &mut calyx_ir::StaticInvoke,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        let cell_name = s.comp.borrow().name();
        // for invokes, we instrument the comb group
        let mut comb_group = match &s.comb_group {
            Some(s) => s.borrow_mut(),
            None => {
                panic!(
                    "Invokes should come with a comb group. Please run `uniquefy_enables` before running this pass!"
                )
            }
        };
        let comb_group_name = comb_group.name();

        // To avoid code cloning, we will reuse create_probes_and_assignments by passing in
        // one-key maps (where the key is the name of the comb group) for cell_invoke_map_opt and primitive_invoke_map_opt
        let mut cell_invoke_map: CallsFromGroupMap<Nothing> = HashMap::new();
        cell_invoke_map.insert(comb_group_name, vec![(cell_name, Guard::True)]);

        // scanning to see if there are primitive uses (this can happen if the comb group was user defined)
        let mut primitive_name_set = HashSet::new();
        let mut primitives_invoked_vec = vec![];
        for assignment_ref in comb_group.assignments.iter() {
            let dst_borrow = assignment_ref.dst.borrow();
            if let ir::PortParent::Cell(cell_ref) = &dst_borrow.parent
                && let calyx_ir::CellType::Primitive { name, .. } =
                    cell_ref.upgrade().borrow().prototype.clone()
            {
                if primitive_name_set.insert(name) {
                    primitives_invoked_vec
                        .push((name, *(assignment_ref.guard.clone())));
                }
            }
        }
        let mut primitive_invoke_map: CallsFromGroupMap<Nothing> =
            HashMap::new();
        primitive_invoke_map.insert(comb_group_name, primitives_invoked_vec);

        let group_name_asgn_and_cell = create_probes_and_assignments(
            comp,
            sigs,
            &[comb_group_name],
            None,
            Some(&cell_invoke_map),
            Some(&primitive_invoke_map),
        );

        // insert created assignments back into comb group
        for (_comb_group_name, asgn, cell) in group_name_asgn_and_cell {
            comb_group.assignments.push(asgn.clone());
            comp.cells.add(cell.to_owned());
        }

        // collect statistics
        let stats = if self.emit_probe_stats.is_some() {
            Some(count(
                1,
                None,
                Some(cell_invoke_map),
                Some(primitive_invoke_map),
            ))
        } else {
            None
        };
        self.invoke_comb_groups_to_stats
            .insert(comb_group_name, stats);

        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut calyx_ir::Component,
        sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        let count = self.emit_probe_stats.is_some();
        let group_stats_opt = group(comp, sigs, count);
        let comb_group_stats_opt = combinational_group(
            comp,
            sigs,
            count,
            &self.invoke_comb_groups_to_stats.keys().cloned().collect(),
        );
        let static_group_stats_opt = static_group(comp, sigs, count);
        let continuous_assignments_opt =
            continuous_assignments(comp, sigs, count);

        if count {
            populate_stats(
                comp.name,
                &mut self.probe_stats,
                vec![
                    group_stats_opt,
                    comb_group_stats_opt,
                    static_group_stats_opt,
                    continuous_assignments_opt,
                ],
            )
        }
        Ok(Action::Continue)
    }

    fn finish_context(&mut self, _ctx: &mut calyx_ir::Context) -> VisResult {
        if let Some(json_out_file) = &mut self.emit_probe_stats {
            let _ = serde_json::to_writer_pretty(
                json_out_file.get_write(),
                &self.probe_stats,
            );
        }
        Ok(Action::Stop)
    }
}
