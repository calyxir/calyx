use crate::{
    debugger::commands::{BreakpointID, BreakpointIdx, WatchID, WatchpointIdx},
    flatten::{
        flat_ir::prelude::GroupIdx,
        structures::{index_trait::impl_index, indexed_map::IndexedMap},
    },
};

use super::super::{
    cidr::SPACING,
    commands::{
        GroupName, ParsedBreakPointID, ParsedGroupName, PrintTuple,
        WatchPosition,
    },
};

use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use calyx_ir::Id;
use owo_colors::OwoColorize;
use smallvec::{smallvec, SmallVec};

use std::fmt::Display;
use std::rc::Rc;

#[derive(Debug, Clone)]
enum PointStatus {
    /// this breakpoint is active
    Enabled,
    /// this breakpoint is inactive
    Disabled,
}

impl PointStatus {
    pub fn enabled(&self) -> bool {
        matches!(self, PointStatus::Enabled)
    }
}

#[derive(Clone, Debug)]
struct BreakPoint {
    group: GroupIdx,
    state: PointStatus,
}

impl BreakPoint {
    pub fn enable(&mut self) {
        self.state = PointStatus::Enabled;
    }

    pub fn disable(&mut self) {
        self.state = PointStatus::Disabled;
    }

    pub fn is_disabled(&self) -> bool {
        matches!(self.state, PointStatus::Disabled)
    }
}

#[derive(Debug, Clone)]
struct WatchPoint {
    group: GroupIdx,
    state: PointStatus,
    print_details: PrintTuple,
}

impl WatchPoint {
    pub fn enable(&mut self) {
        self.state = PointStatus::Enabled;
    }

    pub fn disable(&mut self) {
        self.state = PointStatus::Disabled;
    }

    pub fn is_disabled(&self) -> bool {
        matches!(self.state, PointStatus::Disabled)
    }
}

// impl Display for WatchPoint {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}.  {}", self.id, self.print_details.blue().bold())
//     }
// }

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

#[derive(Debug, Clone, Copy)]
enum PointAction {
    Enable,
    Disable,
}

impl PointAction {
    fn take_action(&self, breakpoint: &mut BreakPoint) {
        match self {
            PointAction::Enable => breakpoint.enable(),
            PointAction::Disable => breakpoint.disable(),
        }
    }

    fn take_action_with_feedback(&self, breakpoint: &mut BreakPoint) {
        self.take_action(breakpoint);
        println!(
            "{} '{:?}'",
            match self {
                PointAction::Enable => "enabled",
                PointAction::Disable => "disabled",
            },
            &breakpoint.group
        )
    }
}

#[derive(Debug)]
struct BreakpointMap {
    group_idx_map: HashMap<GroupIdx, BreakpointIdx>,
    breakpoints: HashMap<BreakpointIdx, BreakPoint>,
    breakpoint_counter: IndexedMap<BreakpointIdx, ()>,
}

impl BreakpointMap {
    fn new() -> Self {
        Self {
            group_idx_map: HashMap::new(),
            breakpoints: HashMap::new(),
            breakpoint_counter: IndexedMap::new(),
        }
    }

    fn insert(&mut self, breakpoint: BreakPoint) {
        let idx = self.breakpoint_counter.next_key();
        self.group_idx_map.insert(breakpoint.group, idx);
        self.breakpoints.insert(idx, breakpoint);
    }

    fn get_by_idx(&self, idx: BreakpointIdx) -> Option<&BreakPoint> {
        self.breakpoints.get(&idx)
    }

    fn get_by_group(&self, group: GroupIdx) -> Option<&BreakPoint> {
        self.group_idx_map
            .get(&group)
            .and_then(|idx| self.get_by_idx(*idx))
    }

    fn get_by_group_mut(&mut self, group: GroupIdx) -> Option<&mut BreakPoint> {
        self.group_idx_map
            .get(&group)
            .and_then(|idx| self.breakpoints.get_mut(idx))
    }

    fn get_by_idx_mut(
        &mut self,
        idx: BreakpointIdx,
    ) -> Option<&mut BreakPoint> {
        self.breakpoints.get_mut(&idx)
    }

    fn breakpoint_exists(&self, group: GroupIdx) -> bool {
        self.group_idx_map.contains_key(&group)
    }

    fn delete_by_idx(&mut self, idx: BreakpointIdx) {
        let br = self.breakpoints.remove(&idx);
        if let Some(br) = br {
            self.group_idx_map.remove(&br.group);
        }
    }

    fn delete_by_group(&mut self, group: GroupIdx) {
        if let Some(idx) = self.group_idx_map.remove(&group) {
            self.breakpoints.remove(&idx);
        }
    }
}

#[derive(Debug)]
enum WatchPointIndices {
    Before(SmallVec<[WatchpointIdx; 8]>),
    After(SmallVec<[WatchpointIdx; 8]>),
    Both {
        before: SmallVec<[WatchpointIdx; 4]>,
        after: SmallVec<[WatchpointIdx; 4]>,
    },
}

impl WatchPointIndices {
    fn insert_before(&mut self, idx: WatchpointIdx) {
        match self {
            Self::Before(b) => b.push(idx),
            Self::Both { before: b, .. } => b.push(idx),
            Self::After(aft) => {
                *self = Self::Both {
                    before: smallvec![idx],
                    after: SmallVec::from_iter(aft.drain(..)),
                }
            }
        }
    }

    fn insert_after(&mut self, idx: WatchpointIdx) {
        match self {
            Self::Before(bef) => {
                *self = Self::Both {
                    before: SmallVec::from_iter(bef.drain(..)),
                    after: smallvec![idx],
                }
            }
            Self::After(a) => a.push(idx),
            Self::Both { after: a, .. } => a.push(idx),
        }
    }

    fn get_before(&self) -> Option<&[WatchpointIdx]> {
        match self {
            Self::Before(idx) => Some(&idx),
            Self::Both { after, .. } => Some(&after),
            Self::After(_) => None,
        }
    }

    fn get_after(&self) -> Option<&[WatchpointIdx]> {
        match self {
            Self::Before(_) => None,
            Self::After(idx) => Some(&idx),
            Self::Both { before, .. } => Some(&before),
        }
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &WatchpointIdx> + '_> {
        match self {
            Self::Before(idx) => Box::new(idx.iter()),
            Self::After(idx) => Box::new(idx.iter()),
            Self::Both { before, after } => {
                Box::new(before.iter().chain(after.iter()))
            }
        }
    }
}

#[derive(Debug)]
struct WatchpointMap {
    group_idx_map: HashMap<GroupIdx, WatchPointIndices>,
    watchpoints_before: HashMap<WatchpointIdx, WatchPoint>,
    watchpoints_after: HashMap<WatchpointIdx, WatchPoint>,
    watchpoint_counter: IndexedMap<WatchpointIdx, ()>,
}

impl WatchpointMap {
    fn new() -> Self {
        Self {
            group_idx_map: HashMap::new(),
            watchpoints_before: HashMap::new(),
            watchpoints_after: HashMap::new(),
            watchpoint_counter: IndexedMap::new(),
        }
    }

    fn insert(&mut self, watchpoint: WatchPoint, position: WatchPosition) {
        let idx = self.watchpoint_counter.next_key();
        if let Some(current) = self.group_idx_map.get_mut(&watchpoint.group) {
            match position {
                WatchPosition::Before => current.insert_before(idx),
                WatchPosition::After => current.insert_after(idx),
            }
        } else {
            self.group_idx_map.insert(
                watchpoint.group,
                match position {
                    WatchPosition::Before => {
                        WatchPointIndices::Before(smallvec![idx])
                    }
                    WatchPosition::After => {
                        WatchPointIndices::After(smallvec![idx])
                    }
                },
            );
        }

        match position {
            WatchPosition::Before => {
                self.watchpoints_before.insert(idx, watchpoint)
            }
            WatchPosition::After => {
                self.watchpoints_after.insert(idx, watchpoint)
            }
        };
    }

    fn get_by_idx(&self, idx: WatchpointIdx) -> Option<&WatchPoint> {
        self.watchpoints_before
            .get(&idx)
            .or_else(|| self.watchpoints_after.get(&idx))
    }

    fn get_by_group(&self, group: GroupIdx) -> Option<&WatchPointIndices> {
        self.group_idx_map.get(&group)
    }

    fn get_by_group_mut(
        &mut self,
        group: GroupIdx,
    ) -> Option<&mut WatchPointIndices> {
        self.group_idx_map.get_mut(&group)
    }

    fn get_by_idx_mut(
        &mut self,
        idx: WatchpointIdx,
    ) -> Option<&mut WatchPoint> {
        self.watchpoints_before
            .get_mut(&idx)
            .or_else(|| self.watchpoints_after.get_mut(&idx))
    }

    fn delete_by_idx(&mut self, idx: WatchpointIdx) {
        let point = self
            .watchpoints_before
            .remove(&idx)
            .or_else(|| self.watchpoints_after.remove(&idx));

        if let Some(point) = point {
            self.group_idx_map
                .get_mut(&point.group)
                .map(|idxs| match idxs {
                    WatchPointIndices::Before(b) => b.retain(|i| *i != idx),
                    WatchPointIndices::After(a) => a.retain(|i| *i != idx),
                    WatchPointIndices::Both { before, after } => {
                        before.retain(|i| *i != idx);
                        after.retain(|i| *i != idx);
                    }
                });
        }
    }

    fn delete_by_group(&mut self, group: GroupIdx) {
        if let Some(idx) = self.group_idx_map.remove(&group) {
            match idx {
                WatchPointIndices::Before(before) => {
                    for point in before {
                        self.watchpoints_before.remove(&point);
                    }
                }
                WatchPointIndices::After(after) => {
                    for point in after {
                        self.watchpoints_after.remove(&point);
                    }
                }
                WatchPointIndices::Both { before, after } => {
                    for point in before {
                        self.watchpoints_before.remove(&point);
                    }
                    for point in after {
                        self.watchpoints_after.remove(&point);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct DebuggingContext {
    breakpoints: BreakpointMap,
    watchpoints: WatchpointMap,
}

impl DebuggingContext {
    pub fn new() -> Self {
        Self {
            breakpoints: BreakpointMap::new(),
            watchpoints: WatchpointMap::new(),
        }
    }

    pub fn add_breakpoint<N>(&mut self, target: GroupIdx) {
        if !self.breakpoints.breakpoint_exists(target) {
            let br = BreakPoint {
                group: target,
                state: PointStatus::Enabled,
            };
            self.breakpoints.insert(br)
        } else {
            print!("A breakpoint already exists for this group",);
            let br = self.breakpoints.get_by_group_mut(target).unwrap();
            if br.is_disabled() {
                br.enable();
                println!(" but it was disabled. It has been re-enabled.");
            } else {
                println!(".");
            }
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
        let watchpoint = WatchPoint {
            group,
            state: PointStatus::Enabled,
            print_details: print.into(),
        };
        // TODO griffin: Check if watchpoint already exists and avoid adding duplicates
        self.watchpoints.insert(watchpoint, position);
    }

    fn act_breakpoint(&mut self, target: BreakpointID, action: PointAction) {
        let target_opt = match target {
            BreakpointID::Name(group) => {
                self.breakpoints.get_by_group_mut(group)
            }
            BreakpointID::Number(idx) => self.breakpoints.get_by_idx_mut(idx),
        };

        if let Some(breakpoint) = target_opt {
            match action {
                PointAction::Enable => {
                    breakpoint.enable();
                }
                PointAction::Disable => {
                    breakpoint.disable();
                }
            }
        } else if matches!(target, BreakpointID::Name(_)) {
            let name = target.as_name().unwrap();
            println!(
                "Error: There is no breakpoint named '{:?}'",
                name.red().bold().strikethrough()
            )
        } else {
            let num = target.as_number().unwrap();
            println!(
                "Error: There is no breakpoint numbered {}",
                num.red().bold().strikethrough()
            )
        }
    }

    pub fn enable_breakpoint(&mut self, target: BreakpointID) {
        self.act_breakpoint(target, PointAction::Enable)
    }
    pub fn disable_breakpoint(&mut self, target: BreakpointID) {
        self.act_breakpoint(target, PointAction::Disable)
    }
    pub fn remove_breakpoint(&mut self, target: BreakpointID) {
        match target {
            BreakpointID::Name(name) => self.breakpoints.delete_by_group(name),
            BreakpointID::Number(num) => self.breakpoints.delete_by_idx(num),
        }
    }

    pub fn remove_watchpoint(&mut self, target: WatchID) {
        match target {
            WatchID::Name(name) => self.remove_watchpoint_by_name(name),
            WatchID::Number(num) => self.remove_watchpoint_by_number(num),
        }
    }

    fn remove_watchpoint_by_name(&mut self, target: GroupIdx) {
        self.watchpoints.delete_by_group(target);
    }

    fn remove_watchpoint_by_number(&mut self, target: WatchpointIdx) {
        self.watchpoints.delete_by_idx(target)
    }

    fn act_watchpoint(&mut self, target: WatchID, action: PointAction) {
        fn act(target: &mut WatchPoint, action: PointAction) {
            match action {
                PointAction::Enable => {
                    target.enable();
                }
                PointAction::Disable => {
                    target.disable();
                }
            }
        }

        match target {
            WatchID::Name(name) => {
                if let Some(points) = self.watchpoints.get_by_group_mut(name) {
                    // mutability trickery
                    let points_actual = std::mem::replace(
                        points,
                        WatchPointIndices::Before(SmallVec::new()),
                    );

                    for point_idx in points_actual.iter() {
                        if let Some(point) =
                            self.watchpoints.get_by_idx_mut(*point_idx)
                        {
                            act(point, action);
                        }
                    }

                    *self.watchpoints.get_by_group_mut(name).unwrap() =
                        points_actual;
                } else {
                    println!(
                        "Error: There are no watchpoints for specified group",
                    )
                }
            }
            WatchID::Number(num) => {
                if let Some(point) = self.watchpoints.get_by_idx_mut(num) {
                    act(point, action);
                } else {
                    println!(
                        "Error: There is no watchpoint numbered {}",
                        num.red().bold().strikethrough()
                    )
                }
            }
        }
    }

    pub fn enable_watchpoint(&mut self, target: WatchID) {
        self.act_watchpoint(target, PointAction::Enable)
    }

    pub fn disable_watchpoint(&mut self, target: WatchID) {
        self.act_watchpoint(target, PointAction::Disable)
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
