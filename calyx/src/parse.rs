use crate::ast::*;
use sexp::Sexp;
use sexp::Sexp::{Atom, List};
use std::fs;

pub fn parse_file(filename: &str) {
    let content = &fs::read_to_string(filename).expect("Something went wrong reading the file");

    let namespace = parse(content);
    println!("{:#?}", namespace);
}

fn parse(prog: &str) -> Namespace {
    match sexp::parse(prog) {
        Ok(exp) => parse_namespace(&exp),
        e => panic!("Error parsing program: {:?}", e),
    }
}

// use lifetime here to specify that the return is borrowing from [e]
fn match_head(e: &Sexp, target: &str) -> Option<Vec<Sexp>> {
    match e {
        Atom(_) => panic!("{:?} is not a head pattern", e),
        List(vec) => match &vec[0] {
            Sexp::Atom(sexp::Atom::S(name)) => {
                if name == target {
                    Some(vec[1..].to_vec())
                } else {
                    None
                }
            }
            _ => None,
        },
    }
}

// XXX(sam) implement this on the Sexp type?
fn atom_to_string(e: &Sexp) -> Option<String> {
    match e {
        Atom(sexp::Atom::S(name)) => Some(name.to_string()),
        _ => None,
    }
}

fn atom_to_i64(e: &Sexp) -> Option<i64> {
    match e {
        Atom(sexp::Atom::I(num)) => Some(*num),
        _ => None,
    }
}

fn parse_namespace(e: &Sexp) -> Namespace {
    let contents = match_head(e, "define/namespace").unwrap();
    Namespace {
        name: atom_to_string(&contents[0]).unwrap(),
        components: contents[1..]
            .into_iter()
            .map(|x| parse_component(x))
            .collect(),
    }
}

fn parse_component(e: &Sexp) -> Component {
    match match_head(e, "define/component").unwrap().as_slice() {
        [name, List(inputs), List(outputs), List(structure), control] => Component {
            name: atom_to_string(name).unwrap(),
            inputs: inputs.into_iter().map(|x| parse_portdef(x)).collect(),
            outputs: outputs.into_iter().map(|x| parse_portdef(x)).collect(),
            structure: structure.into_iter().map(|x| parse_structure(x)).collect(),
            control: parse_control(control),
        },
        _ => panic!("Ill formed component"),
    }
}

fn parse_portdef(e: &Sexp) -> Portdef {
    match e {
        Atom(_) => panic!("Ill formed port"),
        List(vec) => match vec.as_slice() {
            [Atom(sexp::Atom::S(name)), Atom(sexp::Atom::I(width))] => Portdef {
                name: name.to_string(),
                width: *width,
            },
            _ => panic!("Ill formed"),
        },
    }
}

fn parse_structure(e: &Sexp) -> Structure {
    match match_head(e, "new") {
        Some(vec) => match vec.as_slice() {
            [Atom(sexp::Atom::S(name)), Atom(sexp::Atom::S(comp))] => Structure::Decl {
                name: name.to_string(),
                instance: Compinst {
                    name: comp.to_string(),
                    param: vec![],
                },
            },
            [Atom(sexp::Atom::S(name)), List(param_vec)] => Structure::Decl {
                name: name.to_string(),
                instance: parse_param_vec(param_vec),
            },
            _ => panic!("Ill formed"),
        },
        None => match match_head(e, "->").unwrap().as_slice() {
            [src, dest] => Structure::Wire {
                src: parse_port(src),
                dest: parse_port(dest),
            },
            _ => panic!("ill formed"),
        },
    }
}

fn parse_param_vec(e: &Vec<Sexp>) -> Compinst {
    Compinst {
        name: atom_to_string(&e[0]).unwrap(),
        param: e[1..]
            .into_iter()
            .map(|x| atom_to_i64(x).unwrap())
            .collect(),
    }
}

fn parse_port(e: &Sexp) -> Port {
    match match_head(e, "@").unwrap().as_slice() {
        [Atom(sexp::Atom::S(comp)), Atom(sexp::Atom::S(port))] => {
            if comp == "this" {
                Port::This {
                    port: port.to_string(),
                }
            } else {
                Port::Comp {
                    component: comp.to_string(),
                    port: port.to_string(),
                }
            }
        }

        _ => panic!("Ill formed"),
    }
}

fn parse_control(e: &Sexp) -> Control {
    match e {
        Atom(_) => panic!("ill formed"),
        List(list) => {
            let head = &list[0];
            let rest = &list[1..];
            match atom_to_string(&head).unwrap().as_ref() {
                "seq" => Control::Seq(rest.into_iter().map(parse_control).collect()),
                "par" => Control::Par(rest.into_iter().map(parse_control).collect()),
                "if" => match rest {
                    [cond, t, f] => Control::If {
                        cond: parse_port(cond),
                        tbranch: Box::new(parse_control(t)),
                        fbranch: Box::new(parse_control(f)),
                    },
                    _ => panic!("ill formed if"),
                },
                "ifen" => match rest {
                    [cond, t, f] => Control::Ifen {
                        cond: parse_port(cond),
                        tbranch: Box::new(parse_control(t)),
                        fbranch: Box::new(parse_control(f)),
                    },
                    _ => panic!("ill formed ifen"),
                },
                "while" => match rest {
                    [cond, body] => Control::While {
                        cond: parse_port(cond),
                        body: Box::new(parse_control(body)),
                    },
                    _ => panic!("ill formed ifen"),
                },
                "print" => match rest {
                    [Atom(sexp::Atom::S(var))] => Control::Print(var.to_string()),
                    _ => panic!("ill formed ifen"),
                },
                "enable" => Control::Enable(
                    rest.into_iter()
                        .map(|x| atom_to_string(x).unwrap())
                        .collect(),
                ),
                "disable" => Control::Disable(
                    rest.into_iter()
                        .map(|x| atom_to_string(x).unwrap())
                        .collect(),
                ),
                "empty" => Control::Empty,
                _ => panic!("unknown control head"),
            }
        }
    }
}
