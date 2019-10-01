mod ast;
mod parse;
mod rtl_gen;

#[macro_use]
extern crate clap;

fn main() {
    let matches = clap_app!(calyx =>
                            (version: "0.1.0")
                            (author: "Samuel Thomas <sgt43@cornell.edu>")
                            (about: "Optimization passes for futil")
                            (@arg FILE: +required ... "Input file"))
    .get_matches();

    let filename = matches.value_of("FILE").unwrap();
    parse::parse_file(filename);
}
