use super::commands::Command;
use std::collections::VecDeque;
use std::io::prelude::*;
use std::io::{stdin, stdout, BufReader, Stdin};

const SHELL_PROMPT: &str = " > ";

#[inline]
fn print_shell_prompt() {
    print!("{}", SHELL_PROMPT);
    stdout().flush().expect("Cannot flush stdout");
}
pub struct Input {
    buffer: BufReader<Stdin>,
    command_buffer: VecDeque<Command<String>>,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            buffer: BufReader::new(stdin()),
            command_buffer: VecDeque::default(),
        }
    }
}

impl Input {
    pub fn next_command(&mut self) -> Command<String> {
        if !self.command_buffer.is_empty() {
            return self.command_buffer.pop_front().unwrap();
        }
        let mut line = String::new();
        loop {
            print_shell_prompt();
            let result = self.buffer.read_line(&mut line);
            match result {
                Ok(len) => {
                    if len == 0 {
                        panic!("No new input")
                    }
                }
                Err(err) => panic!("Unable to read from stdin! {}", err),
            }
            match Command::<String>::parse(&line) {
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
                    line = String::new()
                }
            }
        }
    }
}
