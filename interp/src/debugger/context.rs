use super::cidr::SPACING;
use super::parser::{BreakPointId, GroupName};

use crate::interpreter_ir as iir;
use crate::structures::names::{CompGroupName, GroupQIN};
use calyx::ir::Id;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

enum BreakPointState {
    Enabled,  // this breakpoint is active
    Disabled, // this breakpoint is inactive
    Resting,  // this breakpoint has been temporarily disabled
}

impl BreakPointState {
    pub fn enabled(&self) -> bool {
        matches!(self, BreakPointState::Enabled)
    }
}
struct BreakPoint {
    id: u64,
    name: CompGroupName, // Name of the group (may not strictly be needed)
    state: BreakPointState,
}

impl BreakPoint {
    pub fn enable(&mut self) {
        self.state = BreakPointState::Enabled;
    }

    pub fn disable(&mut self) {
        self.state = BreakPointState::Disabled;
    }
}

impl std::fmt::Debug for BreakPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.  {}  {}",
            &self.id,
            &self.name,
            match &self.state {
                BreakPointState::Resting | BreakPointState::Enabled =>
                    "enabled",
                BreakPointState::Disabled => "disabled",
            }
        )
    }
}

pub(super) struct DebuggingContext {
    breakpoints: HashMap<CompGroupName, BreakPoint>,
    count: u64,
    // used primarially for checking if a given group exists
    comp_ctx: HashMap<Id, Rc<iir::Component>>,
    main_comp_name: Id,
    /// tracks the breakpoints which are temporarially disabled
    sleeping_breakpoints: HashSet<CompGroupName>,
}

impl DebuggingContext {
    pub fn new(ctx: &iir::ComponentCtx, main_component: &Id) -> Self {
        Self {
            count: 0,
            breakpoints: HashMap::new(),
            main_comp_name: main_component.clone(),
            sleeping_breakpoints: HashSet::new(),
            comp_ctx: ctx
                .iter()
                .map(|x| (x.name.clone(), Rc::clone(x)))
                .collect(),
        }
    }

    pub fn add_breakpoint(&mut self, target: GroupName) {
        let target_name = self.parse_group_name(&target);

        if self
            .comp_ctx
            .get(&target_name.component_name)
            .map(|x| x.groups.find(&target_name.group_name))
            .flatten()
            .is_none()
        {
            println!(
                "{} Error: the group {} does not exit",
                SPACING, target_name
            );
            return;
        }

        if let std::collections::hash_map::Entry::Vacant(e) =
            self.breakpoints.entry(target_name.clone())
        {
            self.count += 1;
            let br = BreakPoint {
                id: self.count,
                name: target_name,
                state: BreakPointState::Enabled,
            };
            e.insert(br);
        } else {
            println!("A breakpoint already exists for \"{}\"", &target_name)
        }
    }

    pub fn remove_breakpoint(&mut self, target: &BreakPointId) {
        match target {
            BreakPointId::Name(name) => self.remove_breakpoint_by_name(name),
            BreakPointId::Number(num) => self.remove_breakpoint_by_number(*num),
        }
    }

    pub fn enable_breakpoint(&mut self, target: &BreakPointId) {
        match target {
            BreakPointId::Name(name) => self.enable_breakpoint_by_name(name),
            BreakPointId::Number(num) => self.enable_breakpoint_by_num(*num),
        }
    }

    pub fn disable_breakpoint(&mut self, target: &BreakPointId) {
        match target {
            BreakPointId::Name(name) => self.disable_breakpoint_by_name(name),
            BreakPointId::Number(num) => self.disable_breakpoint_by_num(*num),
        }
    }

    fn remove_breakpoint_by_name(&mut self, target: &GroupName) {
        let key = self.parse_group_name(target);

        self.breakpoints.remove(&key);
        self.sleeping_breakpoints.remove(&key);
    }

    fn remove_breakpoint_by_number(&mut self, target: u64) {
        let mut sleeping = std::mem::take(&mut self.sleeping_breakpoints);
        self.breakpoints.retain(|k, x| {
            if x.id != target {
                true
            } else {
                sleeping.remove(k);
                false
            }
        });
        self.sleeping_breakpoints = sleeping;
    }

    fn enable_breakpoint_by_name(&mut self, target: &GroupName) {
        let key = self.parse_group_name(target);

        if let Some(breakpoint) = self.breakpoints.get_mut(&key) {
            // add some feedback
            breakpoint.enable()
        }
    }

    fn enable_breakpoint_by_num(&mut self, target: u64) {
        for (_, x) in self.breakpoints.iter_mut() {
            if x.id == target {
                x.enable();
                break;
            }
        }
    }

    fn disable_breakpoint_by_name(&mut self, target: &GroupName) {
        let key = self.parse_group_name(target);

        if let Some(breakpoint) = self.breakpoints.get_mut(&key) {
            self.sleeping_breakpoints.remove(&key);
            // add some feedback
            breakpoint.disable()
        }
    }

    fn disable_breakpoint_by_num(&mut self, target: u64) {
        for x in self.breakpoints.values_mut() {
            if x.id == target {
                self.sleeping_breakpoints.remove(&x.name);
                x.disable();
                break;
            }
        }
    }

    fn parse_group_name(&self, target: &GroupName) -> CompGroupName {
        match target.len() {
            1 => CompGroupName::new(
                target[0].clone(),
                self.main_comp_name.clone(),
            ),
            2 => CompGroupName::new(target[1].clone(), target[0].clone()),
            _ => unreachable!("Something went weird in the parser"),
        }
    }

    pub fn hit_breakpoints(
        &mut self,
        current_executing: HashSet<GroupQIN>,
    ) -> Vec<CompGroupName> {
        let current: HashSet<CompGroupName> =
            current_executing.into_iter().map(|x| x.into()).collect();

        let sleeping = std::mem::take(&mut self.sleeping_breakpoints);

        // wake up sleeping breakpoints
        self.sleeping_breakpoints = sleeping
            .into_iter()
            .filter(|x| {
                if !current.contains(x) {
                    self.breakpoints.get_mut(x).unwrap().enable();
                    false
                } else {
                    true
                }
            })
            .collect();

        current
            .into_iter()
            .filter(|x| {
                if let Some(brk) = self.breakpoints.get_mut(x) {
                    if brk.state.enabled() {
                        self.sleeping_breakpoints.insert(x.clone());
                        brk.state = BreakPointState::Resting;
                        return true;
                    }
                }
                false
            })
            .collect()
    }

    pub fn print_breakpoints(&self) {
        println!("{}Current breakpoints:", SPACING);
        for breakpoint in self.breakpoints.values() {
            println!("{}{:?}", SPACING, breakpoint)
        }
    }
}
