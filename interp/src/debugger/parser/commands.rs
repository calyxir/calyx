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
// This is used internally to print out the help message but otherwise is not used for anything
const HELP_LIST: [Command; 10] = [
    Command::Step,
    Command::Continue,
    Command::Display,
    Command::Print(None),
    Command::Break(Vec::new()),
    Command::Help,
    Command::InfoBreak,
    Command::Disable(Vec::new()),
    Command::Enable(Vec::new()),
    Command::Delete(Vec::new()),
];
pub enum Command {
    Step,                              // Step execution
    Continue,                          // Execute until breakpoint
    Empty,                             // Empty command, does nothing
    Display,                           // Display full environment contents
    Print(Option<Vec<calyx::ir::Id>>), // Print something
    Break(Vec<GroupName>),             // Create a breakpoint
    Help,                              // Help message
    Exit,                              // Exit the debugger
    InfoBreak,                         // List breakpoints
    Disable(Vec<BreakPointId>),
    Enable(Vec<BreakPointId>),
    Delete(Vec<BreakPointId>),
}

impl Command {
    pub fn get_help_string() -> String {
        let mut out = String::new();
        for comm in &HELP_LIST {
            let (names, message) = comm.help_string();
            writeln!(out, "    {: <20}{}", names.join(", "), message).unwrap();
        }
        out
    }
}

impl Command {
    fn help_string(&self) -> (Vec<&str>, &str) {
        match self {
            Command::Step => (vec!["Step", "S"], "Advance the execution by a step"),
            Command::Continue => ( vec!["Continue", "C"], "Continue until the program finishes executing or hits a breakpoint"),
            Command::Display => (vec!["Display"], "Display the full state"),
            Command::Print(_) => (vec!["Print", "P"], "Print target value"),
            Command::Help => (vec!["Help"], "Print this message"),
            Command::Empty | Command::Exit => unreachable!(), // This command needs no public facing help message
            Command::Break(_) => (vec!["Break", "Br"], "Create a breakpoint"),
            Command::InfoBreak => (vec!["Info break"], "List all breakpoints"),
            Command::Delete(_)=> (vec!["del"], "Delete target breakpoint"),
            Command::Enable(_) => (vec!["enable"], "Enable target breakpoint"),
            Command::Disable(_) => (vec!["disable"], "Disable target breakpoint"),
        }
    }
}
