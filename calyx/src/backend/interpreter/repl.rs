use super::eval::Interpreter;
use crate::errors::Error;
use crate::lang::context::Context;
use linefeed::{Interface, ReadResult};

pub fn repl(c: &Context) -> Result<(), Error> {
    let mut interface = Interface::new("calyx_interpreter")?;

    let mut interpreter = Interpreter::new(c);

    interface.set_prompt("interpret>> ")?;

    println!(
        "This is an interactive interpreter for debugging Futil programs."
    );
    println!("Enter \"help\" for a list of commands.");
    println!("Press Ctrl-D or enter \"quit\" to exit.");
    println!("");

    while let ReadResult::Input(line) = interface.read_line()? {
        if !line.trim().is_empty() {
            interface.add_history_unique(line.clone());
        }

        let (cmd, args) = split_first_word(&line);

        match cmd {
            "help" => {
                println!("Interpreter commands:");
                println!();
                for &(cmd, help) in COMMANDS {
                    println!("  {:15} - {}", cmd, help);
                }
                println!();
            }
            "quit" => break,
            _ => println!("Unrecognized Command: {:?}", cmd),
        }
    }

    println!("Exiting Futil REPL");
    Ok(())
}

fn split_first_word(s: &str) -> (&str, &str) {
    let s = s.trim();

    match s.find(|ch: char| ch.is_whitespace()) {
        Some(pos) => (&s[..pos], s[pos..].trim_start()),
        None => (s, ""),
    }
}

static COMMANDS: &[(&str, &str)] = &[
    ("help", "Print this help message"),
    ("quit", "Quit the interpreter"),
];
