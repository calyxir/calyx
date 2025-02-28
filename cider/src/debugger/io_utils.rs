use super::commands::Command;
use super::commands::parse_command;
use crate::errors::{BoxedCiderError, CiderResult};
use rustyline::{DefaultEditor, Editor};

const SHELL_PROMPT: &str = " > ";

pub struct Input {
    buffer: DefaultEditor,
}

impl Input {
    pub fn new() -> CiderResult<Self> {
        Ok(Self {
            buffer: Editor::new()?,
        })
    }
    pub fn next_command(&mut self) -> CiderResult<Command> {
        match self.buffer.readline(SHELL_PROMPT) {
            Ok(command_str) => {
                self.buffer.add_history_entry(command_str.clone())?;
                parse_command(&command_str)
            }
            Err(e) => {
                if let rustyline::error::ReadlineError::Eof = e {
                    Ok(Command::Exit)
                } else {
                    Err(BoxedCiderError::from(e))
                }
            }
        }
    }
}
