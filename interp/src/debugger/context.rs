use super::cidr::SPACING;
use super::commands::{
    BreakPointId, ParsedGroupName, PrintTuple, WatchPosition,
};
use crate::interpreter_ir as iir;
use crate::structures::names::{CompGroupName, GroupQIN};
use calyx_ir::Id;
use owo_colors::OwoColorize;
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
    /// this breakpoint is active
    Enabled,
    /// this breakpoint is inactive
    Disabled,
    /// This breakpoint has been deleted, but has yet to be cleaned up
    Deleted,
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

    pub fn delete(&mut self) {
        self.state = BreakPointState::Deleted
    }

    pub fn is_deleted(&self) -> bool {
        matches!(self.state, BreakPointState::Deleted)
    }
}

#[derive(Debug)]
struct WatchPoint {
    id: u64,
    print_details: PrintTuple,
}

impl Display for WatchPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.  {}", self.id, self.print_details.blue().bold())
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
                BreakPointState::Deleted => "deleted",
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

enum BreakpointAction {
    Enable,
    Disable,
    Delete,
}

impl BreakpointAction {
    fn take_action(&self, breakpoint: &mut BreakPoint) {
        match self {
            BreakpointAction::Enable => breakpoint.enable(),
            BreakpointAction::Disable => breakpoint.disable(),
            BreakpointAction::Delete => breakpoint.delete(),
        }
    }

    fn take_action_with_feedback(&self, breakpoint: &mut BreakPoint) {
        self.take_action(breakpoint);
        println!(
            "{} '{}'",
            match self {
                BreakpointAction::Enable => "enabled",
                BreakpointAction::Disable => "disabled",
                BreakpointAction::Delete => "deleted",
            },
            &breakpoint.name
        )
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
            main_comp_name: *main_component,

            comp_ctx: ctx.iter().map(|x| (x.name, Rc::clone(x))).collect(),
        }
    }

    pub fn add_breakpoint<N>(&mut self, target: N)
    where
        N: ConcretizableName,
    {
        let target = target.concretize(self);
        let component_ref = self.comp_ctx.get(&target.component_name);

        if component_ref.is_none() {
            println!(
                "{} Error: there is no component named {}",
                SPACING,
                target.component_name.purple().bold()
            );
            return;
        }

        let component_ref = component_ref.unwrap();

        let group_exists = {
            let exists = component_ref.groups.find(target.group_name).is_some();
            // if there is no non-comb group, check comb groups
            if !exists {
                component_ref.comb_groups.find(target.group_name).is_some()
            } else {
                true
            }
        };

        if !group_exists {
            println!(
                "{} Error: the group {} does not exist",
                SPACING,
                target.purple().bold()
            );
            return;
        }

        if let std::collections::hash_map::Entry::Vacant(e) =
            self.breakpoints.entry(target.clone())
        {
            let br = BreakPoint {
                id: self.count.next(),
                name: target,
                state: BreakPointState::Enabled,
            };
            e.insert(br);
        } else {
            println!(
                "A breakpoint already exists for \"{}\"",
                &target.green().bold()
            )
        }
    }

    pub fn add_watchpoint<P, N>(
        &mut self,
        key: N,
        position: WatchPosition,
        print: P,
    ) where
        P: Into<PrintTuple>,
        N: ConcretizableName,
    {
        let key = key.concretize(self);
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

    fn act_breakpoint(
        &mut self,
        target: BreakPointId,
        action: BreakpointAction,
    ) {
        match target {
            BreakPointId::Name(target) => {
                let key = self.concretize_group_name(target);

                if let Some(breakpoint) = self.breakpoints.get_mut(&key) {
                    action.take_action_with_feedback(breakpoint);
                } else {
                    println!(
                        "Error: There is no breakpoint named '{}'",
                        key.red().bold().strikethrough()
                    )
                };
            }
            BreakPointId::Number(target) => {
                let mut found = false;
                for x in self.breakpoints.values_mut() {
                    if x.id == target {
                        action.take_action_with_feedback(x);
                        found = true;
                        break;
                    }
                }
                if !found {
                    println!(
                        "Error: There is no breakpoint numbered {}",
                        target.red().bold().strikethrough()
                    )
                };
            }
        }
    }

    pub fn enable_breakpoint(&mut self, target: BreakPointId) {
        self.act_breakpoint(target, BreakpointAction::Enable)
    }
    pub fn disable_breakpoint(&mut self, target: BreakPointId) {
        self.act_breakpoint(target, BreakpointAction::Disable)
    }
    pub fn remove_breakpoint(&mut self, target: BreakPointId) {
        self.act_breakpoint(target, BreakpointAction::Delete);
        self.cleanup_deleted_breakpoints()
    }

    pub fn remove_watchpoint(&mut self, target: BreakPointId) {
        match target {
            BreakPointId::Name(name) => self.remove_watchpoint_by_name(name),
            BreakPointId::Number(num) => self.remove_watchpoint_by_number(num),
        }
    }

    fn cleanup_deleted_breakpoints(&mut self) {
        self.breakpoints.retain(|_k, x| !x.is_deleted());
    }

    fn remove_watchpoint_by_name(&mut self, target: ParsedGroupName) {
        let key = self.concretize_group_name(target);

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

    #[inline]
    pub fn concretize_group_name(
        &self,
        target: ParsedGroupName,
    ) -> CompGroupName {
        target.concretize(&self.main_comp_name)
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
        target: &CompGroupName,
    ) -> bool {
        let current: HashSet<CompGroupName> =
            current_executing.into_iter().map(|x| x.into()).collect();

        current.contains(target)
    }

    pub fn print_breakpoints(&self) {
        println!("{}Current breakpoints:", SPACING);
        for breakpoint in self.breakpoints.values() {
            println!("{}{:?}", SPACING, breakpoint.red().bold())
        }
    }

    pub fn print_watchpoints(&self) {
        println!("{}Current watchpoints:", SPACING);
        let inner_spacing = format!("{}    ", SPACING);
        let outer_spacing = format!("{}  ", SPACING);

        for (group, (_brk, watchpoints)) in self.watchpoints_before.iter() {
            println!("{}Before {}:", outer_spacing, group.magenta().bold());
            for watchpoint in watchpoints.iter() {
                println!("{}{}", inner_spacing, watchpoint.magenta());
            }
        }

        println!();

        for (group, (_brk, watchpoints)) in self.watchpoints_after.iter() {
            if !watchpoints.is_empty() {
                println!("{}After {}:", outer_spacing, group.green().bold());
                for watchpoint in watchpoints.iter() {
                    println!("{}{}", inner_spacing, watchpoint.green());
                }
            }
        }
    }
}

pub(super) trait ConcretizableName {
    fn concretize(self, context: &DebuggingContext) -> CompGroupName;
}

impl ConcretizableName for CompGroupName {
    #[inline]
    fn concretize(self, _context: &DebuggingContext) -> CompGroupName {
        self
    }
}

impl ConcretizableName for ParsedGroupName {
    #[inline]
    fn concretize(self, context: &DebuggingContext) -> CompGroupName {
        context.concretize_group_name(self)
    }
}
