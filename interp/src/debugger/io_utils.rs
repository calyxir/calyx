use super::commands::Command;
use std::io::prelude::*;
use std::io::{stdin, BufReader, Stdin};

const SHELL_PROMPT: &str = "$> ";

#[inline]
fn print_shell_prompt() {
    print!("{}", SHELL_PROMPT);
}
pub struct Input {
    buffer: BufReader<Stdin>,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            buffer: BufReader::new(stdin()),
        }
    }
}

impl Input {
    pub fn next_command(&mut self) -> Command {
        let mut line = String::new();
        loop {
            print_shell_prompt();
            let result = self.buffer.read_line(&mut line);
            match result {
                Ok(len) => {}
                Err(err) => panic!("Unable to read from stdin! {}", err),
            }
            match Command::parse(&line) {
                Ok(comm) => return comm,
                Err(e) => println!("Error: {}", e),
            }
        }
    }
}
