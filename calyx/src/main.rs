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

    let mut verilog_buf = String::new();

    passes::fsm::generate(&mut syntax, &mut names);
    let fsms = backend::fsm::machine_gen::generate_fsms(&mut syntax);

    // generate verilog
    opts.libraries.as_ref().map_or(Ok(()), |libpath| {
        let context =
            Context::init_context(&mut syntax, &opts.component, &libpath[..]);

        let verilog = backend::rtl::gen::to_verilog(&context);
        writeln!(verilog_buf, "{}", verilog)
    })?;

    // generate verilog for fsms
    for fsm in &fsms {
        writeln!(verilog_buf, "{}", rtl_gen::to_verilog(fsm))?;
    }
    path_write(&opts.output, None, Some("v"), &mut |w| {
        write!(w, "{}", verilog_buf)
    });

    // visualize fsms
    opts.visualize_fsm.as_ref().map_or((), |path| {
        // get fsm for specified component
        let fsm = fsms.iter().find(|x| x.name == opts.component);
        fsm.map_or((), |fsm| {
            // commit fsm
            path_write(&path, Some("_fsm"), Some("dot"), &mut |w| {
                write!(w, "{}", fsm.visualize())
            });
            // try running dot
            path.as_ref()
                .map_or((), |p| utils::dot_command(&p, Some("_fsm")));
        })
    });

    // visualize
    opts.visualize_structure.as_ref().map_or((), |path| {
        // get specified component
        let comp = &syntax.components.iter().find(|x| x.name == opts.component);
        comp.map_or((), |comp| {
            // commit visualization for comp
            path_write(&path, Some("_struct"), Some("dot"), &mut |w| {
                write!(w, "{}", comp.structure_graph().visualize())
            });
            // try running dot
            path.as_ref()
                .map_or((), |p| utils::dot_command(&p, Some("_struct")));
        })
    });

    Ok(())
}
