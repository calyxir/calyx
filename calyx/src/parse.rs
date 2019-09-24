use std::fs;
use sexp::Sexp;
use sexp::Sexp::{Atom, List};
use crate::ast::*;

pub fn parse_file(filename: &str) {
    let content = &fs::read_to_string(filename)
        .expect("Something went wrong reading the file");

    parse(content);
}

fn parse(prog: &str) {
    println!("{}", prog);
    let res = sexp::parse(prog);
    match res {
        Ok(exp) => {
            parse_namespace(exp);
        },
        Err(e) => {
            println!("Error parsing program: {}", e);
        }
    }
    /*match exp {
        Atom(_) => panic!("Didn't expect an atom."),
        List(l) => panic!("nyi"),
    }*/
}

fn parse_namespace<'a>(e: Sexp) -> Namespace<'a> {
    match e {
        Atom(a) => panic!("Parsing Namespace: Expected List"),
        List(l) => panic!("Unimplemented"),
    }
}