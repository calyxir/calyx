use super::commands::parse_command;
use super::commands::Command;
use crate::errors::CiderResult;
use rustyline::Editor;
use std::collections::VecDeque;

const SHELL_PROMPT: &str = " > ";

pub struct Input {
    buffer: Editor<()>,
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

        let result = self.buffer.readline(SHELL_PROMPT)?;
        self.buffer.add_history_entry(result.clone());
        parse_command(&result)
    }
}
