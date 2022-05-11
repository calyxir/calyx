use calyx::ir::Id;
use itertools::{self, Itertools};
use std::fmt::{Display, Write};

use crate::structures::names::CompGroupName;

#[derive(Debug)]
pub struct ParsedGroupName {
    component: Option<calyx::ir::Id>,
    group: calyx::ir::Id,
}

impl ParsedGroupName {
    pub fn from_group_name(group: calyx::ir::Id) -> Self {
        Self {
            component: None,
            group,
        }
    }

    pub fn from_comp_and_group(
        component: calyx::ir::Id,
        group: calyx::ir::Id,
    ) -> Self {
        Self {
            component: Some(component),
            group,
        }
    }

    pub fn is_concrete(&self) -> bool {
        self.component.is_some()
    }

    pub fn unwrap_concrete(self) -> CompGroupName {
        CompGroupName::new(self.group, self.component.unwrap())
    }

    pub fn concretize(
        mut self,
        main_comp_name: &calyx::ir::Id,
    ) -> CompGroupName {
        if !self.is_concrete() {
            self.component = Some(main_comp_name.clone());
        }

        self.unwrap_concrete()
    }
}

pub enum BreakPointId {
    Name(ParsedGroupName),
    Number(u64),
}

impl From<ParsedGroupName> for BreakPointId {
    fn from(grp: ParsedGroupName) -> Self {
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
pub struct PrintTuple(Vec<Vec<Id>>, Option<PrintCode>, PrintMode);

impl PrintTuple {
    pub fn target(&self) -> &Vec<Vec<Id>> {
        &self.0
    }

    pub fn print_code(&self) -> &Option<PrintCode> {
        &self.1
    }

    pub fn print_mode(&self) -> &PrintMode {
        &self.2
    }
}

impl From<(Vec<Vec<Id>>, Option<PrintCode>, PrintMode)> for PrintTuple {
    fn from(val: (Vec<Vec<Id>>, Option<PrintCode>, PrintMode)) -> Self {
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
            &self.0.iter().map(|x| x.iter().join(".")).join(" "),
        )
    }
}

pub enum Command {
    Step(u64), // Step execution
    Continue,  // Execute until breakpoint
    Empty,     // Empty command, does nothing
    Display,   // Display full environment contents
    Print(Vec<Vec<calyx::ir::Id>>, Option<PrintCode>, PrintMode), // Print something
    Break(Vec<ParsedGroupName>), // Create a breakpoint
    Help,                        // Help message
    Exit,                        // Exit the debugger
    InfoBreak,                   // List breakpoints
    InfoWatch,
    Disable(Vec<BreakPointId>),
    Enable(Vec<BreakPointId>),
    Delete(Vec<BreakPointId>),
    DeleteWatch(Vec<BreakPointId>),
    StepOver(ParsedGroupName),
    Watch(
        ParsedGroupName,
        WatchPosition,
        Vec<Vec<calyx::ir::Id>>,
        Option<PrintCode>,
        PrintMode,
    ),
    PrintPC,
}

type Description = &'static str;
type UsageExample = &'static str;
type CommandName = &'static str;

type HelpInfo = (Vec<CommandName>, Description, Vec<UsageExample>);

impl Command {
    pub fn get_help_string() -> String {
        let mut out = String::new();
        for (names, message, _) in Command::help_string() {
            writeln!(out, "    {: <20}{}", names.join(", "), message).unwrap();
        }

        out
    }

    fn help_string() -> Vec<HelpInfo> {
        vec![
            (vec!["step", "s"], "Advance the execution by a step. If provided a number, it will advance by that many steps (skips breakpoints).", vec!["> s", "> s 5"]),
            (vec!["step-over"], "Advance the execution over a given group.", vec!["> step-over this_group"]),
            (vec!["continue", "c"], "Continue until the program finishes executing or hits a breakpoint", vec![]),
            (vec!["display"], "Display the full state", vec![]),
            (vec!["print", "p"], "Print target value. Takes an optional print code before the target. Valid print codes are \\u (unsigned), \\s (signed), \\u.X (unsigned fixed-point, X frac bits), \\s.X (signed fixed-point)", vec!["> p reg.write_en", "> p \\u mult1"]),
            (vec!["print-state"], "Print the internal state of the target cell. Takes an optional print code before the target", vec!["> print-state my_register", "> print-state \\s.16 mem"]),
            (vec!["watch"], "Watch a given group with a print statement. Takes an optional position (before/after)", vec!["> watch GROUP with p \\u reg.in", "> watch after GROUP with print-state \\s mem"] ),
            (vec!["help", "h"], "Print this message", vec![]),
            (vec!["break", "br"], "Create a breakpoint", vec!["> br do_add", "> br subcomp::let0"]),
            (vec!["info break", "ib"], "List all breakpoints", vec![]),
            (vec!["delete","del"], "Delete target breakpoint", vec!["> del 1", "> del do_add"]),
            (vec!["enable"], "Enable target breakpoint", vec!["> enable 1", "> enable do_add"]),
            (vec!["disable"], "Disable target breakpoint", vec!["> disable 4", "> disable do_mult"]),
            (vec!["exit", "quit"], "Exit the debugger", vec![])
        ]
    }
}
