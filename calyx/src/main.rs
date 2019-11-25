mod backend;
mod lang;
mod passes;
mod utils;

use crate::lang::*;

#[macro_use]
extern crate clap;

use passes::collapse_seqs;
use passes::visitor::Visitor;

fn main() {
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

    if matches.occurrences_of("LIB") == 1 {
        let filename = matches.value_of("LIB").unwrap();
        println!("LIBRARY FILE: {}\n\n\n", filename);
        let mut lib = lang::library::parse::parse_file(filename);
        //println!("{:#?}", backend::rtl::main::to_verilog());
    }

    //backend::rtl::gen::gen_namespace(&syntax, "./build/".to_string());
    //collapse_seqs::Count::new().do_pass(&mut syntax);

    // You can handle information about subcommands by requesting their matches by name
    // (as below), requesting just the name used, or both at the same time
    if matches.occurrences_of("VIZ") == 1 {
        for comp in &syntax.components {
            //comp.structure.visualize()
        }
    }
}
