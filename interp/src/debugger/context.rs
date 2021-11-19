use super::cidr::{PrintMode, SPACING};
use super::commands::{BreakPointId, GroupName};
use super::PrintCode;

use crate::interpreter_ir as iir;
use crate::structures::names::{CompGroupName, GroupQIN};
use calyx::ir::Id;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

pub struct Counter(u64);

impl Counter {
    pub fn next(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }
    pub fn new() -> Self {
        Self(0)
    }
}

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

type PrintTuple = (Option<Vec<Vec<Id>>>, Option<PrintCode>, PrintMode);

struct WatchPoint {
    _id: u64,
    _name: CompGroupName,
    print_details: PrintTuple,
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
    watchpoints: HashMap<CompGroupName, (BreakPointState, Vec<WatchPoint>)>,
    count: Counter,
    watch_count: Counter,
    // used primarially for checking if a given group exists
    comp_ctx: HashMap<Id, Rc<iir::Component>>,
    main_comp_name: Id,
    /// tracks the breakpoints which are temporarially disabled
    sleeping_breakpoints: HashSet<CompGroupName>,
    sleeping_watchpoints: HashSet<CompGroupName>,
}

impl DebuggingContext {
    pub fn new(ctx: &iir::ComponentCtx, main_component: &Id) -> Self {
        Self {
            count: Counter::new(),
            watch_count: Counter::new(),
            breakpoints: HashMap::new(),
            watchpoints: HashMap::new(),
            main_comp_name: main_component.clone(),
            sleeping_breakpoints: HashSet::new(),
            comp_ctx: ctx
                .iter()
                .map(|x| (x.name.clone(), Rc::clone(x)))
                .collect(),
            sleeping_watchpoints: HashSet::new(),
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
            let br = BreakPoint {
                id: self.count.next(),
                name: target_name,
                state: BreakPointState::Enabled,
            };
            e.insert(br);
        } else {
            println!("A breakpoint already exists for \"{}\"", &target_name)
        }
    }

    pub fn add_watchpoint(&mut self, target: GroupName, print: PrintTuple) {
        let key = self.parse_group_name(&target);

        let watchpoint = WatchPoint {
            _id: self.watch_count.next(),
            _name: key.clone(),
            print_details: print,
        };

        self.watchpoints
            .entry(key)
            .or_insert((BreakPointState::Enabled, Vec::new()))
            .1
            .push(watchpoint);
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

    pub fn process_watchpoints(
        &mut self,
        current_executing: HashSet<GroupQIN>,
    ) -> Vec<&'_ PrintTuple> {
        let current: HashSet<CompGroupName> =
            current_executing.into_iter().map(|x| x.into()).collect();

        let sleeping = std::mem::take(&mut self.sleeping_watchpoints);

        // wake up sleeping breakpoints
        self.sleeping_watchpoints = sleeping
            .into_iter()
            .filter(|x| {
                if !current.contains(x) {
                    self.watchpoints
                        .get_mut(x)
                        .map(|x| x.0 = BreakPointState::Enabled);
                    false
                } else {
                    true
                }
            })
            .collect();

        let hit: Vec<_> = current
            .into_iter()
            .filter(|x| {
                if let Some((state, _)) = self.watchpoints.get_mut(x) {
                    if state.enabled() {
                        self.sleeping_watchpoints.insert(x.clone());
                        *state = BreakPointState::Resting;
                        return true;
                    }
                }
                false
            })
            .collect();

        let mut output_vec: Vec<_> = vec![];
        for target in hit {
            if let Some(x) = self.watchpoints.get(&target) {
                for val in x.1.iter() {
                    output_vec.push(&val.print_details)
                }
            }
        }
        output_vec
    }

    pub fn is_group_running(
        &self,
        current_executing: HashSet<GroupQIN>,
        target: &GroupName,
    ) -> bool {
        let current: HashSet<CompGroupName> =
            current_executing.into_iter().map(|x| x.into()).collect();

        let target = self.parse_group_name(target);
        current.contains(&target)
    }

    pub fn print_breakpoints(&self) {
        println!("{}Current breakpoints:", SPACING);
        for breakpoint in self.breakpoints.values() {
            println!("{}{:?}", SPACING, breakpoint)
        }
    }
}
