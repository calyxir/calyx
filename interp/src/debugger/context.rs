use super::cidb::SPACING;
use calyx::ir;

struct BreakPoint {
    id: u64,
    name: String,
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

impl std::fmt::Display for BreakPoint {
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

    pub fn remove_breakpoint(&mut self, target: String) {
        self.breakpoints.retain(|x| x.name != target)
    }

    pub fn remove_breakpoint_by_number(&mut self, target: u64) {
        self.breakpoints.retain(|x| x.id != target)
    }

    pub fn enable_breakpoint(&mut self, target: &str) {
        for x in self.breakpoints.iter_mut() {
            if x.name == target {
                x.enable();
                break;
            }
        }
    }

    pub fn enable_breakpoint_by_num(&mut self, target: u64) {
        for x in self.breakpoints.iter_mut() {
            if x.id == target {
                x.enable();
                break;
            }
        }
    }

    pub fn disable_breakpoint(&mut self, target: &str) {
        for x in self.breakpoints.iter_mut() {
            if x.name == target {
                x.disable();
                break;
            }
        }
    }

    pub fn disable_breakpoint_by_num(&mut self, target: u64) {
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
            println!("{}{}", SPACING, breakpoint)
        }
    }
}
