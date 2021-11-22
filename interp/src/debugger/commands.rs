use std::fmt::Write;
use std::ops::Deref;

#[derive(Debug, Default)]
pub struct GroupName(pub Vec<calyx::ir::Id>);

impl Deref for GroupName {
    type Target = Vec<calyx::ir::Id>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub enum BreakPointId {
    Name(GroupName),
    Number(u64),
}

impl From<GroupName> for BreakPointId {
    fn from(grp: GroupName) -> Self {
        Self::Name(grp)
    }
}
impl From<u64> for BreakPointId {
    fn from(n: u64) -> Self {
        Self::Number(n)
    }
}

#[derive(Clone, Copy)]
pub enum PrintCode {
    Binary,
    Unsigned,
    Signed,
    UFixed(usize),
    SFixed(usize),
}

impl Default for PrintCode {
    fn default() -> Self {
        Self::Unsigned
    }
}

pub enum Command {
    Step,                                                      // Step execution
    Continue, // Execute until breakpoint
    Empty,    // Empty command, does nothing
    Display,  // Display full environment contents
    Print(Option<Vec<Vec<calyx::ir::Id>>>, Option<PrintCode>), // Print something
    Break(Vec<GroupName>), // Create a breakpoint
    Help,                  // Help message
    Exit,                  // Exit the debugger
    InfoBreak,             // List breakpoints
    Disable(Vec<BreakPointId>),
    Enable(Vec<BreakPointId>),
    Delete(Vec<BreakPointId>),
    StepOver(GroupName),
    PrintState(Option<Vec<Vec<calyx::ir::Id>>>, Option<PrintCode>),
    Watch(
        GroupName,
        Option<Vec<Vec<calyx::ir::Id>>>,
        Option<PrintCode>,
        super::cidr::PrintMode,
    ),
}

impl Command {
    pub fn get_help_string() -> String {
        let mut out = String::new();
        for (names, message) in Command::help_string() {
            writeln!(out, "    {: <20}{}", names.join(", "), message).unwrap();
        }

        out
    }
}

impl Command {
    fn help_string() -> Vec<(Vec<&'static str>, &'static str)> {
        vec![
            (vec!["Step", "S"], "Advance the execution by a step"),
            (vec!["Step-over", "S"], "Advance the execution over a given group"),
            (vec!["Continue", "C"], "Continue until the program finishes executing or hits a breakpoint"),
            (vec!["Display"], "Display the full state"),
            (vec!["Print", "P"], "Print target value"),
            (vec!["Print-state"], "Print the internal state of the target cell"),
            (vec!["Watch"], "Watch a given group with a print statement"),
            (vec!["Help"], "Print this message"),
            (vec!["Break", "Br"], "Create a breakpoint"),
            (vec!["Info break"], "List all breakpoints"),
            (vec!["Delete","Del"], "Delete target breakpoint"),
            (vec!["Enable"], "Enable target breakpoint"),
            (vec!["Disable"], "Disable target breakpoint"),
            (vec!["Exit"], "Exit the debugger")
        ]
    }
}
