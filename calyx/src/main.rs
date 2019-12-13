mod backend;
mod cmdline;
mod errors;
mod lang;
mod passes;
mod utils;

// use crate::backend::framework::Context;
use crate::backend::framework::Context;
use crate::backend::fsm::rtl_gen;
use crate::cmdline::{path_write, Opts};
// use crate::backend::fsm::visualizer;
// use crate::lang::pretty_print::PrettyPrint;
use crate::lang::*;
use crate::utils::NameGenerator;
use std::fmt::Write;
use structopt::StructOpt;
// use crate::backend::fsm::visualize;
// use crate::passes::visitor::Visitor;

fn main() -> Result<(), errors::Error> {
    // better stack traces
    better_panic::install();

    // parse the command line arguments into Opts struct
    let opts: Opts = Opts::from_args();

    let mut names = NameGenerator::new();
    let mut syntax = parse::parse_file(&opts.file)?;

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

    // generate verilog
    opts.libraries.as_ref().map_or((), |libpath| {
        let context =
            Context::init_context(&opts.file, &opts.component, &libpath[..]);

        let verilog = backend::rtl::gen::to_verilog(&context);
        path_write(&opts.output, None, Some("v"), &mut |w| {
            writeln!(w, "{}", verilog)
        })
    });

    passes::fsm::generate(&mut syntax, &mut names);

    // visualize fsms
    let fsms = backend::fsm::machine_gen::generate_fsms(&mut syntax);
    opts.visualize_fsm.as_ref().map_or((), |path| {
        // get fsm for specified component
        let fsm = fsms.iter().find(|x| x.name == opts.component);
        fsm.map_or((), |fsm| {
            // commit fsm
            path_write(&path, Some("_fsm"), Some("dot"), &mut |w| {
                writeln!(w, "{}", fsm.visualize())
            });
            // try running dot
            utils::dot_command(&path)
        })
    });

    // visualize
    opts.visualize_structure.as_ref().map_or((), |path| {
        // get specified component
        let comp = &syntax.components.iter().find(|x| x.name == opts.component);
        comp.map_or((), |comp| {
            // commit visualization for comp
            path_write(&path, Some("_struct"), Some("dot"), &mut |w| {
                writeln!(w, "{}", comp.structure_graph().visualize())
            });
            // try running dot
            utils::dot_command(&path);
        })
    });

    Ok(())
}
