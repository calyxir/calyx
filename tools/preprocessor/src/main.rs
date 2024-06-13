use colored::Colorize;
use preprocessor::Context;
use std::{
    io::{self, BufRead, Write},
    process,
};

fn fail(line_index: usize, msg: &str) -> io::Result<()> {
    writeln!(
        &mut io::stderr(),
        "{}: (line {}): {}",
        "error".bright_red(),
        line_index + 1,
        msg
    )?;
    process::exit(1);
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut ctx = Context::new();
    for (i, line) in stdin.lock().lines().enumerate() {
        match ctx.process(line?) {
            Ok(line) => println!("{}", line),
            Err(err) => fail(i, &err.to_string())?,
        }
    }
    Ok(())
}
