use super::cidr::SPACING;
use super::commands::{BreakPointId, GroupName, PrintTuple, WatchPosition};

use crate::interpreter_ir as iir;
use crate::structures::names::{CompGroupName, GroupQIN};
use calyx::ir::Id;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Display;
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

#[derive(Debug)]
struct WatchPoint {
    id: u64,
    print_details: PrintTuple,
}

impl Display for WatchPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.  {}", self.id, self.print_details)
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
                BreakPointState::Enabled => "enabled",
                BreakPointState::Disabled => "disabled",
            }
        )
    }
}

struct GroupExecutionInfo<T: std::cmp::Eq + std::hash::Hash> {
    previous: HashSet<T>,
    current: HashSet<T>,
}

impl<T: std::cmp::Eq + std::hash::Hash> GroupExecutionInfo<T> {
    fn new() -> Self {
        Self {
            previous: HashSet::new(),
            current: HashSet::new(),
        }
    }

    fn shift_current(&mut self, current: HashSet<T>) {
        std::mem::swap(&mut self.previous, &mut self.current);
        self.current = current;
    }

    fn _in_previous(&self, key: &T) -> bool {
        self.previous.contains(key)
    }

    fn _in_current(&self, key: &T) -> bool {
        self.current.contains(key)
    }

    fn groups_new_off(&self) -> impl Iterator<Item = &T> {
        self.previous.difference(&self.current)
    }

    fn groups_new_on(&self) -> impl Iterator<Item = &T> {
        self.current.difference(&self.previous)
    }
}

pub(super) struct DebuggingContext {
    breakpoints: HashMap<CompGroupName, BreakPoint>,
    watchpoints_before:
        HashMap<CompGroupName, (BreakPointState, Vec<WatchPoint>)>,
    watchpoints_after:
        HashMap<CompGroupName, (BreakPointState, Vec<WatchPoint>)>,
    count: Counter,
    watch_count: Counter,
    group_exec_info: GroupExecutionInfo<CompGroupName>,
    // used primarially for checking if a given group exists
    comp_ctx: HashMap<Id, Rc<iir::Component>>,
    main_comp_name: Id,
}

impl DebuggingContext {
    pub fn new(ctx: &iir::ComponentCtx, main_component: &Id) -> Self {
        Self {
            count: Counter::new(),
            watch_count: Counter::new(),
            breakpoints: HashMap::new(),
            watchpoints_before: HashMap::new(),
            watchpoints_after: HashMap::new(),
            group_exec_info: GroupExecutionInfo::new(),
            main_comp_name: main_component.clone(),

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

    pub fn add_watchpoint<PT: Into<PrintTuple>>(
        &mut self,
        target: GroupName,
        position: WatchPosition,
        print: PT,
    ) {
        let key = self.parse_group_name(&target);

        let watchpoint = WatchPoint {
            id: self.watch_count.next(),
            print_details: print.into(),
        };

        match position {
            WatchPosition::Before => {
                self.watchpoints_before
                    .entry(key)
                    .or_insert((BreakPointState::Enabled, Vec::new()))
                    .1
                    .push(watchpoint);
            }
            WatchPosition::After => {
                self.watchpoints_after
                    .entry(key)
                    .or_insert((BreakPointState::Enabled, Vec::new()))
                    .1
                    .push(watchpoint);
            }
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

    pub fn remove_watchpoint(&mut self, target: &BreakPointId) {
        match target {
            BreakPointId::Name(name) => self.remove_watchpoint_by_name(name),
            BreakPointId::Number(num) => self.remove_watchpoint_by_number(*num),
        }
    }

    fn remove_breakpoint_by_name(&mut self, target: &GroupName) {
        let key = self.parse_group_name(target);

        self.breakpoints.remove(&key);
    }

    fn remove_breakpoint_by_number(&mut self, target: u64) {
        self.breakpoints.retain(|_k, x| x.id != target);
    }

    fn remove_watchpoint_by_name(&mut self, target: &GroupName) {
        let key = self.parse_group_name(target);

        self.watchpoints_before.remove(&key);
        self.watchpoints_after.remove(&key);
    }

    fn remove_watchpoint_by_number(&mut self, target: u64) {
        // TODO (Griffin): Make this less inefficient, if it becomes a problem
        // probably add a reverse lookup table or something
        for watchpoints in self
            .watchpoints_before
            .values_mut()
            .chain(self.watchpoints_after.values_mut())
        {
            watchpoints.1.retain(|x| x.id != target);
        }
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
            // TODO (Griffin): add some feedback
            breakpoint.disable()
        }
    }

    fn disable_breakpoint_by_num(&mut self, target: u64) {
        for x in self.breakpoints.values_mut() {
            if x.id == target {
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

    pub fn advance_time(&mut self, current: HashSet<GroupQIN>) {
        self.group_exec_info
            .shift_current(current.into_iter().map(|x| x.into()).collect());
    }

    pub fn set_current_time(&mut self, current: HashSet<GroupQIN>) {
        let current: HashSet<CompGroupName> =
            current.into_iter().map(|x| x.into()).collect();
        self.group_exec_info.shift_current(current.clone());
        self.group_exec_info.shift_current(current);
    }

    pub fn hit_breakpoints(&self) -> Vec<&CompGroupName> {
        self.group_exec_info
            .groups_new_on()
            .filter(|x| {
                if let Some(brk) = self.breakpoints.get(x) {
                    return brk.state.enabled();
                }
                false
            })
            .collect()
    }

    pub fn process_watchpoints(&self) -> Vec<&'_ PrintTuple> {
        let mut output_vec: Vec<_> = vec![];

        let before_iter = self.group_exec_info.groups_new_on().filter(|x| {
            if let Some((state, _)) = self.watchpoints_before.get(x) {
                return state.enabled();
            }
            false
        });

        let after_iter = self.group_exec_info.groups_new_off().filter(|x| {
            if let Some((state, _)) = self.watchpoints_after.get(x) {
                return state.enabled();
            }
            false
        });

        for target in before_iter {
            if let Some(x) = self.watchpoints_before.get(target) {
                for val in x.1.iter() {
                    output_vec.push(&val.print_details)
                }
            }
        }

        for target in after_iter {
            if let Some(x) = self.watchpoints_after.get(target) {
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

    pub fn print_watchpoints(&self) {
        println!("{}Current watchpoints:", SPACING);
        let inner_spacing = format!("{}    ", SPACING);
        let outer_spacing = format!("{}  ", SPACING);

        for (group, (_brk, watchpoints)) in self.watchpoints_before.iter() {
            println!("{}Before {}:", outer_spacing, group);
            for watchpoint in watchpoints.iter() {
                println!("{}{}", inner_spacing, watchpoint);
            }
        }

        println!();

        for (group, (_brk, watchpoints)) in self.watchpoints_after.iter() {
            if !watchpoints.is_empty() {
                println!("{}After {}:", outer_spacing, group);
                for watchpoint in watchpoints.iter() {
                    println!("{}{}", inner_spacing, watchpoint);
                }
            }
        }
    }
}
