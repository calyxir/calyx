use calyx::ir;

#[derive(Default)]
pub(super) struct DebuggingContext {
    breakpoints: Vec<String>,
}

impl DebuggingContext {
    pub fn add_breakpoint(&mut self, target: String) {
        self.breakpoints.push(target);
    }

    pub fn remove_breakpoint(&mut self, target: String) {
        self.breakpoints.retain(|x| x != &target)
    }

    pub fn hit_breakpoints(
        &self,
        current_executing: &[&ir::Id],
    ) -> Vec<&String> {
        self.breakpoints
            .iter()
            .filter(|x| current_executing.iter().any(|y| y == x))
            .collect()
    }
}
