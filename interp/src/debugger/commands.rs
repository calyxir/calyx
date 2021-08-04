use std::fmt::Display;

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
pub enum Command {
    Step,     // Step execution
    Continue, // Execute until breakpoint
    Empty,    // Empty command, does nothing
    Display,  // Display full environment contents
    Print(String),
}

impl Command {
    /// Parse the given input string into a Command returning an
    /// InterpreterError if the parse is invalid
    pub fn parse(input: &str) -> Result<Self, InterpreterError> {
        let saved_input: Vec<_> = input.split_whitespace().skip(1).collect();
        let input = input.trim().to_lowercase(); // basic normalization
        let input: Vec<_> = input.split_whitespace().collect();

        match input[..] {
            [] => Ok(Command::Empty),
            ["step"] | ["s"] => Ok(Command::Step),
            ["continue"] => Ok(Command::Continue),
            ["display"] => Ok(Command::Display),
            ["print", _target] => {
                Ok(Command::Print(saved_input[0].to_string()))
            }
            ["print", ..] => Err(InterpreterError::InvalidCommand(
                "Print requires exactly one target".to_string(),
            )),
            // can't get the size of the pattern match so use `input`
            _ => Err(InterpreterError::UnknownCommand(input.join(" "))),
        }
    }
}
