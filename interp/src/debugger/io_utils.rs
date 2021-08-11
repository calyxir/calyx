use super::commands::Command;
//use rustyline::error::ReadlineError;
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
    pub fn next_command(&mut self) -> Command<String> {
        if !self.command_buffer.is_empty() {
            return self.command_buffer.pop_front().unwrap();
        }
        loop {
            let result = self.buffer.readline(SHELL_PROMPT);
            match result {
                Ok(result) => {
                    self.buffer.add_history_entry(result.clone());
                    match Command::<String>::parse(&result) {
                        Ok(mut comm) => {
                            if comm.len() == 1 {
                                return comm.remove(0);
                            } else {
                                let res = comm.remove(0);
                                self.command_buffer.extend(comm);
                                return res;
                            }
                        }
                        Err(e) => {
                            println!("Error: {}", e);
                        }
                    }
                }
                Err(err) => panic!("{}", err),
            }
        }
    }
}
