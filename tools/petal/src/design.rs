use crate::adls::{ComponentInfo, PosInfo};
use crate::calyx_timeline::{CalyxTimeline, CurrentlyActive, NON_ID_THREAD};
use crate::control::{ControlInfo, ControlRegister, PathDescriptorInfo};
use crate::shared_cells::SharedCellsInfo;
use crate::visuals::perfetto_protos::track_event::Type;
use crate::visuals::timeline::Uuid;
use anyhow::{Context, Result, anyhow};
use baa::{BitVecOps, BitVecValue};
use core::panic;
use cranelift_entity::{PrimaryMap, entity_impl};
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::{SmallVec, smallvec};
use wellen::{Hierarchy, Scope, ScopeRef, SignalRef, VarRef};

const COMPILER_GENERATED_MSG: &str = "compiler-generated";

#[derive(Clone, Copy, Hash, PartialEq, Eq, Default, PartialOrd, Ord)]
pub struct CellId(u32);
entity_impl!(CellId, "cell");

#[derive(Clone, Debug)]
/// Represents a (component/primitive) cell in the static tree.
struct Cell {
    /// The user-defined name of the cell.
    name: String,
    /// Full path of the cell using component cell names (ex. toplevel.main.mac)
    full_path: String,
    /// Ids of control nodes that could be called directly from this cell, if it is a component.
    /// NOTE: Primitive cells should have an empty vec here.
    control: SmallVec<[ControlId; 6]>,
    /// Ids of groups that could be called directly from this cell, if it is a component.
    /// NOTE: Primitive cells should have an empty vec here.
    groups: SmallVec<[GroupId; 6]>,
    /// Control Registers
    control_registers: SmallVec<[RegisterId; 6]>,
    /// The scope of the cell in the RTL trace.
    _scope: Option<ScopeRef>,
    /// Is the cell a primitive?
    is_primitive: bool,
    /// If the cell is of the main component, contains a ref to main.go and main.done.
    probes: Option<(SignalRef, SignalRef)>,
    /// If the cell is of the main component, contains the bitvector index for main.go and main.done.
    probe_idxs: Option<(u32, u32)>,
    /// Non-empty if the cell is from a user-defined component.
    /// FIXME: might be worth pulling the primitive's original name as well?
    component: String,
    /// cells that are defined within the component
    instances: SmallVec<[CellId; 6]>,
    /// Name of the replacement cell, if the cell was a shared primitive.
    replacement: Option<String>,
    /// Information necessary to map back to the ADL.
    /// Will only be Some if we are dealing with an ADL that almost 1-1 maps to Calyx (Calyx-py)
    /// NOTE: In the future we might want to make a struct with only the necessary information
    adl_mapping: Option<PosInfo>,
    adl_component: Option<PosInfo>,
}

impl Cell {
    /// String representation of cell for trace and visualizations
    pub fn display_name(&self) -> String {
        if self.is_primitive {
            let s = format!("{} (primitive)", self.name);
            if let Some(r) = &self.replacement {
                format!("{s} -> {}", r)
            } else {
                s
            }
        } else if self.component == "main" {
            self.name.clone()
        } else {
            format!("{} [{}]", self.name, self.component)
        }
    }

    pub fn stats_name(&self) -> String {
        assert!(!self.is_primitive);
        format!("{} [{}]", self.full_path, self.component)
    }

    pub fn adl_display_name(&self) -> String {
        println!("{:?}", self.name);
        assert!((self.adl_mapping.is_some() || self.name == "main"));
        assert!(self.is_primitive || self.adl_component.is_some());

        if self.is_primitive {
            format!(
                "{} (primitive)",
                self.adl_mapping.clone().unwrap().adl_str()
            )
        } else if let Some(adl_component) = &self.adl_component {
            if self.component == "main" {
                adl_component.adl_str().to_string()
            } else if let Some(adl_mapping) = &self.adl_mapping {
                format!(
                    "{} [{}]",
                    adl_mapping.adl_str(),
                    adl_component.adl_str()
                )
            } else {
                panic!(
                    "Non-main Cell {} does not have a ADL mapping!",
                    self.name
                );
            }
        } else {
            panic!(
                "Cell {} does not have either a ADL mapping or ADL component mapping!",
                self.name
            )
        }
    }

    pub fn mixed_display_name(&self) -> String {
        assert!((self.adl_mapping.is_some() || self.name == "main"));
        assert!(self.is_primitive || self.adl_component.is_some());

        if let Some(adl_component) = &self.adl_component {
            if self.component == "main" {
                format!("{} {}", self.name, adl_component.loc_str())
            } else if let Some(adl_mapping) = &self.adl_mapping {
                format!(
                    "{} {} [{} {}]",
                    self.name,
                    adl_mapping.loc_str(),
                    self.component,
                    adl_component.loc_str()
                )
            } else {
                panic!(
                    "Should be unreachable; either the component is main or there is an ADL mapping for the cell!"
                )
            }
        } else if self.is_primitive {
            format!(
                "{} (primitive) {}",
                self.name,
                self.adl_mapping.clone().unwrap().loc_str()
            )
        } else {
            panic!(
                "Should be unreachable; either the cell is a primitive or has a component ADL!"
            )
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Default, PartialOrd, Ord)]
pub struct GroupId(u32);
entity_impl!(GroupId, "group");

#[derive(Debug, Clone)]
/// Represents a group activation from a component's control.
struct Group {
    name: String,
    probe: SignalRef,
    invokes: SmallVec<[InvokeId; 6]>,
    probe_idx: u32,
    component: String,
    /// Information necessary to map back to the ADL. Could be None if the group was generated
    /// by the Calyx compiler (ex. invoke groups).
    /// Will only be Some if we are dealing with an ADL that almost 1-1 maps to Calyx (Calyx-py)
    /// NOTE: In the future we might want to make a struct with only the necessary information
    adl_mapping: Option<PosInfo>,
}

impl Group {
    /// String representation of group for trace and visualizations
    pub fn display_name(&self) -> String {
        // remove unique group identifier.
        self.name.split("UG").next().unwrap().to_string()
    }

    /// String representation of group for stats
    pub fn static_name(&self) -> String {
        format!(
            "{}.{}",
            self.component,
            self.name.split("UG").next().unwrap()
        )
    }

    pub fn adl_display_name(&self) -> String {
        if let Some(adl_mapping) = &self.adl_mapping {
            adl_mapping.adl_str()
        } else {
            format!("'{}' {{{COMPILER_GENERATED_MSG}}}", self.display_name())
        }
    }

    pub fn mixed_display_name(&self) -> String {
        if let Some(adl_mapping) = &self.adl_mapping {
            format!("{} {}", self.name, adl_mapping.loc_str())
        } else {
            format!("{} {{{COMPILER_GENERATED_MSG}}}", self.display_name())
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Default, PartialOrd, Ord)]
pub struct ControlId(u32);
entity_impl!(ControlId, "control");

#[derive(Debug, Clone)]
/// Represents a control activation from a component.
struct Control {
    name: String,
    go: SignalRef,
    invokes: SmallVec<[InvokeId; 6]>,
    go_idx: u32,
    _pos: u32,
    pretty: String,
}

impl Control {
    pub fn display_name(&self) -> String {
        format!("{} ~ {} (ctrl)", self.name, self.pretty)
    }

    pub fn pretty(&self) -> String {
        self.pretty.to_string()
    }

    pub fn adl_display_name(&self) -> String {
        format!("{COMPILER_GENERATED_MSG} (ctrl)")
    }

    pub fn mixed_display_name(&self) -> String {
        format!("{} (ctrl) {{{COMPILER_GENERATED_MSG}}}", self.name)
    }
}

#[derive(Debug, Clone)]
/// Represents a Control Register within a component.
struct CRegister {
    name: String,
    write_en_signal_ref: SignalRef,
    in_signal_ref: SignalRef,
    /// if the register is a pd, then this would be false.
    is_fsm: bool,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct RegisterId(u32);
entity_impl!(RegisterId, "register");

#[derive(Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct InvokeId(u32);
entity_impl!(InvokeId, "invoke");

#[derive(Debug, Clone)]
/// Represents a group invoking either a component or primitive cell, or another group (via a structural enable).
struct Invoke {
    /// The name of the cell being invoked.
    _name: String,
    probe: SignalRef,
    target: InvokeTarget,
    probe_idx: u32,
}

#[derive(Debug, Clone)]
/// Represents a target of an invoke from a group/control node
enum InvokeTarget {
    // Group invokes a component/primitive cell
    Cell(CellId),
    // Control/Group invokes a group (structural enable)
    Group(GroupId),
    // Control invokes a control
    Control(ControlId),
}

#[derive(Debug, Clone)]
/// Information necessary to "stitch" Control nodes in the appropriate spot within a Cell.
pub struct CellControl {
    /// Map from group to their Control parent (so we know where in the tree to add groups to.)
    group_to_parent: FxHashMap<String, Option<ControlId>>,
    /// The outermost control node, if one exists.
    /// Will be None when there are no control nodes in the component.
    /// (ex. a component with a single-group control)
    toplevel_control: Option<ControlId>,
    /// Necessary for timeline view tracking (not used for constructing the call tree)
    fsms: Vec<String>,
    pds: Vec<String>,
}

#[derive(Clone, Debug)]
/// Represents the static call tree (all possible calls).
pub struct Design {
    cells: PrimaryMap<CellId, Cell>,
    controls: PrimaryMap<ControlId, Control>,
    groups: PrimaryMap<GroupId, Group>,
    invokes: PrimaryMap<InvokeId, Invoke>,
    control_registers: PrimaryMap<RegisterId, CRegister>,
    main: CellId,
    clk: SignalRef,
    signals: Vec<SignalRef>,
    register_write_ens_to_in: FxHashMap<SignalRef, (SignalRef, RegisterId)>,
}

/// Returns the main scope if one exists.
/// NOTE: Some versions of Verilator/OSs have a different sequence of toplevel scopes.
/// (ex. TOP.toplevel.main vs toplevel.main)
fn find_main_scope(h: &wellen::Hierarchy) -> Result<ScopeRef> {
    h.all_scopes()
        .find(|s| h[*s].name(h) == "main")
        .ok_or(anyhow!("Failed to find main scope"))
}

pub enum AdlMode {
    Calyx,
    Mixed,
    CalyxPy,
}

impl Design {
    pub fn new(
        h: &wellen::Hierarchy,
        c: ControlInfo,
        s: SharedCellsInfo,
    ) -> Result<Self> {
        let main = find_main_scope(h)?;
        let clk = get_var(h, &h[main], "clk")?;
        let clk = h[clk].signal_ref();
        let mut out = Self {
            cells: PrimaryMap::new(),
            groups: PrimaryMap::new(),
            invokes: PrimaryMap::new(),
            controls: PrimaryMap::new(),
            control_registers: PrimaryMap::new(),
            main: CellId(u32::MAX),
            clk,
            signals: vec![],
            register_write_ens_to_in: FxHashMap::default(),
        };
        out.populate(h, c, s)?;
        out.build_idx();
        out.build_register_signal_map();
        Ok(out)
    }

    pub fn get_signals(&self) -> Vec<SignalRef> {
        self.signals.clone()
    }

    pub fn get_register_signals_map(
        &self,
    ) -> FxHashMap<SignalRef, (SignalRef, RegisterId)> {
        self.register_write_ens_to_in.clone()
    }

    pub fn clk(&self) -> SignalRef {
        self.clk
    }

    pub fn main_probes(&self) -> (SignalRef, SignalRef) {
        self.cells[self.main].probes.unwrap()
    }

    /// Adds all control register updates to the timeline. (This is done separately from the
    /// Petal trace construction; control registers updates are not part of the trace and are
    /// only used in the timeline view for better understanding of where "control cycles" come from)
    pub fn add_control_registers_to_timeline(
        &self,
        timeline: &mut CalyxTimeline,
        register_value_diffs: FxHashMap<u64, FxHashMap<RegisterId, u64>>,
    ) -> Result<()> {
        let mut ordered_cycles: Vec<u64> =
            register_value_diffs.keys().copied().collect();
        ordered_cycles.sort();
        for cycle in ordered_cycles {
            let diff_map = &register_value_diffs[&cycle];
            let mut uuid_to_out_string: FxHashMap<Uuid, String> =
                FxHashMap::default();

            for (id, new_value) in diff_map {
                let reg = &self.control_registers[*id];
                let update_str = format!("{}: {}", reg.name, new_value);
                let uuid = timeline.get_control_register_uuid(id);
                if let Some(s) = uuid_to_out_string.get(uuid) {
                    uuid_to_out_string.insert(
                        *uuid,
                        format!("{s}, {update_str}").to_string(),
                    );
                } else {
                    uuid_to_out_string.insert(*uuid, update_str);
                }
            }

            for (uuid, out_str) in uuid_to_out_string {
                timeline.register_event(
                    out_str.clone(),
                    uuid,
                    cycle,
                    Type::SliceBegin,
                )?;
                timeline.register_event(
                    out_str,
                    uuid,
                    cycle + 1,
                    Type::SliceEnd,
                )?;
            }
        }
        Ok(())
    }

    /// Computes the active call tree from a cycle, represented as a list of stacks (Python Petal style).
    /// values is the probe signals bitvector obtained from the cycle in question.
    pub fn compute_cycle_trace(
        &self,
        values: &BitVecValue,
        adl_mode: AdlMode,
    ) -> Result<(Vec<Stack>, CurrentlyActive, bool)> {
        let main = &self.cells[self.main];
        let (main_go, main_done) = main.probe_idxs.unwrap();
        let main_active =
            values.is_bit_set(main_go) & !values.is_bit_set(main_done);
        let mut stacks = vec![];
        let mut group_or_primitive_leaf: bool = false;
        let mut current_active = CurrentlyActive::new();
        if main_active {
            (stacks, group_or_primitive_leaf) = self.find_active_from_cell(
                values,
                self.main,
                vec![],
                &mut current_active,
                &adl_mode,
            );
            stacks.sort();
            stacks.dedup();
        }
        Ok((stacks, current_active, group_or_primitive_leaf))
    }

    /// Constructs all tracks in the timeline view.
    pub fn build_timeline_tracks(
        &self,
        t: &mut CalyxTimeline,
        par_tracks: &FxHashMap<String, FxHashMap<String, u32>>,
    ) -> Result<()> {
        self.build_cell_timeline_tracks(&self.main, t, par_tracks)
    }

    pub fn get_group_component_names(&self) -> Vec<(GroupId, String, String)> {
        self.groups
            .iter()
            .map(|(id, group)| {
                (id, group.static_name(), group.component.clone())
            })
            .collect()
    }

    pub fn get_cell_name_fsm_count(
        &self,
    ) -> Vec<(CellId, String, FxHashSet<RegisterId>)> {
        let mut out: Vec<(CellId, String, FxHashSet<RegisterId>)> = Vec::new();
        for (id, cell) in self.cells.iter() {
            if !cell.is_primitive {
                let name = cell.stats_name().to_string();
                let mut fsms: FxHashSet<RegisterId> = FxHashSet::default();
                let mut pds: FxHashSet<RegisterId> = FxHashSet::default();
                for r in cell.control_registers.iter() {
                    if self.control_registers[*r].is_fsm {
                        fsms.insert(*r);
                    } else {
                        pds.insert(*r);
                    }
                }
                out.push((id, name, fsms));
            }
        }
        out
    }

    /// Embeds ADL position information into call tree nodes.
    /// Used in Calyx-Py profiling (since there is a straightfoward mapping from Calyx constructs
    /// to Calyx-Py constructs)
    pub fn embed_pos(&mut self, component_infos: &Vec<ComponentInfo>) {
        self.embed_pos_in_cell(&self.main.clone(), None, component_infos)
    }
}

pub fn parse_probe_name(name: &str) -> Result<ProbeName<'_>> {
    let pat = "___";
    if let Some(prefix) = name.strip_suffix("_group_probe") {
        // ex. invoke2UG___main_group_probe
        let mut parts = prefix.split(pat);
        let group = parts.next().unwrap();
        let component = parts.next().unwrap();
        Ok(ProbeName::Group { group, component })
    } else if let Some(prefix) = name.strip_suffix("_cell_probe") {
        // ex. mac___invoke2UG___main_cell_probe
        let mut parts = prefix.split(pat);
        let cell = parts.next().unwrap();
        let group = parts.next().unwrap();
        let component = parts.next().unwrap();
        Ok(ProbeName::InvokeCell {
            name: cell,
            group,
            component,
        })
    } else if let Some(prefix) = name.strip_suffix("_primitive_probe") {
        // ex. lt0___in_rangeUG___main_primitive_probe
        let mut parts = prefix.split(pat);
        let primitive = parts.next().unwrap();
        let group = parts.next().unwrap();
        let component = parts.next().unwrap();
        Ok(ProbeName::InvokePrimitive {
            name: primitive,
            group,
            component,
        })
    } else if let Some(prefix) = name.strip_suffix("_se_probe") {
        // ex. wr_a___wr_b___main_se_probe
        let mut parts = prefix.split(pat);
        let enabled_group = parts.next().unwrap();
        let caller_group = parts.next().unwrap();
        let component = parts.next().unwrap();
        Ok(ProbeName::InvokeGroup {
            name: enabled_group,
            group: caller_group,
            component,
        })
    } else {
        anyhow::bail!("failed to parse {name}")
    }
}

// trying to build up the same thing that we have in pypetal for now.
pub type Stack = Vec<String>;

impl Design {
    /// Builds up all active tree paths this cycle from the component cell of cell_id.
    /// prefix is the state of the stack before this particular cell.
    /// Returns: The active call tree and a flag indicating whether there was a leaf (last element of a stack)
    /// that is a group or primitive (used to classify cells for `staticstics::CellStats`).
    /// NOTE: This function is co-recursive with `find_active_from_control()` and `find_active_from_group()`.
    fn find_active_from_cell(
        &self,
        value: &BitVecValue,
        cell_id: CellId,
        mut prefix: Stack,
        active_this_cycle: &mut CurrentlyActive,
        adl_mode: &AdlMode,
    ) -> (Vec<Stack>, bool) {
        let cell = &self.cells[cell_id];
        active_this_cycle.add_active_cell(cell_id);
        if let Some((main_go_idx, main_done_idx)) = cell.probe_idxs {
            // the main component cell is the only one to have a probe_idx.
            if value.is_bit_set(main_go_idx) && !value.is_bit_set(main_done_idx)
            {
                match adl_mode {
                    AdlMode::CalyxPy => prefix.push(cell.adl_display_name()),
                    AdlMode::Calyx => prefix.push(cell.display_name()),
                    AdlMode::Mixed => prefix.push(cell.mixed_display_name()),
                }
            } else {
                return (vec![prefix], false);
            }
        }
        if cell.groups.is_empty() && cell.control.is_empty() {
            // No more children, so this is a sink.
            return (vec![prefix], false);
        }
        let mut out = vec![];
        let mut group_primitive_leaf = false;
        for &group_idx in &cell.groups {
            let group = &self.groups[group_idx];
            if value.is_bit_set(group.probe_idx) {
                let (mut group_stacks, flag) = self.find_active_from_group(
                    value,
                    group_idx,
                    prefix.clone(),
                    active_this_cycle,
                    adl_mode,
                );
                out.append(&mut group_stacks);
                group_primitive_leaf |= flag;
            }
        }
        for &control_idx in &cell.control {
            let control = &self.controls[control_idx];
            if value.is_bit_set(control.go_idx) {
                let (mut control_stacks, flag) = self.find_active_from_control(
                    value,
                    control_idx,
                    prefix.clone(),
                    active_this_cycle,
                    adl_mode,
                );
                out.append(&mut control_stacks);
                group_primitive_leaf |= flag;
            }
        }

        // Edge case: if the cell is static, there could be cycles where
        // no group or control is active!
        if out.is_empty() {
            out.push(prefix);
        }

        (out, group_primitive_leaf)
    }

    /// Builds up all active tree paths this cycle from the group of group_id.
    /// prefix is the state of the stack before this particular group.
    /// Returns: The active call tree and a flag indicating whether there was a leaf (last element of a stack)
    /// that is a group or primitive (used to classify cells for `staticstics::CellStats`).
    /// NOTE: This function is co-recursive with compute_cell(), and only called when
    /// the control group is active (otherwise this function would not be called.)
    fn find_active_from_control(
        &self,
        value: &BitVecValue,
        control_id: ControlId,
        mut prefix: Stack,
        active_this_cycle: &mut CurrentlyActive,
        adl_mode: &AdlMode,
    ) -> (Vec<Stack>, bool) {
        let control = &self.controls[control_id];
        let control_display_name = match adl_mode {
            AdlMode::Calyx => control.display_name(),
            AdlMode::Mixed => control.mixed_display_name(),
            AdlMode::CalyxPy => control.adl_display_name(),
        };
        prefix.push(control_display_name);
        active_this_cycle.add_active_control(control_id);
        // it probably wouldn't make any sense for a control to not contain any invokes?
        assert!(!control.invokes.is_empty());
        let mut out = vec![];
        let mut group_primitive_leaf = false;
        for &invoke_id in &control.invokes {
            let invoke = &self.invokes[invoke_id];
            if value.is_bit_set(invoke.probe_idx) {
                match invoke.target {
                    InvokeTarget::Cell(_) => {
                        panic!(
                            "Control node {} directly invokes cell (control nodes should only invoke control nodes and groups)",
                            control.name
                        )
                    }
                    InvokeTarget::Group(target_group_id) => {
                        let (mut group_stacks, flag) = self
                            .find_active_from_group(
                                value,
                                target_group_id,
                                prefix.clone(),
                                active_this_cycle,
                                adl_mode,
                            );
                        out.append(&mut group_stacks);
                        group_primitive_leaf |= flag;
                    }
                    InvokeTarget::Control(target_control_id) => {
                        let (mut control_stacks, flag) = self
                            .find_active_from_control(
                                value,
                                target_control_id,
                                prefix.clone(),
                                active_this_cycle,
                                adl_mode,
                            );
                        out.append(&mut control_stacks);
                        group_primitive_leaf |= flag;
                    }
                }
            }
        }
        // if the control node invokes control nodes/groups but none of them are active,
        // we still need to add the control node.
        // NOTE: this would be a control cycle.
        if !control.invokes.is_empty() && out.is_empty() {
            out.push(prefix);
        }

        (out, group_primitive_leaf)
    }

    /// Builds up all active tree paths this cycle from the group of group_id.
    /// prefix is the state of the stack before this particular group.
    /// Returns: The active call tree and a flag indicating whether there was a leaf (last element of a stack)
    /// that is a group or primitive (used to classify cells for `staticstics::CellStats`).
    /// NOTE: This function is co-recursive with compute_cell() and compute_control(), and only called when
    /// the group is active (otherwise this function would not be called.)
    fn find_active_from_group(
        &self,
        value: &BitVecValue,
        group_id: GroupId,
        mut prefix: Stack,
        active_this_cycle: &mut CurrentlyActive,
        adl_mode: &AdlMode,
    ) -> (Vec<Stack>, bool) {
        let group = &self.groups[group_id];
        let group_display_name = match adl_mode {
            AdlMode::Calyx => group.display_name(),
            AdlMode::Mixed => group.mixed_display_name(),
            AdlMode::CalyxPy => group.adl_display_name(),
        };
        prefix.push(group_display_name);
        active_this_cycle.add_active_group(group_id);
        if group.invokes.is_empty() {
            // this group is a leaf, since it does not invoke anything.
            return (vec![prefix], true);
        }
        let mut out: Vec<Stack> = vec![];
        let mut group_or_primitive_leaf = false;
        for &invoke_id in &group.invokes {
            let mut this_thread_prefix = prefix.clone();
            let invoke = &self.invokes[invoke_id];
            if value.is_bit_set(invoke.probe_idx) {
                // the invoke probe is active
                match invoke.target {
                    InvokeTarget::Cell(target_cell_id) => {
                        // component or primitive cell activation
                        let target_cell = &self.cells[target_cell_id];
                        let display_name = match &adl_mode {
                            AdlMode::Calyx => target_cell.display_name(),
                            AdlMode::Mixed => target_cell.mixed_display_name(),
                            AdlMode::CalyxPy => target_cell.adl_display_name(),
                        };
                        this_thread_prefix.push(display_name);
                        if target_cell.is_primitive {
                            out.push(this_thread_prefix);
                            // A primitive will always be a leaf as it cannot call anything else.
                            group_or_primitive_leaf = true;
                        } else {
                            let (mut cell_stacks, flag) = self
                                .find_active_from_cell(
                                    value,
                                    target_cell_id,
                                    this_thread_prefix.clone(),
                                    active_this_cycle,
                                    adl_mode,
                                );
                            out.append(&mut cell_stacks);
                            group_or_primitive_leaf |= flag;
                        }
                    }
                    InvokeTarget::Group(target_group_id) => {
                        // structural enable (group enables another group)
                        let (mut group_stacks, flag) = self
                            .find_active_from_group(
                                value,
                                target_group_id,
                                this_thread_prefix.clone(),
                                active_this_cycle,
                                adl_mode,
                            );
                        out.append(&mut group_stacks);
                        group_or_primitive_leaf |= flag;
                    }
                    InvokeTarget::Control(_) => {
                        panic!("Group should not invoke a Control node!")
                    }
                }
            }
        }
        (out, group_or_primitive_leaf)
    }

    /// Maps between probes and their indices in self.signals().
    fn build_idx(&mut self) {
        self.signals = self.probe_signals();
        let to_index = FxHashMap::from_iter(
            self.signals
                .iter()
                .enumerate()
                .map(|(idx, &signal)| (signal, idx as u32)),
        );

        for (_, cell) in self.cells.iter_mut() {
            if let Some((main_go_probe, main_done_probe)) = cell.probes {
                cell.probe_idxs = Some((
                    to_index[&main_go_probe],
                    to_index[&main_done_probe],
                ));
            }
        }

        for (_, control) in self.controls.iter_mut() {
            control.go_idx = to_index[&control.go];
        }

        for (_, group) in self.groups.iter_mut() {
            group.probe_idx = to_index[&group.probe];
        }

        for (_, invoke) in self.invokes.iter_mut() {
            invoke.probe_idx = to_index[&invoke.probe];
        }
    }

    /// Same thing as build_idx, but with control registers (can't store them in a BitVector)
    fn build_register_signal_map(&mut self) {
        for (id, register) in self.control_registers.iter_mut() {
            self.register_write_ens_to_in.insert(
                register.write_en_signal_ref,
                (register.in_signal_ref, id),
            );
        }
    }

    /// Helper for build_idx() to obtain all probe signals.
    fn probe_signals(&self) -> Vec<SignalRef> {
        let mut signals = vec![];
        for cell in self.cells.values() {
            if let Some((main_go_probe, main_done_probe)) = cell.probes {
                signals.push(main_go_probe);
                signals.push(main_done_probe);
            }
        }

        for control in self.controls.values() {
            signals.push(control.go);
        }

        for group in self.groups.values() {
            signals.push(group.probe);
        }

        for invoke in self.invokes.values() {
            signals.push(invoke.probe);
        }

        signals.push(self.clk);

        signals.sort();
        signals.dedup();
        signals
    }

    /// Helper function for `self.scan_probes()`.
    /// Construct control nodes and the edges between them, and returns information necessary
    /// to "stitch" the control nodes into the tree.
    fn populate_control(
        &mut self,
        h: &Hierarchy,
        s: ScopeRef,
        c: &ControlInfo,
        component: &str,
    ) -> Result<CellControl> {
        let mut toplevel_control = None;

        let mut pos_to_id = FxHashMap::default();
        let mut descriptor_to_id: FxHashMap<String, ControlId> =
            FxHashMap::default();

        let descriptors = c.descriptors(component);
        // let ctrl_map = descriptors.control_pos;

        let mut fsms = Vec::new();
        let mut pds = Vec::new();

        // iterate through control par descriptors and construct Control nodes
        for (d, pos_set) in descriptors.control_pos.iter() {
            if pos_set.is_empty() {
                // if the pos_set for a descriptor is empty, it's not a real control descriptor
                println!(
                    "Skipping descriptor with empty pos set (static control; will not manifest in a control group): {d}"
                );
                continue;
            }
            if let Some((pretty, pos)) = c.get_pretty(pos_set) &&
                // any pos without an entry in tdcc was compiled away; we ignore these.
                let Some(tdcc_info_vec) = c.get_tdcc(pos)?
            {
                // pos is the entry to the Calyx-generated position of the control node,
                // so there should only be one entry in the Vector.
                assert_eq!(tdcc_info_vec.len(), 1);
                let tdcc_info = tdcc_info_vec.iter().next().unwrap();
                let name = tdcc_info.name.clone();

                match &tdcc_info.control_register {
                    ControlRegister::Fsm(f) => fsms.push(f.clone()),
                    ControlRegister::Pd(p) => pds.append(&mut p.clone()),
                };

                let ctrl_scope = get_scope(h, &h[s], &format!("{name}_go"))?;
                let go_ref = get_var(h, &h[ctrl_scope], "out")?;
                let go = h[go_ref].signal_ref();

                let ctrl = Control {
                    name,
                    go,
                    invokes: smallvec![],
                    go_idx: u32::MAX,
                    _pos: pos,
                    pretty,
                };
                let ctrl_id = self.controls.push(ctrl);
                pos_to_id.insert(pos, ctrl_id);
                descriptor_to_id.insert(d.clone(), ctrl_id);
            } else {
                println!(
                    "Could not find control group for position set {pos_set:?}"
                );
            }
        }

        // compute groups' parent info (reverse sort for easier checking)
        let mut ctrl_desc_rev_sorted =
            descriptor_to_id.keys().collect::<Vec<_>>();
        ctrl_desc_rev_sorted.sort();
        ctrl_desc_rev_sorted.reverse();

        for (idx, desc) in ctrl_desc_rev_sorted.iter().enumerate() {
            let ctrl_id = descriptor_to_id[*desc];
            let control = self.controls.get(ctrl_id).unwrap();
            let mut found = false;
            let mut i = idx + 1;
            while i < ctrl_desc_rev_sorted.len() {
                let &maybe_parent = ctrl_desc_rev_sorted.get(i).unwrap();
                if desc.starts_with(maybe_parent) {
                    // maybe_parent --> desc invoke
                    let invoke = Invoke {
                        _name: control.name.clone(),
                        probe: control.go,
                        target: InvokeTarget::Control(ctrl_id),
                        probe_idx: u32::MAX,
                    };
                    let invoke_id = self.invokes.push(invoke);
                    let parent_ctrl_id = descriptor_to_id[maybe_parent];
                    let parent_ctrl =
                        self.controls.get_mut(parent_ctrl_id).unwrap();
                    parent_ctrl.invokes.push(invoke_id);
                    found = true;
                    break;
                }
                i += 1;
            }
            if !found {
                // the only ctrl node without a parent should be the toplevel control node
                assert_eq!(idx, ctrl_desc_rev_sorted.len() - 1);
                toplevel_control = Some(ctrl_id);
            }
        }

        let group_to_parent = Self::groups_to_ctrl_parent(
            &descriptor_to_id,
            descriptors,
            &ctrl_desc_rev_sorted,
        );

        Ok(CellControl {
            group_to_parent,
            toplevel_control,
            fsms,
            pds,
        })
    }

    /// Helper function that returns a map from groups to their control parent in the tree.
    fn groups_to_ctrl_parent(
        descriptor_to_id: &FxHashMap<String, ControlId>,
        descriptors: &PathDescriptorInfo,
        ctrl_desc_sorted: &Vec<&String>,
    ) -> FxHashMap<String, Option<ControlId>> {
        let mut out: FxHashMap<String, Option<ControlId>> =
            FxHashMap::default();
        for (g, d) in descriptors.enables.iter() {
            // find the immediate control node parent
            let mut found = false;
            for &cd in ctrl_desc_sorted.iter() {
                if d.starts_with(cd) {
                    out.insert(
                        g.clone(),
                        Some(*descriptor_to_id.get(cd).unwrap()),
                    );
                    found = true;
                    break; // breaking because we found the first match
                }
            }
            if !found {
                out.insert(g.clone(), None);
            }
        }
        out
    }

    /// Builds the static call tree by scanning through all probes to find tree edges.
    fn populate(
        &mut self,
        h: &wellen::Hierarchy,
        c: ControlInfo,
        s: SharedCellsInfo,
    ) -> Result<()> {
        let main_scope = find_main_scope(h)?;
        let main_go = get_var(h, &h[main_scope], "go")?;
        let main_done = get_var(h, &h[main_scope], "done")?;
        let mut main_cell = Cell {
            name: "main".to_string(),
            full_path: "main".to_string(),
            control: smallvec![],
            groups: smallvec![],
            control_registers: smallvec![],
            probes: Some((h[main_go].signal_ref(), h[main_done].signal_ref())),
            _scope: Some(main_scope),
            is_primitive: false,
            instances: smallvec![],
            component: String::new(),
            probe_idxs: None,
            replacement: None,
            adl_mapping: None,
            adl_component: None,
        };
        // add control nodes for main
        self.scan_probes(h, main_scope, &mut main_cell, &c, &s)?;
        self.main = self.cells.push(main_cell);
        Ok(())
    }

    /// Constructs the static tree from available probes.
    fn scan_probes(
        &mut self,
        h: &Hierarchy,
        cell_scope: ScopeRef,
        cell: &mut Cell,
        c: &ControlInfo,
        s: &SharedCellsInfo,
    ) -> Result<()> {
        // Create control nodes and add an edge from a cell to the toplevel control.
        let component = get_component(h, cell_scope)?;
        cell.component = component.to_string();
        let CellControl {
            group_to_parent,
            toplevel_control,
            fsms,
            pds,
        } = self.populate_control(h, cell_scope, c, component)?;
        if let Some(top_ctrl) = toplevel_control {
            cell.control.push(top_ctrl);
        }

        // add entries for CRegisters
        for register_scope in h[cell_scope].scopes(h).filter(|p| {
            fsms.contains(&h[*p].name(h).to_string())
                || pds.contains(&h[*p].name(h).to_string())
        }) {
            let write_en_var = get_var(h, &h[register_scope], "write_en")?;
            let in_var = get_var(h, &h[register_scope], "in")?;
            let write_en_signal_ref = h[write_en_var].signal_ref();
            let in_signal_ref = h[in_var].signal_ref();
            let name = h[register_scope].name(h).to_string();
            let register_id = self.control_registers.push(CRegister {
                is_fsm: fsms.contains(&name),
                name,
                write_en_signal_ref,
                in_signal_ref,
            });
            cell.control_registers.push(register_id);
        }

        // cell.groups should not contain any structurally enabled groups.
        // So, we will record all names of structurally invoked groups so later we can prevent cell.groups from
        // containing any groups with such names.
        let mut structurally_invoked_group_names: Vec<String> = vec![];
        // collection of all groups defined within this cell. Will filter out those in structurally_invoked_groups
        let mut all_groups: Vec<GroupId> = vec![];
        let mut parentless_invokes: Vec<(&str, InvokeId)> = vec![];
        // First pass approach: Iterate through all structural enables first to prevent creating duplicate group entries.
        // (structurally enabled groups have two probes; the structural enable probe and the group active probe.)
        for probe_scope in h[cell_scope].scopes(h) {
            let name = h[probe_scope].name(h);
            if name.ends_with("_se_probe") {
                // only grabbing structural enables
                let out = get_var(h, &h[probe_scope], "out")?;
                let probe = h[out].signal_ref();
                let probe_name = parse_probe_name(name)?;
                match probe_name {
                    ProbeName::InvokeGroup {
                        name,
                        group,
                        component,
                    } => {
                        assert!(
                            cell.component.is_empty()
                                || cell.component == component
                        );
                        if cell.component.is_empty() {
                            cell.component = component.to_string();
                        }
                        // check for the target group, and create it if it does not exist.
                        let maybe_target_group = all_groups
                            .iter()
                            .find(|&&g| self.groups[g].name == name);
                        let target = if let Some(&t) = maybe_target_group {
                            t
                        } else {
                            let invokes = parentless_invokes
                                .extract_if(.., |(group_name, _)| {
                                    group_name == &name
                                })
                                .map(|(_, ii)| ii)
                                .collect();
                            let group_id = self.groups.push(Group {
                                name: name.to_string(),
                                probe,
                                invokes,
                                probe_idx: u32::MAX,
                                component: component.to_string(),
                                adl_mapping: None,
                            });
                            all_groups.push(group_id);
                            structurally_invoked_group_names
                                .push(name.to_string());
                            group_id
                        };
                        let invoke_id = self.invokes.push(Invoke {
                            _name: name.to_string(),
                            probe,
                            target: InvokeTarget::Group(target),
                            probe_idx: u32::MAX,
                        });
                        // Check for the caller group, and add an Invoke entry if it exists.
                        let maybe_caller_group = all_groups
                            .iter()
                            .find(|&&g| self.groups[g].name == group);
                        if let Some(&group) = maybe_caller_group {
                            self.groups[group].invokes.push(invoke_id);
                        } else {
                            parentless_invokes.push((group, invoke_id))
                        }
                    }
                    _ => {
                        panic!("{name} should be a structural enable probe!")
                    }
                }
            }
        }
        // iterate through all probes in this scope. Structural enable probes will be ignored as nodes for them were previously created.
        for probe_scope in h[cell_scope].scopes(h) {
            let name = h[probe_scope].name(h);
            if name.ends_with("_probe")
                && !name.ends_with("_contprimitive_probe")
            {
                // ignore continuous primitives for now
                let out = get_var(h, &h[probe_scope], "out")?;
                let probe = h[out].signal_ref();
                let probe_name = parse_probe_name(name)?;
                match probe_name {
                    ProbeName::Group { group, component } => {
                        assert!(
                            cell.component.is_empty()
                                || cell.component == component
                        );
                        if cell.component.is_empty() {
                            cell.component = component.to_string();
                        }
                        let name = group.to_string();
                        let invokes = parentless_invokes
                            .extract_if(.., |(group_name, _)| {
                                group_name == &name
                            })
                            .map(|(_, ii)| ii)
                            .collect();
                        if !structurally_invoked_group_names.contains(&name) {
                            // only create a group entry if this group was not structurally enabled.
                            let groupid = self.groups.push(Group {
                                name: name.clone(),
                                probe,
                                invokes,
                                probe_idx: u32::MAX,
                                component: component.to_string(),
                                adl_mapping: None,
                            });
                            if let Some(Some(ctrl_parent)) =
                                group_to_parent.get(&name)
                            {
                                let invokeid = self.invokes.push(Invoke {
                                    _name: name,
                                    probe,
                                    target: InvokeTarget::Group(groupid),
                                    probe_idx: u32::MAX,
                                });
                                self.controls[*ctrl_parent]
                                    .invokes
                                    .push(invokeid);
                            } else {
                                cell.groups.push(groupid);
                            }
                            all_groups.push(groupid);
                        }
                    }
                    ProbeName::InvokePrimitive {
                        name,
                        group,
                        component,
                    }
                    | ProbeName::InvokeCell {
                        name,
                        group,
                        component,
                    } => {
                        assert!(
                            cell.component.is_empty()
                                || cell.component == component
                        );
                        let is_primitive = matches!(
                            probe_name,
                            ProbeName::InvokePrimitive { .. }
                        );
                        if cell.component.is_empty() {
                            cell.component = component.to_string();
                        }
                        let maybe_group = all_groups
                            .iter()
                            .find(|&&g| self.groups[g].name == group);
                        // create target cell.
                        let maybe_target = cell
                            .instances
                            .iter()
                            .find(|&&c| self.cells[c].name == name);
                        let target = if let Some(&t) = maybe_target {
                            t
                        } else {
                            // TODO: Primitives would not have a scope if they are shared.
                            let scope = get_scope(h, &h[cell_scope], name).ok();
                            let replacement = if scope.is_none() {
                                s.get_replacement(
                                    component.to_string(),
                                    name.to_string(),
                                )
                            } else {
                                None
                            };
                            let full_path =
                                format!("{}.{name}", cell.full_path);
                            let mut cell_instance = Cell {
                                name: name.to_string(),
                                full_path: full_path.clone(),
                                groups: smallvec![],
                                control: smallvec![],
                                control_registers: smallvec![],
                                _scope: scope,
                                is_primitive,
                                instances: smallvec![],
                                component: String::new(),
                                probes: None,
                                probe_idxs: None,
                                replacement,
                                adl_component: None,
                                adl_mapping: None,
                            };
                            if !is_primitive {
                                assert!(scope.is_some());
                                self.scan_probes(
                                    h,
                                    scope.unwrap(),
                                    &mut cell_instance,
                                    c,
                                    s,
                                )?;
                            }
                            let cell_id = self.cells.push(cell_instance);
                            cell.instances.push(cell_id);
                            cell_id
                        };
                        let invoke_id = self.invokes.push(Invoke {
                            _name: name.to_string(),
                            probe,
                            target: InvokeTarget::Cell(target),
                            probe_idx: u32::MAX,
                        });
                        if let Some(&group) = maybe_group {
                            self.groups[group].invokes.push(invoke_id);
                        } else {
                            parentless_invokes.push((group, invoke_id))
                        }
                    }
                    ProbeName::InvokeGroup {
                        name: _,
                        group: _,
                        component: _,
                    } => {
                        // we iterated through all structural enables at the beginning, so we will skip those here.
                        continue;
                    }
                }
            }
        }
        assert!(parentless_invokes.is_empty());
        Ok(())
    }

    /// Helper function for `self.build_timeline_tracks()`.
    /// Creates timeline tracks for the control group with ID c, and any of its child
    /// Control or Group nodes.
    fn build_control_timeline_tracks(
        &self,
        c: &ControlId,
        control_groups_uuid: u64,
        component: &str,
        t: &mut CalyxTimeline,
        par_tracks: &FxHashMap<String, FxHashMap<String, u32>>,
        thread_tracks: &FxHashMap<u32, u64>,
    ) -> Result<()> {
        // create a descriptor for control group
        let control = &self.controls[*c];
        let name = format!("Control Group: {}", control.pretty());
        t.register_control(
            name,
            control.pretty.to_string(),
            *c,
            control_groups_uuid,
        )?;

        // recurse on any control/group that we find
        for &invoke_id in control.invokes.iter() {
            let invoke = &self.invokes[invoke_id];
            match invoke.target {
                InvokeTarget::Cell(_) => {
                    panic!(
                        "Control node {} directly invokes cell (control nodes should only invoke control nodes and groups)",
                        control.name
                    )
                }
                InvokeTarget::Group(g_id) => {
                    self.build_group_timeline_tracks(
                        &g_id,
                        component,
                        t,
                        par_tracks,
                        thread_tracks,
                    )?;
                }
                InvokeTarget::Control(c_id) => {
                    self.build_control_timeline_tracks(
                        &c_id,
                        control_groups_uuid,
                        component,
                        t,
                        par_tracks,
                        thread_tracks,
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Helper function for `self.build_timeline_tracks()`.
    /// Creates timeline tracks for the group with ID g, and any of its child
    /// non-primitive Cell or Group nodes.
    fn build_group_timeline_tracks(
        &self,
        g: &GroupId,
        component: &str,
        t: &mut CalyxTimeline,
        par_tracks: &FxHashMap<String, FxHashMap<String, u32>>,
        thread_tracks: &FxHashMap<u32, u64>,
    ) -> Result<()> {
        let group = &self.groups[*g];
        // Goals:
        // register group name and uuid
        assert!(par_tracks.contains_key(component));
        let thread_id = if let Some(t) = par_tracks[component].get(&group.name)
        {
            t
        } else {
            // structurally enabled groups will be under the special "Non-id-ed groups" thread.
            &NON_ID_THREAD
        };
        assert!(thread_tracks.contains_key(thread_id));
        let uuid = thread_tracks[thread_id];
        t.register_group(*g, uuid, group.display_name())?;

        // call `build_cell_timeline_tracks()` on any non-primitive cell we find.
        for &invoke_id in group.invokes.iter() {
            let invoke = &self.invokes[invoke_id];
            match invoke.target {
                InvokeTarget::Cell(cell_id) => {
                    let cell = &self.cells[cell_id];
                    if !cell.is_primitive {
                        self.build_cell_timeline_tracks(
                            &cell_id, t, par_tracks,
                        )?;
                    }
                }
                InvokeTarget::Group(group_id) => {
                    self.build_group_timeline_tracks(
                        &group_id,
                        component,
                        t,
                        par_tracks,
                        thread_tracks,
                    )?;
                }
                InvokeTarget::Control(_) => {
                    panic!("Group should not invoke a Control node!")
                }
            }
        }
        Ok(())
    }

    /// Helper function for `self.build_timeline_tracks()`.
    /// Creates timeline tracks for the component cell with ID c, and any of its child Group or Control nodes.
    /// Also creates a timeline track for the Control registers in this component.
    fn build_cell_timeline_tracks(
        &self,
        c: &CellId,
        t: &mut CalyxTimeline,
        par_tracks: &FxHashMap<String, FxHashMap<String, u32>>,
    ) -> Result<()> {
        let cell = &self.cells[*c];
        // All component cells get their own top-level track
        let (cell_uuid, thread_tracks) = t.register_cell(
            *c,
            cell.full_path.to_string(),
            par_tracks.get(&cell.component),
        )?;

        t.register_control_registers(cell_uuid, &cell.control_registers)?;
        // these still need to be passed into build_control_tracks because control will call groups

        for group in cell.groups.iter() {
            self.build_group_timeline_tracks(
                group,
                &cell.component,
                t,
                par_tracks,
                &thread_tracks,
            )?;
        }

        // Create control
        if !cell.control.is_empty() {
            for control in cell.control.iter() {
                let control_groups_uuid =
                    t.register_control_groups_track(cell_uuid)?;
                self.build_control_timeline_tracks(
                    control,
                    control_groups_uuid,
                    &cell.component,
                    t,
                    par_tracks,
                    &thread_tracks,
                )?;
            }
        }

        Ok(())
    }

    /// Mapping group names to a set of GroupIds (distinct enables of that same group).
    /// Used in `DahliaDesign::new()`.
    pub fn get_group_name_to_ids(
        &self,
    ) -> FxHashMap<String, FxHashSet<GroupId>> {
        let mut out: FxHashMap<String, FxHashSet<GroupId>> =
            FxHashMap::default();
        for (id, g) in self.groups.iter() {
            let name = g.display_name();
            let id_set = out.entry(name.clone()).or_default();
            id_set.insert(id);
        }
        println!("{out:?}");
        out
    }

    /// Helper function for `self.embed_pos()`.
    /// Embeds ADL position information into this cell and its descendants.
    fn embed_pos_in_cell(
        &mut self,
        c: &CellId,
        cell_adl_pos_info: Option<PosInfo>,
        component_infos: &Vec<ComponentInfo>,
    ) {
        let cell = &self.cells[*c];
        if cell.is_primitive {
            self.embed_pos_in_primitive(c, cell_adl_pos_info);
        } else {
            self.embed_pos_in_component_cell(
                c,
                cell_adl_pos_info,
                component_infos,
            );
        }
    }

    /// Helper function for `self.embed_pos_in_cell()`.
    /// Embeds ADL position information into a component (non-primitive) cell and its descendants.
    fn embed_pos_in_component_cell(
        &mut self,
        c: &CellId,
        cell_adl_pos_info: Option<PosInfo>,
        component_infos: &Vec<ComponentInfo>,
    ) {
        let component_info_idx = {
            // update the cell to contain ADL position info.
            let mut_cell = &mut self.cells[*c];
            let component_info_idx = component_infos
                .iter()
                .position(|c| *c.component == mut_cell.component)
                .unwrap();
            let component_info = &component_infos[component_info_idx];
            let mut component_pos_info = PosInfo {
                name: mut_cell.component.clone(),
                filename: component_info.filename.clone().unwrap(),
                linenum: component_info.linenum.unwrap(),
                varname: component_info.varname.clone().unwrap(),
            };
            component_pos_info.cleanup();
            mut_cell.adl_mapping = cell_adl_pos_info;
            mut_cell.adl_component = Some(component_pos_info);
            component_info_idx
        };

        let cell = self.cells[*c].clone();
        for group in cell.groups.iter() {
            self.embed_pos_in_group(group, component_infos, component_info_idx);
        }

        for control in cell.control.iter() {
            self.embed_pos_in_control(
                control,
                component_infos,
                component_info_idx,
            );
        }
    }

    /// Helper function for `self.embed_pos_in_cell()`.
    /// Embeds ADL position information into a primitive cell which will always be a leaf node.
    fn embed_pos_in_primitive(
        &mut self,
        c: &CellId,
        cell_adl_pos_info: Option<PosInfo>,
    ) {
        let cell = &mut self.cells[*c];
        assert!(cell.is_primitive);
        cell.adl_mapping = cell_adl_pos_info;
    }

    /// Helper function for `self.embed_pos()`.
    /// Embeds ADL position information into this group and its descendants.
    fn embed_pos_in_group(
        &mut self,
        g: &GroupId,
        component_infos: &Vec<ComponentInfo>,
        ci_idx: usize,
    ) {
        let component_info = &component_infos[ci_idx];
        {
            // modify group
            let group = &mut self.groups[*g];
            group.adl_mapping = component_info
                .groups
                .iter()
                .find(|p| p.name == group.name)
                .cloned();
        }

        // iterate over the invokes inside the group
        let group = &self.groups[*g].clone();

        for i in group.invokes.iter() {
            let invoke = &self.invokes[*i];
            match invoke.target {
                InvokeTarget::Cell(cell_id) => {
                    let cell = &self.cells[cell_id];
                    // find cell's entry inside component_info
                    let cell_info = component_info
                        .cells
                        .iter()
                        .find(|c| c.name == cell.name)
                        .cloned();
                    assert!(cell_info.is_some());
                    self.embed_pos_in_cell(
                        &cell_id,
                        cell_info,
                        component_infos,
                    );
                }
                InvokeTarget::Group(g) => {
                    self.embed_pos_in_group(&g, component_infos, ci_idx);
                }
                InvokeTarget::Control(_) => {
                    panic!("Group should not invoke a Control node!")
                }
            }
        }
    }

    /// Helper function for `self.embed_pos()`.
    /// Embeds ADL position information into this control group's descendants.
    fn embed_pos_in_control(
        &mut self,
        c: &ControlId,
        component_infos: &Vec<ComponentInfo>,
        ci_idx: usize,
    ) {
        let control = self.controls[*c].clone();
        for i in control.invokes.iter() {
            let invoke = &self.invokes[*i];
            match invoke.target {
                InvokeTarget::Cell(_) => {
                    panic!("Control node should not invoke a Cell node!")
                }
                InvokeTarget::Group(g) => {
                    self.embed_pos_in_group(&g, component_infos, ci_idx);
                }
                InvokeTarget::Control(c) => {
                    self.embed_pos_in_control(&c, component_infos, ci_idx);
                }
            }
        }
    }
}

/// Called to grab the name of the cell's component before computing control.
pub fn get_component(h: &Hierarchy, cell_scope: ScopeRef) -> Result<&str> {
    // brute-force approach: Grab a probe signal in the cell's scope, parse and return the component name.
    let probe_scope = h[cell_scope]
        .scopes(h)
        .find(|s| h[*s].name(h).ends_with("_probe"))
        .unwrap();
    let name = h[probe_scope].name(h);
    match parse_probe_name(name)? {
        ProbeName::Group { component, .. }
        | ProbeName::InvokePrimitive { component, .. }
        | ProbeName::InvokeCell { component, .. }
        | ProbeName::InvokeGroup { component, .. } => Ok(component),
    }
}

/// Returns a VarRef of the name `name` from the scope `s`, if it exists.
pub fn get_var(h: &wellen::Hierarchy, s: &Scope, name: &str) -> Result<VarRef> {
    s.vars(h)
        .find(|&v| h[v].name(h) == name)
        .with_context(|| format!("Failed to find {name} in {}", s.full_name(h)))
}

/// Returns a VarRef of the name `name` from the scope `s`, if it exists.
pub fn get_scope(
    h: &wellen::Hierarchy,
    s: &Scope,
    name: &str,
) -> Result<ScopeRef> {
    s.scopes(h)
        .find(|&v| h[v].name(h) == name)
        .with_context(|| format!("Failed to find {name} in {}", s.full_name(h)))
}

#[derive(PartialEq, Debug)]
/// Represents a probe name after parsing.
pub enum ProbeName<'a> {
    Group {
        group: &'a str,
        component: &'a str,
    },
    InvokePrimitive {
        name: &'a str,
        group: &'a str,
        component: &'a str,
    },
    InvokeCell {
        name: &'a str,
        group: &'a str,
        component: &'a str,
    },
    InvokeGroup {
        // group that is invoked
        name: &'a str,
        // group that does the invoking
        group: &'a str,
        component: &'a str,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_probe_name() {
        assert_eq!(
            parse_probe_name("invoke2UG___main_group_probe").unwrap(),
            ProbeName::Group {
                group: "invoke2UG",
                component: "main"
            }
        );
        assert_eq!(
            parse_probe_name("mac___invoke2UG___main_cell_probe").unwrap(),
            ProbeName::InvokeCell {
                name: "mac",
                group: "invoke2UG",
                component: "main"
            }
        );
        assert_eq!(
            parse_probe_name("lt0___in_rangeUG___main_primitive_probe")
                .unwrap(),
            ProbeName::InvokePrimitive {
                name: "lt0",
                group: "in_rangeUG",
                component: "main"
            }
        );
        assert_eq!(
            parse_probe_name("wr_a___wr_b___main_se_probe").unwrap(),
            ProbeName::InvokeGroup {
                name: "wr_a",
                group: "wr_b",
                component: "main"
            }
        );
    }
}
