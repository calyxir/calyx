use super::super::{
    commands::{PrintTuple, WatchPosition},
    debugger_core::SPACING,
};
use crate::flatten::{structures::context::LookupName, text_utils::Color};
use crate::{
    debugger::commands::{BreakpointID, BreakpointIdx, WatchID, WatchpointIdx},
    flatten::{
        flat_ir::prelude::{Control, ControlIdx, GroupIdx},
        structures::{context::Context, environment::Environment},
    },
};
use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use cider_idx::maps::IndexedMap;
use itertools::Itertools;
use smallvec::{SmallVec, smallvec};
use std::fmt::Display;

#[derive(Debug, Clone)]
enum PointStatus {
    /// this breakpoint is active
    Enabled,
    /// this breakpoint is inactive
    Disabled,
}

impl Display for PointStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PointStatus::Enabled => {
                write!(f, "{}", "enabled".stylize_breakpoint_enabled())
            }
            PointStatus::Disabled => {
                write!(f, "{}", "disabled".stylize_breakpoint_disabled())
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct BreakPoint {
    control: ControlIdx,
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

    pub fn is_enabled(&self) -> bool {
        matches!(self.state, PointStatus::Enabled)
    }

    pub fn format(&self, ctx: &Context) -> String {
        let control = &ctx.primary.control[self.control].control;

        // Get parent
        let parent_comp = ctx.lookup_control_definition(self.control);
        let parent_name = ctx.lookup_name(parent_comp);

        match control {
            // Group
            Control::Enable(enable) => {
                let group = enable.group();
                let group_name = ctx.lookup_name(group);
                format!("{parent_name}::{group_name}: {}", self.state)
            }
            _ => {
                format!("{parent_name}: {}", self.state)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct WatchPoint {
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

    pub fn _is_enabled(&self) -> bool {
        matches!(self.state, PointStatus::Enabled)
    }

    pub fn _group(&self) -> GroupIdx {
        self.group
    }

    pub fn print_details(&self) -> &PrintTuple {
        &self.print_details
    }
}

// impl Display for WatchPoint {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}.  {}", self.id, self.print_details.blue().bold())
//     }
// }

#[derive(Debug)]
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

    fn ctrl_nodes_off(&self) -> impl Iterator<Item = &T> {
        self.previous.difference(&self.current)
    }

    fn ctrl_nodes_on(&self) -> impl Iterator<Item = &T> {
        self.current.difference(&self.previous)
    }
}

#[derive(Debug, Clone, Copy)]
enum PointAction {
    Enable,
    Disable,
}

#[derive(Debug)]
struct BreakpointMap {
    control_idx_map: HashMap<ControlIdx, BreakpointIdx>,
    breakpoints: HashMap<BreakpointIdx, BreakPoint>,
    breakpoint_counter: IndexedMap<BreakpointIdx, ()>,
}

impl BreakpointMap {
    fn new() -> Self {
        Self {
            control_idx_map: HashMap::new(),
            breakpoints: HashMap::new(),
            breakpoint_counter: IndexedMap::new(),
        }
    }

    fn insert(&mut self, breakpoint: BreakPoint) {
        let idx = self.breakpoint_counter.next_key();
        self.control_idx_map.insert(breakpoint.control, idx);
        self.breakpoints.insert(idx, breakpoint);

        // ADD FOR GROUP RIGHT NOW FOR ENABLES
    }

    fn get_by_idx(&self, idx: BreakpointIdx) -> Option<&BreakPoint> {
        self.breakpoints.get(&idx)
    }

    fn get_by_control(&self, group: ControlIdx) -> Option<&BreakPoint> {
        // ASK HOW TO GET GROUP
        self.control_idx_map
            .get(&group)
            .and_then(|idx| self.get_by_idx(*idx))
    }

    fn get_by_group_mut(
        &mut self,
        control: ControlIdx,
    ) -> Option<&mut BreakPoint> {
        self.control_idx_map
            .get(&control)
            .and_then(|idx| self.breakpoints.get_mut(idx))
    }

    fn get_by_idx_mut(
        &mut self,
        idx: BreakpointIdx,
    ) -> Option<&mut BreakPoint> {
        self.breakpoints.get_mut(&idx)
    }

    fn breakpoint_exists(&self, control: ControlIdx) -> bool {
        self.control_idx_map.contains_key(&control)
    }

    fn delete_by_idx(&mut self, idx: BreakpointIdx) {
        let br = self.breakpoints.remove(&idx);
        if let Some(br) = br {
            self.control_idx_map.remove(&br.control);
        }
    }

    fn delete_by_control(&mut self, control: ControlIdx) {
        if let Some(idx) = self.control_idx_map.remove(&control) {
            self.breakpoints.remove(&idx);
        }
        // TODO ADD FUNCTIONALITY TO DELETE BY GROUPS RIGHT NOW DELETES BY COMPONENT
    }

    fn iter(&self) -> impl Iterator<Item = (&BreakpointIdx, &BreakPoint)> {
        self.breakpoints.iter()
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
            Self::Before(idx) => Some(idx),
            Self::Both { before, .. } => Some(before),
            Self::After(_) => None,
        }
    }

    fn get_after(&self) -> Option<&[WatchpointIdx]> {
        match self {
            Self::Before(_) => None,
            Self::After(idx) => Some(idx),
            Self::Both { after, .. } => Some(after),
        }
    }

    fn _iter(&self) -> Box<dyn Iterator<Item = &WatchpointIdx> + '_> {
        match self {
            Self::Before(idx) => Box::new(idx.iter()),
            Self::After(idx) => Box::new(idx.iter()),
            Self::Both { before, after } => {
                Box::new(before.iter().chain(after.iter()))
            }
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Self::Before(idx) => idx.is_empty(),
            Self::After(idx) => idx.is_empty(),
            Self::Both { before, after } => {
                before.is_empty() && after.is_empty()
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

    fn _get_by_group_mut(
        &mut self,
        group: GroupIdx,
    ) -> Option<&mut WatchPointIndices> {
        self.group_idx_map.get_mut(&group)
    }

    fn _get_by_idx_mut(
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
            if let Some(idxs) = self.group_idx_map.get_mut(&point.group) {
                match idxs {
                    WatchPointIndices::Before(b) => b.retain(|i| *i != idx),
                    WatchPointIndices::After(a) => a.retain(|i| *i != idx),
                    WatchPointIndices::Both { before, after } => {
                        before.retain(|i| *i != idx);
                        after.retain(|i| *i != idx);
                    }
                }

                if idxs.is_empty() {
                    self.group_idx_map.remove(&point.group);
                }
            }
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

    fn _iter_before(
        &self,
    ) -> impl Iterator<Item = (&WatchpointIdx, &WatchPoint)> {
        self.watchpoints_before.iter()
    }

    fn _iter_after(
        &self,
    ) -> impl Iterator<Item = (&WatchpointIdx, &WatchPoint)> {
        self.watchpoints_after.iter()
    }

    fn iter_groups(
        &self,
    ) -> impl Iterator<Item = (&GroupIdx, &WatchPointIndices)> {
        self.group_idx_map.iter()
    }
}

#[derive(Debug)]
pub(crate) struct DebuggingContext<C: AsRef<Context> + Clone> {
    breakpoints: BreakpointMap,
    watchpoints: WatchpointMap,
    // Emulating the original behavior for the time being, but this could be
    // shifted to use individual control points or full control nodes instead.
    group_info: GroupExecutionInfo<ControlIdx>,
    context: C,
}

impl<C: AsRef<Context> + Clone> DebuggingContext<C> {
    pub fn new(context: C) -> Self {
        Self {
            group_info: GroupExecutionInfo::new(),
            breakpoints: BreakpointMap::new(),
            watchpoints: WatchpointMap::new(),
            context,
        }
    }

    pub fn add_breakpoint(&mut self, target: ControlIdx) {
        if !self.breakpoints.breakpoint_exists(target) {
            let br = BreakPoint {
                control: target,
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
            let name = format!("{:?}", name);
            println!(
                "Error: There is no breakpoint named '{}'",
                name.stylize_debugger_missing()
            )
        } else {
            let num = target.as_number().unwrap();
            println!(
                "Error: There is no breakpoint numbered {}",
                num.stylize_debugger_missing()
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
            BreakpointID::Name(name) => {
                self.breakpoints.delete_by_control(name)
            }
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

    pub fn enable_watchpoint(&mut self, target: WatchID) {
        self.act_watchpoint(target, PointAction::Enable)
    }

    pub fn disable_watchpoint(&mut self, target: WatchID) {
        self.act_watchpoint(target, PointAction::Disable)
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
                if let Some(points) = self.watchpoints._get_by_group_mut(name) {
                    // mutability trickery
                    let points_actual = std::mem::replace(
                        points,
                        WatchPointIndices::Before(SmallVec::new()),
                    );

                    for point_idx in points_actual._iter() {
                        if let Some(point) =
                            self.watchpoints._get_by_idx_mut(*point_idx)
                        {
                            act(point, action);
                        }
                    }

                    *self.watchpoints._get_by_group_mut(name).unwrap() =
                        points_actual;
                } else {
                    println!(
                        "Error: There are no watchpoints for specified group",
                    )
                }
            }
            WatchID::Number(num) => {
                if let Some(point) = self.watchpoints._get_by_idx_mut(num) {
                    act(point, action);
                } else {
                    println!(
                        "Error: There is no watchpoint numbered {}",
                        num.stylize_debugger_missing()
                    )
                }
            }
        }
    }

    pub fn _enable_watchpoint(&mut self, target: WatchID) {
        self.act_watchpoint(target, PointAction::Enable)
    }

    pub fn _disable_watchpoint(&mut self, target: WatchID) {
        self.act_watchpoint(target, PointAction::Disable)
    }

    pub fn hit_breakpoints(&self) -> impl Iterator<Item = ControlIdx> + '_ {
        self.group_info
            .ctrl_nodes_on()
            .filter(|&&x| {
                self.breakpoints
                    .get_by_control(x)
                    .map(|x| x.is_enabled())
                    .unwrap_or_default()
            })
            .copied()
    }

    pub fn set_current_time<I: Iterator<Item = ControlIdx>>(
        &mut self,
        groups: I,
    ) {
        let group_map: HashSet<_> = groups.collect();
        self.group_info.shift_current(group_map.clone());
        self.group_info.shift_current(group_map);
    }

    pub fn advance_time<I: Iterator<Item = ControlIdx>>(&mut self, groups: I) {
        let group_map: HashSet<_> = groups.collect();
        self.group_info.shift_current(group_map);
    }

    pub fn hit_watchpoints(
        &self,
    ) -> impl Iterator<Item = (WatchpointIdx, &WatchPoint)> {
        let before_iter = self
            .group_info
            .ctrl_nodes_on()
            .filter_map(|x| {
                extract_group(self.context.as_ref(), *x)
                    .and_then(|x| self.watchpoints.get_by_group(x))
                    .map(|watchpoint_indices| match watchpoint_indices {
                        WatchPointIndices::Before(x) => x.iter(),
                        WatchPointIndices::Both { before, .. } => before.iter(),
                        _ => [].iter(),
                    })
            })
            .flatten();

        let after_iter = self
            .group_info
            .ctrl_nodes_off()
            .filter_map(|x| {
                if let Some(watchpoint_indices) =
                    extract_group(self.context.as_ref(), *x)
                        .and_then(|x| self.watchpoints.get_by_group(x))
                {
                    Some(match watchpoint_indices {
                        WatchPointIndices::After(x) => x.iter(),
                        WatchPointIndices::Both { after, .. } => after.iter(),
                        // this is stupid but works
                        _ => [].iter(),
                    })
                } else {
                    None
                }
            })
            .flatten();

        before_iter.chain(after_iter).filter_map(|watchpoint_idx| {
            let watchpoint =
                self.watchpoints.get_by_idx(*watchpoint_idx).unwrap();

            if watchpoint.is_disabled() {
                None
            } else {
                Some((*watchpoint_idx, watchpoint))
            }
        })
    }

    pub fn print_breakpoints(&self, ctx: &Context) {
        println!("{}Current breakpoints:", SPACING);
        for (breakpoint_idx, breakpoint) in self
            .breakpoints
            .iter()
            .sorted_by(|(a_idx, _), (b_idx, _)| a_idx.cmp(b_idx))
        {
            println!("{SPACING}({breakpoint_idx}) {}", breakpoint.format(ctx))
        }
    }

    pub fn print_watchpoints(&self, env: &Environment<C>) {
        println!("{}Current watchpoints:", SPACING);
        let inner_spacing = SPACING.to_string() + "    ";
        let outer_spacing = SPACING.to_string() + "  ";

        for (group, indicies) in self.watchpoints.iter_groups() {
            let group_name = env.ctx().lookup_name(*group);

            if indicies.get_before().is_some() {
                println!(
                    "{outer_spacing}Before {}:",
                    group_name.stylize_breakpoint()
                );
            }
            for watchpoint_idx in indicies
                .get_before()
                .map(|x| x.iter())
                .unwrap_or_else(|| [].iter())
            {
                let watchpoint =
                    self.watchpoints.get_by_idx(*watchpoint_idx).unwrap();
                println!(
                    "{inner_spacing} ({watchpoint_idx}): {} {}",
                    &watchpoint.print_details.format(env),
                    watchpoint.state
                );
            }

            if indicies.get_after().is_some() {
                println!(
                    "{outer_spacing}After {}:",
                    group_name.stylize_breakpoint()
                );
            }

            for watchpoint_idx in indicies
                .get_after()
                .map(|x| x.iter())
                .unwrap_or_else(|| [].iter())
            {
                let watchpoint =
                    self.watchpoints.get_by_idx(*watchpoint_idx).unwrap();
                println!(
                    "{inner_spacing} ({watchpoint_idx}): {} {}",
                    &watchpoint.print_details.format(env),
                    watchpoint.state
                );
            }
        }
    }
}

fn extract_group(ctx: &Context, control: ControlIdx) -> Option<GroupIdx> {
    if let Control::Enable(enable) = &ctx.primary[control].control {
        Some(enable.group())
    } else {
        None
    }
}

pub fn format_control_node(ctx: &Context, control_idx: ControlIdx) -> String {
    let control = &ctx.primary.control[control_idx].control;

    // Get parent
    let parent_comp = ctx.lookup_control_definition(control_idx);
    let parent_name = ctx.lookup_name(parent_comp);
    let name = parent_comp.lookup_name(ctx);

    match control {
        // Group
        Control::Enable(enable) => {
            let group = enable.group();
            let group_name = ctx.lookup_name(group);
            format!("{parent_name}::{group_name}")
        }
        _ => {
            let string_path = ctx.string_path(control_idx, name);
            format!("{parent_name}: {}", string_path)
        }
    }
}
