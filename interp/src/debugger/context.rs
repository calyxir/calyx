use super::cidr::SPACING;
use super::parser::{BreakPointId, GroupName};
use calyx::ir;
struct BreakPoint {
    id: u64,
    name: String, // Name of the group
    enabled: bool,
}

impl BreakPoint {
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false
    }

    pub fn matches(&self, grp: &ir::Id) -> bool {
        self.enabled && grp == &self.name
    }
}

impl std::fmt::Debug for BreakPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.  {}  {}",
            &self.id,
            if self.enabled { "enabled" } else { "disabled" },
            &self.name
        )
    }
}

#[derive(Default)]
pub(super) struct DebuggingContext {
    breakpoints: Vec<BreakPoint>,
    count: u64,
}

impl DebuggingContext {
    pub fn add_breakpoint(&mut self, target: String) {
        if !self.breakpoints.iter().any(|x| x.name == target) {
            self.count += 1;
            let br = BreakPoint {
                id: self.count,
                name: target,
                enabled: true,
            };
            self.breakpoints.push(br);
        } else {
            println!("A breakpoint already exists for \"{}\"", target)
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
            BreakPointId::Name(name) => self.remove_breakpoint_by_name(name),
            BreakPointId::Number(num) => self.remove_breakpoint_by_number(*num),
        }
    }

    pub fn disable_breakpoint(&mut self, target: &BreakPointId) {
        match target {
            BreakPointId::Name(name) => self.disable_breakpoint_by_name(name),
            BreakPointId::Number(num) => self.disable_breakpoint_by_num(*num),
        }
    }

    fn remove_breakpoint_by_name(&mut self, target: &GroupName) {
        // TODO (Griffin): Fix this
        self.breakpoints.retain(|x| target[0] != x.name)
    }

    fn remove_breakpoint_by_number(&mut self, target: u64) {
        self.breakpoints.retain(|x| x.id != target)
    }

    fn enable_breakpoint_by_name(&mut self, target: &GroupName) {
        for x in self.breakpoints.iter_mut() {
            // TODO (Griffin): Fix this
            if target[0] == x.name {
                x.enable();
                break;
            }
        }
    }

    fn enable_breakpoint_by_num(&mut self, target: u64) {
        for x in self.breakpoints.iter_mut() {
            if x.id == target {
                x.enable();
                break;
            }
        }
    }

    fn disable_breakpoint_by_name(&mut self, target: &GroupName) {
        for x in self.breakpoints.iter_mut() {
            // TODO (Griffin): Fix this
            if target[0] == x.name {
                x.disable();
                break;
            }
        }
    }

    fn disable_breakpoint_by_num(&mut self, target: u64) {
        for x in self.breakpoints.iter_mut() {
            if x.id == target {
                x.disable();
                break;
            }
        }
    }

    pub fn hit_breakpoints(
        &self,
        current_executing: &[&ir::Id],
    ) -> Vec<&String> {
        self.breakpoints
            .iter()
            .filter(|x| current_executing.iter().any(|y| x.matches(y)))
            .map(|x| &x.name)
            .collect()
    }

    pub fn print_breakpoints(&self) {
        println!("{}Current breakpoints:", SPACING);
        for breakpoint in self.breakpoints.iter() {
            println!("{}{:?}", SPACING, breakpoint)
        }
    }
}
