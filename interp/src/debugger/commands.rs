use calyx::ir::Id;
use itertools::{self, Itertools};
use std::fmt::{Display, Write};
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

#[derive(Debug, Clone, Copy)]
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

impl Display for PrintCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PrintCode::Binary => "\\b".to_string(),
                PrintCode::Unsigned => "\\u".to_string(),
                PrintCode::Signed => "\\s".to_string(),
                PrintCode::UFixed(n) => format!("\\u.{}", n),
                PrintCode::SFixed(n) => format!("\\s.{}", n),
            }
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub enum WatchPosition {
    Before,
    After,
}

impl Default for WatchPosition {
    fn default() -> Self {
        Self::Before
    }
}

#[derive(Debug)]
pub enum PrintMode {
    State,
    Port,
}
#[derive(Debug)]
pub struct PrintTuple(Option<Vec<Vec<Id>>>, Option<PrintCode>, PrintMode);

impl PrintTuple {
    pub fn target(&self) -> &Option<Vec<Vec<Id>>> {
        &self.0
    }

    pub fn print_code(&self) -> &Option<PrintCode> {
        &self.1
    }

    pub fn print_mode(&self) -> &PrintMode {
        &self.2
    }
}

impl From<(Option<Vec<Vec<Id>>>, Option<PrintCode>, PrintMode)> for PrintTuple {
    fn from(val: (Option<Vec<Vec<Id>>>, Option<PrintCode>, PrintMode)) -> Self {
        PrintTuple(val.0, val.1, val.2)
    }
}

impl Display for PrintTuple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self.2 {
                PrintMode::State => "print-state",
                PrintMode::Port => "print",
            }
        )?;
        write!(
            f,
            " {}",
            match &self.1 {
                Some(s) => format!("{}", s),
                None => "".to_string(),
            }
        )?;
        write!(
            f,
            " {}",
            match &self.0 {
                Some(v) => v.iter().map(|x| x.iter().join(".")).join(" "),
                None => "".to_string(),
            }
        )
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
    InfoWatch,
    Disable(Vec<BreakPointId>),
    Enable(Vec<BreakPointId>),
    Delete(Vec<BreakPointId>),
    DeleteWatch(Vec<BreakPointId>),
    StepOver(GroupName),
    PrintState(Option<Vec<Vec<calyx::ir::Id>>>, Option<PrintCode>),
    Watch(
        GroupName,
        WatchPosition,
        Option<Vec<Vec<calyx::ir::Id>>>,
        Option<PrintCode>,
        PrintMode,
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
            (vec!["Step", "s"], "Advance the execution by a step"),
            (vec!["Step-over"], "Advance the execution over a given group"),
            (vec!["Continue", "c"], "Continue until the program finishes executing or hits a breakpoint"),
            (vec!["Display"], "Display the full state"),
            (vec!["Print", "p"], "Print target value"),
            (vec!["Print-state"], "Print the internal state of the target cell"),
            (vec!["Watch"], "Watch a given group with a print statement"),
            (vec!["Help", "h"], "Print this message"),
            (vec!["Break", "br"], "Create a breakpoint"),
            (vec!["Info break", "ib"], "List all breakpoints"),
            (vec!["Delete","del"], "Delete target breakpoint"),
            (vec!["Enable"], "Enable target breakpoint"),
            (vec!["Disable"], "Disable target breakpoint"),
            (vec!["Exit"], "Exit the debugger")
        ]
    }
}
