use crate::lang::library::ast::{Library, PrimPortdef, Primitive, Width};
use crate::lang::utils::*;
use sexp::Sexp;
use sexp::Sexp::{Atom, List};
use std::fs;
use std::path::PathBuf;

pub fn parse_file(filename: &PathBuf) -> Library {
    let content = &fs::read(filename)
        .expect("Something went wrong reading the library file");
    let string = std::str::from_utf8(content).unwrap();
    parse(string)
}

fn parse(prog: &str) -> Library {
    let exp = sexp::parse(prog).expect("Error parsing library");
    Library::from(&exp)
}

// ===============================================
//                  Main Parser
// ===============================================

impl From<&Sexp> for Library {
    fn from(e: &Sexp) -> Self {
        let (_def, e1) = get_str(e);
        let lst = get_rest(&e1);

        let primitives: Vec<Primitive> =
            lst.into_iter().map(|exp| Primitive::from(&exp)).collect();

        Library { primitives }
    }
}

impl From<&Sexp> for Width {
    fn from(e: &Sexp) -> Self {
        match e {
            Atom(_) => panic!("Expected list but got an atom"),
            List(vec) => {
                let head = &vec[0];
                match head {
                    Atom(sexp::Atom::S(str)) => Width::Param {
                        value: String::from(str),
                    },
                    Atom(sexp::Atom::I(i)) => Width::Const { value: *i },
                    _ => panic!("Expected Atom but found: {:?}", e),
                }
            }
        }
    }
}

impl From<&Sexp> for PrimPortdef {
    fn from(e: &Sexp) -> Self {
        let (_port, e1) = get_str(e);
        let (name, e2) = get_str(&e1);
        let width = Width::from(&e2);
        // TODO verify e3 is empty and port == "port"
        PrimPortdef { name, width }
    }
}

impl From<&Sexp> for Primitive {
    fn from(e: &Sexp) -> Self {
        let (_def, e1) = get_str(e);
        let lst = get_rest(&e1);

        let decl = get_rest(&lst[0]);
        let name = sexp_to_str(&decl[0]);
        let params = decl[1..]
            .to_vec()
            .into_iter()
            .map(|exp| sexp_to_str(&exp))
            .collect();

        let inputs = get_rest(&lst[1])
            .into_iter()
            .map(|exp| PrimPortdef::from(&exp))
            .collect();
        let outputs = get_rest(&lst[2])
            .into_iter()
            .map(|exp| PrimPortdef::from(&exp))
            .collect();
        Primitive {
            name,
            params,
            inputs,
            outputs,
        }
    }
}
