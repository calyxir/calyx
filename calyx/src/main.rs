mod lang;
mod passes;
mod rtl;
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
                            (@arg FILE: +required ... "Input file"))
        .get_matches();

    let filename = matches.value_of("FILE").unwrap();
    let mut syntax: ast::Namespace = parse::parse_file(filename);
    // rtl::gen::gen_namespace(&syntax, "./build/".to_string());
    collapse_seqs::Count::new().do_pass(&mut syntax);
}
