use crate::{
    debugger::commands::BreakpointID, flatten::flat_ir::prelude::GroupIdx,
};

use super::super::{
    cidr::SPACING,
    commands::{
        GroupName, ParsedBreakPointID, ParsedGroupName, PrintTuple,
        WatchPosition,
    },
};

use calyx_ir::Id;
use owo_colors::OwoColorize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Display;
use std::rc::Rc;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
enum BreakPointStatus {
    /// this breakpoint is active
    Enabled,
    /// this breakpoint is inactive
    Disabled,
    /// This breakpoint has been deleted, but has yet to be cleaned up
    Deleted,
}

impl BreakPointStatus {
    pub fn enabled(&self) -> bool {
        matches!(self, BreakPointStatus::Enabled)
    }
}

#[derive(Clone)]
struct BreakPoint {
    id: u64,
    group: GroupIdx,
    state: BreakPointStatus,
}

impl BreakPoint {
    pub fn enable(&mut self) {
        self.state = BreakPointStatus::Enabled;
    }

    pub fn disable(&mut self) {
        self.state = BreakPointStatus::Disabled;
    }

    pub fn delete(&mut self) {
        self.state = BreakPointStatus::Deleted
    }

    pub fn is_deleted(&self) -> bool {
        matches!(self.state, BreakPointStatus::Deleted)
    }
}

#[derive(Debug, Clone)]
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
            "{}.  {:?}  {}",
            &self.id,
            &self.group,
            match &self.state {
                BreakPointStatus::Enabled => "enabled",
                BreakPointStatus::Disabled => "disabled",
                BreakPointStatus::Deleted => "deleted",
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
            "{} '{:?}'",
            match self {
                BreakpointAction::Enable => "enabled",
                BreakpointAction::Disable => "disabled",
                BreakpointAction::Delete => "deleted",
            },
            &breakpoint.group
        )
    }
}

#[derive(Debug, Clone)]
pub(super) struct DebuggingContext {
    breakpoints: HashMap<GroupIdx, BreakPoint>,
    watchpoints_before: HashMap<GroupIdx, (BreakPointStatus, Vec<WatchPoint>)>,
    watchpoints_after: HashMap<GroupIdx, (BreakPointStatus, Vec<WatchPoint>)>,
    break_count: Counter,
    watch_count: Counter,
}

impl DebuggingContext {
    pub fn new() -> Self {
        Self {
            break_count: Counter::new(),
            watch_count: Counter::new(),
            breakpoints: HashMap::new(),
            watchpoints_before: HashMap::new(),
            watchpoints_after: HashMap::new(),
        }
    }

    pub fn add_breakpoint<N>(&mut self, target: GroupIdx) {
        if let std::collections::hash_map::Entry::Vacant(e) =
            self.breakpoints.entry(target)
        {
            let br = BreakPoint {
                id: self.break_count.next(),
                group: target,
                state: BreakPointStatus::Enabled,
            };
            e.insert(br);
        } else {
            println!("A breakpoint already exists",)
        }
    }

    pub fn add_watchpoint<P>(
        &mut self,
        group: GroupIdx,
        position: WatchPosition,
        print: P,
    ) where
        P: Into<PrintTuple>,
    {
        let map = match position {
            WatchPosition::Before => &mut self.watchpoints_before,
            WatchPosition::After => &mut self.watchpoints_after,
        };

        map.entry(group)
            .or_insert((BreakPointStatus::Enabled, Vec::with_capacity(1)))
            .1
            .push(WatchPoint {
                id: self.watch_count.next(),
                print_details: print.into(),
            });
    }

    fn act_breakpoint(
        &mut self,
        target: BreakpointID,
        action: BreakpointAction,
    ) {
        match target {
            BreakpointID::Name(target) => {
                if let Some(breakpoint) = self.breakpoints.get_mut(&target) {
                    action.take_action_with_feedback(breakpoint);
                } else {
                    println!(
                        "Error: There is no breakpoint named '{:?}'",
                        target.red().bold().strikethrough()
                    )
                };
            }
            BreakpointID::Number(target) => {
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

    pub fn enable_breakpoint(&mut self, target: BreakpointID) {
        self.act_breakpoint(target, BreakpointAction::Enable)
    }
    pub fn disable_breakpoint(&mut self, target: BreakpointID) {
        self.act_breakpoint(target, BreakpointAction::Disable)
    }
    pub fn remove_breakpoint(&mut self, target: BreakpointID) {
        self.act_breakpoint(target, BreakpointAction::Delete);
        self.cleanup_deleted_breakpoints()
    }

    pub fn remove_watchpoint(&mut self, target: BreakpointID) {
        match target {
            BreakpointID::Name(name) => self.remove_watchpoint_by_name(name),
            BreakpointID::Number(num) => self.remove_watchpoint_by_number(num),
        }
    }

    fn cleanup_deleted_breakpoints(&mut self) {
        self.breakpoints.retain(|_k, x| !x.is_deleted());
    }

    fn remove_watchpoint_by_name(&mut self, target: GroupIdx) {
        self.watchpoints_before.remove(&target);
        self.watchpoints_after.remove(&target);
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

    pub fn hit_breakpoints(&self) -> Vec<&GroupName> {
        // self.group_exec_info
        //     .groups_new_on()
        //     .filter(|x| {
        //         if let Some(brk) = self.breakpoints.get(x) {
        //             return brk.state.enabled();
        //         }
        //         false
        //     })
        //     .collect()
        todo!()
    }

    pub fn process_watchpoints(&self) -> Vec<&'_ PrintTuple> {
        // let mut output_vec: Vec<_> = vec![];

        // let before_iter = self.group_exec_info.groups_new_on().filter(|x| {
        //     if let Some((state, _)) = self.watchpoints_before.get(x) {
        //         return state.enabled();
        //     }
        //     false
        // });

        // let after_iter = self.group_exec_info.groups_new_off().filter(|x| {
        //     if let Some((state, _)) = self.watchpoints_after.get(x) {
        //         return state.enabled();
        //     }
        //     false
        // });

        // for target in before_iter {
        //     if let Some(x) = self.watchpoints_before.get(target) {
        //         for val in x.1.iter() {
        //             output_vec.push(&val.print_details)
        //         }
        //     }
        // }

        // for target in after_iter {
        //     if let Some(x) = self.watchpoints_after.get(target) {
        //         for val in x.1.iter() {
        //             output_vec.push(&val.print_details)
        //         }
        //     }
        // }

        // output_vec

        todo!()
    }

    pub fn is_group_running(
        &self,
        current_executing: HashSet<GroupIdx>,
        target: &GroupName,
    ) -> bool {
        // let current: HashSet<GroupName> =
        //     current_executing.into_iter().map(|x| x.into()).collect();

        // current.contains(target)
        todo!()
    }

    pub fn print_breakpoints(&self) {
        // println!("{}Current breakpoints:", SPACING);
        // for breakpoint in self.breakpoints.values() {
        //     println!("{}{:?}", SPACING, breakpoint.red().bold())
        // }

        todo!()
    }

    pub fn print_watchpoints(&self) {
        todo!()
        // println!("{}Current watchpoints:", SPACING);
        // let inner_spacing = format!("{}    ", SPACING);
        // let outer_spacing = format!("{}  ", SPACING);

        // for (group, (_brk, watchpoints)) in self.watchpoints_before.iter() {
        //     println!("{}Before {}:", outer_spacing, group.magenta().bold());
        //     for watchpoint in watchpoints.iter() {
        //         println!("{}{}", inner_spacing, watchpoint.magenta());
        //     }
        // }

        // println!();

        // for (group, (_brk, watchpoints)) in self.watchpoints_after.iter() {
        //     if !watchpoints.is_empty() {
        //         println!("{}After {}:", outer_spacing, group.green().bold());
        //         for watchpoint in watchpoints.iter() {
        //             println!("{}{}", inner_spacing, watchpoint.green());
        //         }
        //     }
        // }
    }
}
