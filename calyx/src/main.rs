mod backend;
mod cmdline;
mod errors;
mod lang;
mod passes;
mod utils;

use crate::backend::framework::Context;
use crate::backend::fsm::machine::FSM;
use crate::backend::fsm::{machine_gen, rtl_gen};
use crate::cmdline::{path_write, Opts};
use crate::lang::pretty_print::PrettyPrint;
use crate::lang::*;
use crate::passes::visitor::Visitor;
use crate::utils::NameGenerator;
use std::fmt::Write;
use structopt::StructOpt;

fn main() -> Result<(), errors::Error> {
    // better stack traces
    better_panic::install();

    // parse the command line arguments into Opts struct
    let opts: Opts = Opts::from_args();

    let mut names = NameGenerator::new();
    let mut syntax = parse::parse_file(&opts.file)?;

    let mut verilog_buf = String::new();

    utils::ignore(writeln!(verilog_buf, "`include \"sim/lib/std.v\""));

    passes::add_read_wire::ReadWire::new().do_pass(&mut syntax);
    passes::lat_insensitive::LatencyInsenstive::new().do_pass(&mut syntax);
    passes::fsm::generate(&mut syntax, &mut names);
    passes::interfacing::Interfacing::new().do_pass(&mut syntax);
    passes::control_lookup::Lookup::new(&mut names).do_pass(&mut syntax);
    passes::toplevel_component::Toplevel::new(opts.component.clone())
        .do_pass(&mut syntax);
    //let fsms = backend::fsm::machine_gen::generate_fsms(&mut syntax);

    // output futil after passes
    opts.futil_output.as_ref().map_or((), |path| {
        path_write(&path, Some("futil"), Some("futil"), &mut |w| {
            writeln!(w, "{}", syntax.pretty_string())
        })
    });

    let fsms: Vec<FSM> = syntax
        .components
        .iter()
        .filter_map(machine_gen::generate_fsm)
        .collect();

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

    // generate verilog
    opts.libraries.as_ref().map_or(Ok(()), |libpath| {
        let context =
            Context::init_context(&mut syntax, &opts.component, &libpath[..]);

        let verilog = backend::rtl::gen::to_verilog(&context);
        writeln!(verilog_buf, "{}", verilog)
    })?;

    for comp in &syntax.components {
        if comp.name.starts_with("lut_control") {
            let verilog = backend::fsm::rtl_gen::control_lut_verilog(comp);
            writeln!(verilog_buf, "{}", verilog)?;
        }
    }

    // generate verilog for fsms
    for comp in &syntax.components {
        machine_gen::generate_fsm(comp).map_or(Ok(()), |fsm| {
            writeln!(verilog_buf, "{}", rtl_gen::to_verilog(&fsm, comp))
        })?;
    }
    // Commit Verilog buffer to output file
    path_write(&opts.output, None, Some("v"), &mut |w| {
        write!(w, "{}", verilog_buf)
    });

    Ok(())
}
