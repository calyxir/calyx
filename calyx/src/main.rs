mod ast;
mod parse;
mod pass;
mod rtl_gen;
mod unit_pass;

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
    let mut syntax: ast::Namespace = parse::parse_file(filename);
    unit_pass::do_nothing(&mut syntax);
}
