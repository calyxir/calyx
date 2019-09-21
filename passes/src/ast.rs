use sexp::Sexp;
use std::fs;

#[derive(Debug, Eq, PartialEq)]
pub enum SubModule {
    Name(String),
    ParamMod(String, Vec<i64>),
}

#[derive(Debug, Eq, PartialEq)]
pub enum Port {
    IO(String),
    Mod(String, String),
}

#[derive(Debug, Eq, PartialEq)]
pub enum Structure {
    Decl(String, SubModule),
    Connect(Port, Port),
}

pub enum ParSeq {
    Nyi,
}

pub struct Module {
    structure: Vec<Structure>,
    control: ParSeq,
}

pub struct Design {
    modules: Vec<Module>,
}

fn parse_structre(exp: Sexp) -> Vec<Structure> {
    use sexp::Sexp::{Atom, List};
    match exp {
        Atom(_) => panic!("Didn't expect an atom."),
        List(l) => panic!("nyi"),
    }
}

pub fn read_futil(filename: &str) -> Design {
    let content = &fs::read_to_string(filename).unwrap();
    let futil = sexp::parse(content).unwrap();
    match futil {
        Sexp::Atom(s) => println!("{:?}", s),
        Sexp::List(l) => println!("{:?}", l),
    }
    Design { modules: vec![] }
}
