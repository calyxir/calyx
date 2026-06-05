use anyhow::{Context, Result};

use baa::{BitVecOps, BitVecValue};
use cranelift_entity::{PrimaryMap, entity_impl};
use rustc_hash::FxHashMap;
use smallvec::{SmallVec, smallvec};
use wellen::{Hierarchy, Scope, ScopeRef, SignalRef, VarRef};

#[derive(Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct CellId(u32);
entity_impl!(CellId, "cell");

#[derive(Clone, Debug)]
/// Represents
struct Cell {
    name: String,
    groups: SmallVec<[GroupId; 6]>,
    scope: ScopeRef,
    is_primitive: bool,
    probe: Option<SignalRef>,
    probe_idx: Option<u32>,
    component: String,
    /// cells that are defined within the component
    instances: SmallVec<[CellId; 6]>,
}

impl Cell {
    pub fn display_name(&self) -> String {
        if self.is_primitive {
            format!("{} (primitive)", self.name)
        } else if (self.component == "main") {
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
    pub fn display_name(&self) -> String {
        // remove unique group identifier.
        self.name.split("UG").next().unwrap().to_string()
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct InvokeId(u32);
entity_impl!(InvokeId, "invoke");

#[derive(Debug, Clone)]
/// Represents a group invoking either a component or primitive cell.
/// TODO: Should we include structural enables here? If so, the type of target would need to change.
struct Invoke {
    name: String,
    probe: SignalRef,
    target: CellId,
    probe_idx: u32,
}

#[derive(Clone, Debug)]
/// Represents the static call tree (all possible calls).
pub struct Design {
    cells: PrimaryMap<CellId, Cell>,
    groups: PrimaryMap<GroupId, Group>,
    invokes: PrimaryMap<InvokeId, Invoke>,
    main: CellId,
    clk: SignalRef,
    signals: Vec<SignalRef>,
}

impl Design {
    pub fn new(h: &wellen::Hierarchy) -> Result<Self> {
        let main = h
            .lookup_scope(&[&"toplevel", &"main"])
            .with_context(|| format!("Failed to find main scope"))?;
        let clk = get_var(h, &h[main], "clk")?;
        let clk = h[clk].signal_ref();
        let mut out = Self {
            cells: PrimaryMap::new(),
            groups: PrimaryMap::new(),
            invokes: PrimaryMap::new(),
            main: CellId(u32::MAX),
            clk,
            signals: vec![],
        };
        out.populate(h)?;
        out.build_idx();
        Ok(out)
    }

    pub fn get_signals(&self) -> Vec<SignalRef> {
        self.signals.clone()
    }

    pub fn clk(&self) -> SignalRef {
        self.clk
    }

    pub fn compute(&self, values: &BitVecValue) -> Result<Vec<Stack>> {
        let main = &self.cells[self.main];
        let main_active = values.is_bit_set(main.probe_idx.unwrap());
        let mut stacks = vec![];
        if main_active {
            stacks = self.compute_cell(values, self.main, vec![]);
            stacks.sort();
            stacks.dedup();
        }
        Ok(stacks)
    }
}

pub fn parse_probe_name(name: &str) -> Result<ProbeName> {
    let pat = "___";
    if let Some(prefix) = name.strip_suffix("_group_probe") {
        // invoke2UG___main_group_probe
        let mut parts = prefix.split(pat);
        // let group = parts.next().unwrap().split("UG").next().unwrap();
        let group = parts.next().unwrap();
        let component = parts.next().unwrap();
        Ok(ProbeName::Group { group, component })
    } else if let Some(prefix) = name.strip_suffix("_cell_probe") {
        // mac___invoke2UG___main_cell_probe
        let mut parts = prefix.split(pat);
        let cell = parts.next().unwrap();
        // let group = parts.next().unwrap().split("UG").next().unwrap();
        let group = parts.next().unwrap();
        let component = parts.next().unwrap();
        Ok(ProbeName::InvokeCell {
            name: cell,
            group,
            component,
        })
    } else if let Some(prefix) = name.strip_suffix("_primitive_probe") {
        //lt0___in_rangeUG___main_primitive_probe
        let mut parts = prefix.split(pat);
        let primitive = parts.next().unwrap();
        // let group = parts.next().unwrap().split("UG").next().unwrap();
        let group = parts.next().unwrap();
        let component = parts.next().unwrap();
        Ok(ProbeName::InvokePrimitive {
            name: primitive,
            group,
            component,
        })
    } else {
        anyhow::bail!("failed to parse {name}")
    }
}

// trying to build up the same thing that we have in pypetal for now.
pub type Stack = Vec<String>;

impl Design {
    fn compute_group(
        &self,
        value: &BitVecValue,
        group_id: GroupId,
        mut prefix: Stack,
    ) -> Vec<Stack> {
        // assumes that the group is active (otherwise this function would not be called.)
        let group = &self.groups[group_id];
        prefix.push(group.display_name());
        if group.invokes.is_empty() {
            return vec![prefix];
        }
        let mut out: Vec<Stack> = vec![];
        for &invoke_id in &group.invokes {
            let mut this_thread_prefix = prefix.clone();
            let invoke = &self.invokes[invoke_id];
            let target_cell_id = invoke.target;
            let target_cell = &self.cells[target_cell_id];
            if value.is_bit_set(invoke.probe_idx) {
                // the invoke probe is active
                this_thread_prefix.push(target_cell.display_name());
                if target_cell.is_primitive {
                    // println!("Primitive {}", target_cell.name);
                    out.push(this_thread_prefix);
                } else {
                    // println!(
                    //     "Cell {} [{}]",
                    //     target_cell.name, target_cell.component
                    // );
                    let mut cell_stack = self.compute_cell(
                        value,
                        target_cell_id,
                        this_thread_prefix.clone(),
                    );
                    out.append(&mut cell_stack);
                }
            }
        }
        out
    }

    fn compute_cell(
        &self,
        value: &BitVecValue,
        cell_id: CellId,
        mut prefix: Stack,
    ) -> Vec<Stack> {
        let cell = &self.cells[cell_id];
        if let Some(idx) = cell.probe_idx {
            // the main component cell is the only one to have a probe_idx.
            if value.is_bit_set(idx) {
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
        // NOTE: this is a control cycle.
        if !cell.groups.is_empty() && out.is_empty() {
            out.push(prefix);
        }

        out
    }

    fn build_idx(&mut self) {
        self.signals = self.signals();
        let to_index = FxHashMap::from_iter(
            self.signals
                .iter()
                .enumerate()
                .map(|(idx, &signal)| (signal, idx as u32)),
        );

        for (_, cell) in self.cells.iter_mut() {
            if let Some(p) = cell.probe {
                cell.probe_idx = Some(to_index[&p]);
            }
        }

        for (_, group) in self.groups.iter_mut() {
            group.probe_idx = to_index[&group.probe];
        }

        for (_, invoke) in self.invokes.iter_mut() {
            invoke.probe_idx = to_index[&invoke.probe];
        }
    }

    fn signals(&self) -> Vec<SignalRef> {
        let mut signals = vec![];
        for cell in self.cells.values() {
            if let Some(p) = cell.probe {
                signals.push(p);
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

    fn populate(&mut self, h: &wellen::Hierarchy) -> Result<()> {
        let main_scope = h
            .lookup_scope(&[&"toplevel", &"main"])
            .with_context(|| format!("Failed to find main scope"))?;
        let main_go = get_var(h, &h[main_scope], "go")?;
        let mut main_cell = Cell {
            name: "main".to_string(),
            groups: smallvec![],
            probe: Some(h[main_go].signal_ref()),
            scope: main_scope,
            is_primitive: false,
            instances: smallvec![],
            component: String::new(),
            probe_idx: None,
        };
        self.scan_probes(h, main_scope, &mut main_cell)?;
        self.main = self.cells.push(main_cell);

        Ok(())
    }

    fn scan_probes(
        &mut self,
        h: &Hierarchy,
        cell_scope: ScopeRef,
        cell: &mut Cell,
    ) -> Result<()> {
        let mut parentless_invokes: Vec<(&str, InvokeId)> = vec![];
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
                        let groupid = self.groups.push(Group {
                            name,
                            probe,
                            invokes,
                            probe_idx: u32::MAX,
                        });
                        cell.groups.push(groupid);
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
                        let is_primitive = matches!(
                            probe_name,
                            ProbeName::InvokePrimitive { .. }
                        );
                        assert!(
                            cell.component.is_empty()
                                || cell.component == component
                        );
                        if cell.component.is_empty() {
                            cell.component = component.to_string();
                        }
                        let maybe_group = cell
                            .groups
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
                            let scope = get_scope(h, &h[cell_scope], &name)?;
                            let mut cell_instance = Cell {
                                name: name.to_string(),
                                groups: smallvec![],
                                scope,
                                is_primitive,
                                instances: smallvec![],
                                component: String::new(),
                                probe: None,
                                probe_idx: None,
                            };
                            self.scan_probes(h, scope, &mut cell_instance)?;
                            let cell_id = self.cells.push(cell_instance);
                            cell.instances.push(cell_id);
                            cell_id
                        };
                        let invoke_id = self.invokes.push(Invoke {
                            name: name.to_string(),
                            probe,
                            target,
                            probe_idx: u32::MAX,
                        });
                        if let Some(&group) = maybe_group {
                            self.groups[group].invokes.push(invoke_id);
                        } else {
                            parentless_invokes.push((group, invoke_id))
                        }
                    }
                }
            }
        }
        assert!(parentless_invokes.is_empty());
        Ok(())
    }
}

pub fn get_var(h: &wellen::Hierarchy, s: &Scope, name: &str) -> Result<VarRef> {
    s.vars(h)
        .find(|&v| h[v].name(h) == name)
        .with_context(|| format!("Failed to find {name} in {}", s.full_name(h)))
}

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_probe_name() {
        assert_eq!(
            parse_probe_name("invoke2UG___main_group_probe").unwrap(),
            ProbeName::Group {
                group: "invoke2",
                component: "main"
            }
        );
        assert_eq!(
            parse_probe_name("mac___invoke2UG___main_cell_probe").unwrap(),
            ProbeName::InvokeCell {
                name: "mac",
                group: "invoke2",
                component: "main"
            }
        );
        assert_eq!(
            parse_probe_name("lt0___in_rangeUG___main_primitive_probe")
                .unwrap(),
            ProbeName::InvokePrimitive {
                name: "lt0",
                group: "in_range",
                component: "main"
            }
        );
    }
}
