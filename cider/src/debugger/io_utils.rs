use super::commands::parse_command;
use super::commands::Command;
use crate::errors::{BoxedCiderError, CiderResult};
use rustyline::{DefaultEditor, Editor};
use std::collections::VecDeque;

const SHELL_PROMPT: &str = " > ";

pub struct Input {
    buffer: DefaultEditor,
    command_buffer: VecDeque<Command>,
}

impl Input {
    pub fn new() -> CiderResult<Self> {
        Ok(Self {
            buffer: Editor::new()?,
            command_buffer: VecDeque::default(),
        })
    }
    pub fn next_command(&mut self) -> CiderResult<Command> {
        if !self.command_buffer.is_empty() {
            return Ok(self.command_buffer.pop_front().unwrap());
        }

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
