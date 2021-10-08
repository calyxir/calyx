use crate::errors::{InterpreterError, InterpreterResult};
use std::fmt::Write;

// This is used internally to print out the help message but otherwise is not used for anything
const HELP_LIST: [Command<&str>; 10] = [
    Command::Step,
    Command::Continue,
    Command::Display,
    Command::Print(Vec::new()),
    Command::Break(""),
    Command::Help,
    Command::InfoBreak,
    Command::DelBreakpointByName(""),
    Command::EnableBreakpointByName(""),
    Command::DisableBreakpointByName(""),
];
pub enum Command<S: AsRef<str>> {
    Step,          // Step execution
    Continue,      // Execute until breakpoint
    Empty,         // Empty command, does nothing
    Display,       // Display full environment contents
    Print(Vec<S>), // Print something
    Break(S),      // Create a breakpoint
    Help,          // Help message
    Exit,          // Exit the debugger
    InfoBreak,     // List breakpoints
    DelBreakpointByNum(u64),
    DelBreakpointByName(S),
    EnableBreakpointByNum(u64),
    EnableBreakpointByName(S),
    DisableBreakpointByNum(u64),
    DisableBreakpointByName(S),
}

impl Command<&str> {
    pub fn get_help_string() -> String {
        let mut out = String::new();
        for comm in &HELP_LIST {
            let (names, message) = comm.help_string();
            writeln!(out, "    {: <20}{}", names.join(", "), message).unwrap();
        }
        out
    }
}

impl<S: AsRef<str>> Command<S> {
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
            Command::DelBreakpointByNum(_) | Command::DelBreakpointByName(_) => (vec!["del"], "Delete target breakpoint"),
            Command::EnableBreakpointByNum(_) | Command::EnableBreakpointByName(_) => (vec!["enable"], "Enable target breakpoint"),
            Command::DisableBreakpointByNum(_) | Command::DisableBreakpointByName(_) => (vec!["disable"], "Disable target breakpoint"),
        }
    }

    /// Parse the given input string into a Command returning an
    /// InterpreterError if the parse is invalid
    pub fn parse(input: &str) -> InterpreterResult<Vec<Command<String>>> {
        let saved_input: Vec<_> = input.split_whitespace().skip(1).collect();
        let input = input.trim().to_lowercase(); // basic normalization
        let input: Vec<_> = input.split_whitespace().collect();

        match input[..] {
            [] => Ok(vec![Command::Empty]),
            ["step"] | ["s"] => Ok(vec![Command::Step]),
            ["continue"] | ["c"] => Ok(vec![Command::Continue]),
            ["display"] => Ok(vec![Command::Display]),
            ["print", _target] | ["p", _target] => {
                let mut target: Vec<_> =
                    saved_input[0].split('.').map(str::to_string).collect();
                if target.len() == 0 {
                    return Ok(vec![Command::Empty]);
                } else if target[0] == "main" {
                    target.remove(0);
                };

                let command = Command::Print(target);
                Ok(vec![command])
            }
            ["print", ..] | ["p", ..] => Err(InterpreterError::InvalidCommand(
                "Print requires exactly one target".to_string(),
            )),
            ["break", _target, ..] | ["br", _target, ..] => {
                let vec = saved_input
                    .iter()
                    .map(|x| Command::Break(x.to_string()))
                    .collect();
                Ok(vec)
            }
            ["info", "break"]
            | ["info", "br"]
            | ["i", "br"]
            | ["i", "break"] => Ok(vec![Command::InfoBreak]),
            ["del"] | ["d"] => Err(InterpreterError::InvalidCommand(
                "Command requires a target".to_string(),
            )),
            ["del", ..] | ["d", ..] => {
                let vec = saved_input
                    .iter()
                    .map(|target| {
                        if let Ok(num) = target.parse::<u64>() {
                            Command::DelBreakpointByNum(num)
                        } else {
                            Command::DelBreakpointByName(target.to_string())
                        }
                    })
                    .collect();
                Ok(vec)
            }
            ["enable", ..] => {
                let vec = saved_input
                    .iter()
                    .map(|target| {
                        if let Ok(num) = target.parse::<u64>() {
                            Command::EnableBreakpointByNum(num)
                        } else {
                            Command::EnableBreakpointByName(target.to_string())
                        }
                    })
                    .collect();
                Ok(vec)
            }
            ["disable", ..] => {
                let vec = saved_input
                    .iter()
                    .map(|target| {
                        if let Ok(num) = target.parse::<u64>() {
                            Command::DisableBreakpointByNum(num)
                        } else {
                            Command::DisableBreakpointByName(target.to_string())
                        }
                    })
                    .collect();
                Ok(vec)
            }
            ["help"] => Ok(vec![Command::Help]),
            ["quit"] | ["exit"] => Ok(vec![Command::Exit]),
            // can't get the size of the pattern match so use `input`
            _ => Err(InterpreterError::UnknownCommand(input.join(" "))),
        }
    }
}
