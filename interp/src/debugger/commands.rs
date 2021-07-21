use std::fmt::Display;

pub enum InterpreterError {
    InvalidCommand,
    UnknownCommand,
}

impl Display for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out_str = match self {
            InterpreterError::InvalidCommand => "Invalid command",
            InterpreterError::UnknownCommand => "Unknown command",
        };
        f.write_str(out_str)
    }
}
pub enum Command {
    Step,
    Continue,
}

impl Command {
    pub fn parse(input: &String) -> Result<Self, InterpreterError> {
        let input = input.trim().to_lowercase();
        let input: Vec<_> = input.split_whitespace().collect();
        match input[..] {
            [] => Err(InterpreterError::InvalidCommand),
            ["step"] => Ok(Command::Step),
            ["continue"] => Ok(Command::Continue),
            _ => Err(InterpreterError::UnknownCommand),
        }
    }
}
