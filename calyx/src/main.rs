mod backend;
mod cmdline;
mod errors;
mod lang;
mod passes;
mod utils;

// use crate::backend::framework::Context;
use crate::cmdline::{path_write, Opts};
// use crate::lang::pretty_print::PrettyPrint;
use crate::lang::*;
use crate::utils::NameGenerator;
use std::fmt::Write;
use structopt::StructOpt;
// use crate::backend::fsm::visualize;
// use crate::passes::visitor::Visitor;

fn main() -> Result<(), errors::Error> {
    better_panic::install();

    let opts: Opts = Opts::from_args();

    let mut syntax = parse::parse_file(&opts.file)?;

    let mut names = NameGenerator::new();

    // if matches.occurrences_of("LIB") == 1 {
    //     let libname = matches.value_of("LIB").unwrap();
    //     let context = Context::init_context(
    //         filename.to_string(),
    //         component_name.to_string(),
    //         vec![libname.to_string()],
    //     );

    //     let verilog = backend::rtl::gen::to_verilog(&context);

    //     println!("{}", verilog);
    // }

    passes::fsm::generate(&mut syntax, &mut names);
    // match opts.visualize_structure {
    //     None => (),
    //     Some(None) =>
    // }
    // if matches.occurrences_of("VIZ") == 0 {
    //     syntax.pretty_print();
    // }

    let fsms = backend::fsm::machine_gen::generate_fsms(&mut syntax);
    match &opts.visualize_fsm {
        None => (),
        Some(po) => {
            for fsm in fsms {
                path_write(po, Box::new(move |w| writeln!(w, "{:#?}", fsm)));
            }
        }
    }
    // println!("{:#?}", fsms);

    // // You can handle information about subcommands by requesting their matches by name
    // // (as below), requesting just the name used, or both at the same time
    // if matches.occurrences_of("VIZ") == 1 {
    //     for comp in &syntax.components {
    //         if comp.name == component_name {
    //             comp.structure_graph().visualize();
    //         }
    //     }
    // }
    Ok(())
}
