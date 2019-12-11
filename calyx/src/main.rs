mod backend;
mod lang;
mod passes;
mod utils;

use crate::backend::framework::Context;
use crate::lang::pretty_print::PrettyPrint;
use crate::lang::*;
use crate::utils::NameGenerator;
// use crate::passes::visitor::Visitor;

#[macro_use]
extern crate clap;

fn main() {
    better_panic::install();

    let matches = clap_app!(calyx =>
        (version: "0.1.0")
        (author: "Samuel Thomas <sgt43@cornell.edu>, Kenneth Fang <kwf37@cornell.edu>")
        (about: "Optimization passes for futil")
        (@arg FILE: +required "File to use")
        (@arg COMPONENT: +required "Toplevel Component")
        (@arg LIB: -l --lib +takes_value "Libraries to load in")
        (@arg VIZ: -s --show "Output the structure in the Graphviz format")
    )
    .get_matches();

    let filename = matches.value_of("FILE").unwrap();
    let component_name = matches.value_of("COMPONENT").unwrap();
    let mut syntax: ast::Namespace = parse::parse_file(filename);

    let mut names = NameGenerator::new();

    if matches.occurrences_of("LIB") == 1 {
        let libname = matches.value_of("LIB").unwrap();
        let context = Context::init_context(
            filename.to_string(),
            component_name.to_string(),
            vec![libname.to_string()],
        );

        let verilog = backend::rtl::gen::to_verilog(&context);

        println!("{}", verilog);
    }

    passes::fsm::generate(&mut syntax, &mut names);
    if matches.occurrences_of("VIZ") == 0 {
        syntax.pretty_print();
    }

    let fsms = backend::fsm::machine_gen::generate_fsms(&mut syntax);
    //println!("{:#?}", fsms);

    // You can handle information about subcommands by requesting their matches by name
    // (as below), requesting just the name used, or both at the same time
    if matches.occurrences_of("VIZ") == 1 {
        for comp in &syntax.components {
            if comp.name == component_name {
                comp.structure_graph().visualize();
            }
        }
    }
}
