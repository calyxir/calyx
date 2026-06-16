use core::panic;

use anyhow::{Context, Result};

use baa::{BitVecOps, BitVecValue};
use cranelift_entity::{PrimaryMap, entity_impl};
use rustc_hash::FxHashMap;
use smallvec::{SmallVec, smallvec};
use wellen::{Hierarchy, Scope, ScopeRef, SignalRef, VarRef};

use crate::control::AllControl;

#[derive(Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct CellId(u32);
entity_impl!(CellId, "cell");

#[derive(Clone, Debug)]
/// Represents a (component/primitive) cell in the static tree.
struct Cell {
    /// The user-defined name of the cell.
    name: String,
    /// Ids of control nodes that could be called from this cell, if it is a component.
    /// NOTE: Primitive cells should have an empty vec here.
    control: SmallVec<[ControlId; 6]>,
    /// Ids of groups that could be called from this cell, if it is a component.
    /// NOTE: Primitive cells should have an empty vec here.
    groups: SmallVec<[GroupId; 6]>,
    /// The scope of the cell in the RTL trace.
    _scope: ScopeRef,
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
}

impl Cell {
    /// String representation of cell for trace and visualizations
    pub fn display_name(&self) -> String {
        if self.is_primitive {
            format!("{} (primitive)", self.name)
        } else if self.component == "main" {
            self.name.clone()
        } else {
            format!("{} [{}]", self.name, self.component)
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct GroupId(u32);
entity_impl!(GroupId, "group");

#[derive(Debug, Clone)]
/// Represents a group activation from a component's control.
struct Group {
    name: String,
    probe: SignalRef,
    invokes: SmallVec<[InvokeId; 6]>,
    probe_idx: u32,
}

impl Group {
    /// String representation of cell for trace and visualizations
    pub fn display_name(&self) -> String {
        // remove unique group identifier.
        self.name.split("UG").next().unwrap().to_string()
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct ControlId(u32);
entity_impl!(ControlId, "control");

#[derive(Debug, Clone)]
/// Represents a control activation from a component.
struct Control {
    name: String,
    go: SignalRef,
    invokes: SmallVec<[InvokeId; 6]>,
    go_idx: u32,
    // deal with registers later?
    pos: u32,
    line_num: u32,
    control_type: String,
}

impl Control {
    pub fn display_name(&self) -> String {
        format!(
            "{} ~ L{}:{} (ctrl)",
            self.name, self.line_num, self.control_type
        )
    }
}

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

#[derive(Clone, Debug)]
/// Represents the static call tree (all possible calls).
pub struct Design {
    cells: PrimaryMap<CellId, Cell>,
    controls: PrimaryMap<ControlId, Control>,
    groups: PrimaryMap<GroupId, Group>,
    invokes: PrimaryMap<InvokeId, Invoke>,
    main: CellId,
    clk: SignalRef,
    signals: Vec<SignalRef>,
}

impl Design {
    pub fn new(h: &wellen::Hierarchy, c: AllControl) -> Result<Self> {
        let main = h
            .lookup_scope(&[&"toplevel", &"main"])
            .with_context(|| "Failed to find main scope")?;
        let clk = get_var(h, &h[main], "clk")?;
        let clk = h[clk].signal_ref();
        let mut out = Self {
            cells: PrimaryMap::new(),
            groups: PrimaryMap::new(),
            invokes: PrimaryMap::new(),
            controls: PrimaryMap::new(),
            main: CellId(u32::MAX),
            clk,
            signals: vec![],
        };

        out.populate(h, c)?;
        out.build_idx();
        Ok(out)
    }

    pub fn get_signals(&self) -> Vec<SignalRef> {
        self.signals.clone()
    }

    pub fn clk(&self) -> SignalRef {
        self.clk
    }

    /// Computes the active call tree from a cycle, represented as a list of stacks (Python Petal style).
    /// values is the probe signals bitvector obtained from the cycle in question.
    pub fn compute_cycle_trace(
        &self,
        values: &BitVecValue,
    ) -> Result<Vec<Stack>> {
        let main = &self.cells[self.main];
        let (main_go, main_done) = main.probe_idxs.unwrap();
        let main_active =
            values.is_bit_set(main_go) & !values.is_bit_set(main_done);
        let mut stacks = vec![];
        if main_active {
            stacks = self.compute_cell(values, self.main, vec![]);
            stacks.sort();
            stacks.dedup();
        }
        Ok(stacks)
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
    /// Builds up all active tree paths this cycle from the cell of cell_id.
    /// prefix is the state of the stack before this particular cell.
    /// NOTE: This function is co-recursive with compute_group().
    fn compute_cell(
        &self,
        value: &BitVecValue,
        cell_id: CellId,
        mut prefix: Stack,
    ) -> Vec<Stack> {
        let cell = &self.cells[cell_id];
        if let Some((main_go_idx, main_done_idx)) = cell.probe_idxs {
            // the main component cell is the only one to have a probe_idx.
            if value.is_bit_set(main_go_idx) && !value.is_bit_set(main_done_idx)
            {
                prefix.push(cell.display_name());
            } else {
                return vec![prefix];
            }
        }
        if cell.groups.is_empty() {
            // No more children, so this is a sink.
            return vec![prefix];
        }
        let mut out = vec![];
        for &group_idx in &cell.groups {
            let group = &self.groups[group_idx];
            if value.is_bit_set(group.probe_idx) {
                let mut group_stacks =
                    self.compute_group(value, group_idx, prefix.clone());
                out.append(&mut group_stacks);
            }
        }
        // if the cell has groups but none of them are active, we still need to add the cell
        // NOTE: this would be a control cycle.
        if !cell.groups.is_empty() && out.is_empty() {
            out.push(prefix);
        }

        out
    }

    /// Builds up all active tree paths this cycle from the group of group_id.
    /// prefix is the state of the stack before this particular group.
    /// NOTE: This function is co-recursive with compute_cell(), and only called when
    /// the group is active (otherwise this function would not be called.)
    fn compute_group(
        &self,
        value: &BitVecValue,
        group_id: GroupId,
        mut prefix: Stack,
    ) -> Vec<Stack> {
        let group = &self.groups[group_id];
        prefix.push(group.display_name());
        if group.invokes.is_empty() {
            return vec![prefix];
        }
        let mut out: Vec<Stack> = vec![];
        for &invoke_id in &group.invokes {
            let mut this_thread_prefix = prefix.clone();
            let invoke = &self.invokes[invoke_id];
            if value.is_bit_set(invoke.probe_idx) {
                // the invoke probe is active
                match invoke.target {
                    InvokeTarget::Cell(target_cell_id) => {
                        // component or primitive cell activation
                        let target_cell = &self.cells[target_cell_id];
                        this_thread_prefix.push(target_cell.display_name());
                        if target_cell.is_primitive {
                            out.push(this_thread_prefix);
                        } else {
                            let mut cell_stacks = self.compute_cell(
                                value,
                                target_cell_id,
                                this_thread_prefix.clone(),
                            );
                            out.append(&mut cell_stacks);
                        }
                    }
                    InvokeTarget::Group(target_group_id) => {
                        // structural enable (group enables another group)
                        let mut group_stacks = self.compute_group(
                            value,
                            target_group_id,
                            this_thread_prefix.clone(),
                        );
                        out.append(&mut group_stacks);
                    }
                }
            }
        }
        out
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

        for (_, group) in self.groups.iter_mut() {
            group.probe_idx = to_index[&group.probe];
        }

        for (_, invoke) in self.invokes.iter_mut() {
            invoke.probe_idx = to_index[&invoke.probe];
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

    fn populate_control(
        &mut self,
        h: &Hierarchy,
        s: ScopeRef,
        c: AllControl,
        component: String,
    ) -> Result<()> {
        if let Some(ctrl_stack) = c.component_to_ctrl_stack.get(&component) {
            let component_meta_info = c.component_to_controls.get(&component).unwrap();

            for ctrl_pos in ctrl_stack {
                let meta_info = component_meta_info.get(ctrl_pos).unwrap();
                let ctrl_group_scope = get_scope(h, &h[s], &meta_info.name)?;
                let ctrl_group_go = get_var(h, &h[ctrl_group_scope], "go")?;
                // create Control node
                let ctrl_node = Control {
                    name: "".to_string(),
                    go: h[ctrl_group_go].signal_ref(),
                    invokes: Default::default(),
                    go_idx: u32::MAX,
                    pos: 0,
                    line_num: 0,
                    control_type: "".to_string(),
                };
            }
        }
        Ok(())
    }

    /// Builds the static call tree by scanning through all probes to find tree edges.
    fn populate(&mut self, h: &wellen::Hierarchy, c: AllControl) -> Result<()> {
        let main_scope = h
            .lookup_scope(&[&"toplevel", &"main"])
            .with_context(|| "Failed to find main scope")?;
        let main_go = get_var(h, &h[main_scope], "go")?;
        let main_done = get_var(h, &h[main_scope], "done")?;
        let mut main_cell = Cell {
            name: "main".to_string(),
            control: smallvec![],
            groups: smallvec![],
            probes: Some((h[main_go].signal_ref(), h[main_done].signal_ref())),
            _scope: main_scope,
            is_primitive: false,
            instances: smallvec![],
            component: String::new(),
            probe_idxs: None,
        };
        // add control nodes for main

        self.scan_probes(h, main_scope, &mut main_cell)?;
        self.main = self.cells.push(main_cell);
        Ok(())
    }

    /// Constructs the static tree from available probes.
    fn scan_probes(
        &mut self,
        h: &Hierarchy,
        cell_scope: ScopeRef,
        cell: &mut Cell,
    ) -> Result<()> {
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
                            let groupid = self.groups.push(Group {
                                name: name.to_string(),
                                probe,
                                invokes,
                                probe_idx: u32::MAX,
                            });
                            all_groups.push(groupid);
                            structurally_invoked_group_names
                                .push(name.to_string());
                            groupid
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
            if name.ends_with("_probe") {
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
                                name,
                                probe,
                                invokes,
                                probe_idx: u32::MAX,
                            });
                            cell.groups.push(groupid);
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
                            let scope = get_scope(h, &h[cell_scope], name)?;
                            let mut cell_instance = Cell {
                                name: name.to_string(),
                                groups: smallvec![],
                                control: smallvec![],
                                _scope: scope,
                                is_primitive,
                                instances: smallvec![],
                                component: String::new(),
                                probes: None,
                                probe_idxs: None,
                            };
                            self.scan_probes(h, scope, &mut cell_instance)?;
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
