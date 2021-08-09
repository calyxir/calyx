use std::fmt::Display;
use std::fmt::Write;

pub enum InterpreterError {
    InvalidCommand(String), // this isn't used yet, but may be useful later when commands have more syntax
    UnknownCommand(String),
}

impl Display for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out_str = match self {
            InterpreterError::InvalidCommand(msg) => {
                format!("Invalid Command: {}", msg)
            }
            InterpreterError::UnknownCommand(s) => {
                format!("Unknown command {}", s)
            }
        };
        f.write_str(&out_str)
    }
}

const HELP_LIST: [Command<&str>; 6] = [
    Command::Step,
    Command::Continue,
    Command::Display,
    Command::PrintOne(""),
    Command::Break(""),
    Command::Help,
];
pub enum Command<S: AsRef<str>> {
    Step,                // Step execution
    Continue,            // Execute until breakpoint
    Empty,               // Empty command, does nothing
    Display,             // Display full environment contents
    PrintOne(S),         // Print a cell's ports
    PrintTwo(S, S),      // Print a specific port or specific cell
    PrintThree(S, S, S), // Print a specific port (fully specified)
    Break(S),            // Create a breakpoint
    Help,                // Help message
    Exit,
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
            Command::PrintOne(_) | Command::PrintTwo(..) | Command::PrintThree(..) => (vec!["Print", "P"], "Print target value"),
            Command::Help => (vec!["Help"], "Print this message"),
            Command::Empty | Command::Exit => unimplemented!(),
            Command::Break(_) => (vec!["Break", "Br"], "Create a breakpoint"), // This command needs no public facing
        }
    }

    /// Parse the given input string into a Command returning an
    /// InterpreterError if the parse is invalid
    pub fn parse(input: &str) -> Result<Command<String>, InterpreterError> {
        let saved_input: Vec<_> = input.split_whitespace().skip(1).collect();
        let input = input.trim().to_lowercase(); // basic normalization
        let input: Vec<_> = input.split_whitespace().collect();

        match input[..] {
            [] => Ok(Command::Empty),
            ["step"] | ["s"] => Ok(Command::Step),
            ["continue"] | ["c"] => Ok(Command::Continue),
            ["display"] => Ok(Command::Display),
            ["print", _target] | ["p", _target] => {
                let target: Vec<_> = saved_input[0].split('.').collect();
                match target[..] {
                    [t] => Ok(Command::PrintOne(t.to_string())),
                    [first, second] => Ok(Command::PrintTwo(
                        first.to_string(),
                        second.to_string(),
                    )),
                    [component, cell, port] => Ok(Command::PrintThree(
                        component.to_string(),
                        cell.to_string(),
                        port.to_string(),
                    )),
                    _ => Err(InterpreterError::InvalidCommand(
                        "Print requires exactly one target".to_string(),
                    )),
                }
            }
            ["print", ..] | ["p", ..] => Err(InterpreterError::InvalidCommand(
                "Print requires exactly one target".to_string(),
            )),
            ["break", _target] | ["br", _target] => {
                let target = saved_input[0];
                Ok(Command::Break(target.to_string()))
            }
            ["help"] => Ok(Command::Help),
            ["quit"] | ["exit"] => Ok(Command::Exit),
            // can't get the size of the pattern match so use `input`
            _ => Err(InterpreterError::UnknownCommand(input.join(" "))),
        }
    }
}
