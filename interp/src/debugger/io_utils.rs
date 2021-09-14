use super::commands::Command;
use crate::errors::InterpreterResult;
use rustyline::Editor;
use std::collections::VecDeque;

const SHELL_PROMPT: &str = " > ";

pub struct Input {
    buffer: Editor<()>,
    command_buffer: VecDeque<Command<String>>,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            buffer: Editor::new(),
            command_buffer: VecDeque::default(),
        }
    }
}

impl Input {
    pub fn next_command(&mut self) -> InterpreterResult<Command<String>> {
        if !self.command_buffer.is_empty() {
            return Ok(self.command_buffer.pop_front().unwrap());
        }

        let result = self.buffer.readline(SHELL_PROMPT)?;
        self.buffer.add_history_entry(result.clone());
        let mut comm = Command::<String>::parse(&result)?;

        if comm.len() == 1 {
            Ok(comm.remove(0))
        } else {
            let res = comm.remove(0);
            self.command_buffer.extend(comm);
            Ok(res)
        }
    }
}
