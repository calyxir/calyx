use super::commands::Command;
use super::parser::parse_command;
use crate::errors::InterpreterResult;
use rustyline::Editor;
use std::collections::VecDeque;

const SHELL_PROMPT: &str = " > ";

pub struct Input {
    buffer: Editor<()>,
    command_buffer: VecDeque<Command>,
}

impl Input {
    pub fn new() -> InterpreterResult<Self> {
        Ok(Self {
            buffer: Editor::new()?,
            command_buffer: VecDeque::default(),
        })
    }
    pub fn next_command(&mut self) -> InterpreterResult<Command> {
        if !self.command_buffer.is_empty() {
            return Ok(self.command_buffer.pop_front().unwrap());
        }

        let result = self.buffer.readline(SHELL_PROMPT)?;
        self.buffer.add_history_entry(result.clone());
        parse_command(&result)
    }
}
