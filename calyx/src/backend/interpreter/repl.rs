use super::eval::Interpreter;
use crate::errors::Error;
use crate::lang::component::Component;
use crate::lang::context::Context;
use linefeed::{DefaultTerminal, Interface, ReadResult};

pub fn repl(c: &Context) -> Result<(), Error> {
    let interface = Interface::new("calyx_interpreter")?;

    let interpreter = Interpreter::new(c);

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

fn get_component_to_interpret(
    interface: Interface<DefaultTerminal>,
    interpreter: Interpreter,
) -> Result<Component, Error> {
    println!("First pick a component to simulate.");
    println!("If you pick a library component you may need to provide additional parameters.");
    println!("Here are the components you have loaded in the interpreter:");
    println!("");
    println!("User-defined components:");
    let iter_wrapper = interpreter.context.comp_def_iter();
    for (name, _comp_def) in iter_wrapper.into_iter() {
        println!("{}", name.to_string());
    }
    println!("");
    println!("Library-defined components:");
    let iter = interpreter.context.lib_def_iter();
    for (name, lib_def) in iter {
        let mut params = String::new();
        for param in &lib_def.params {
            params.push_str(&param.to_string());
            params.push(' ');
        }
        println!("{}, with params: {}", name.to_string(), params);
    }

    unimplemented!("to be continued")

    // if let ReadResult::Input(line) = interface.read_line()? {
    //     Ok(line)
    // }
    // Error()
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
