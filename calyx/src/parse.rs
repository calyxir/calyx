use crate::ast::*;
use sexp::Sexp;
use sexp::Sexp::{Atom, List};
use std::fs;

pub fn parse_file(filename: &str) {
    let content = &fs::read_to_string(filename).expect("Something went wrong reading the file");

    let namespace = parse(content);
    println!("{:#?}", namespace);
}

// ===============================================
//             Parsing Helper Functions
// ===============================================

/**
 * Converts a Sexp library s-expression to a string
 */
fn sexp_to_str(e: &Sexp) -> String {
    match e {
        Atom(sexp::Atom::S(str)) => return String::from(str),
        _ => panic!("Error: {:?}", e),
    }
}

/**
 * Converts a Sexp library s-expression to an int
 */
fn sexp_to_int(e: &Sexp) -> i64 {
    match e {
        Atom(sexp::Atom::I(i)) => return *i,
        _ => panic!("Error: {:?}", e),
    }
}

/**
 * Grabs the first element in a Sexp List and converts
 * it to a string, if possible. Returns the string and the
 * remaining s-expressions
 */
fn get_str(e: &Sexp) -> (String, Sexp) {
    match e {
        Atom(_) => panic!("Error: {:?}", e),
        List(vec) => {
            let head = &vec[0];
            let tail = List(vec[1..].to_vec());
            return (sexp_to_str(head), tail);
        }
    }
}

/**
 * Grabs the first element in a Sexp List and converts
 * it to an int, if possible. Returns the int and the
 * remaining s-expressions
 */
fn get_int(e: &Sexp) -> (i64, Sexp) {
    match e {
        Atom(_) => panic!("Error: {:?}", e),
        List(vec) => {
            let head = &vec[0];
            let tail = List(vec[1..].to_vec());
            return (sexp_to_int(head), tail);
        }
    }
}

/**
 * Unboxes an Sexp into a Vector of S expressions, if it
 * has the proper type.
 */
fn get_rest(e: &Sexp) -> Vec<Sexp> {
    match e {
        Atom(_) => panic!("Error: {:?}", e),
        List(vec) => {
            return vec.clone();
        }
    }
}

// ===============================================
//                  Main Parser
// ===============================================

impl From<&Sexp> for Portdef {
    fn from(e: &Sexp) -> Self {
        let (port, e1) = get_str(e);
        let (name, e2) = get_str(&e1);
        let (width, e3) = get_int(&e2);
        // TODO verify e3 is empty and port == "port"
        return Portdef {
            name: name,
            width: width,
        };
    }
}

impl From<&Sexp> for Port {
    fn from(e: &Sexp) -> Self {
        let (name, e1) = get_str(e);
        let (component, e2) = get_str(&e1);
        let (port, e3) = get_str(&e2);
        // TODO error checking
        if port == "this" {
            return Port::This { port: port };
        } else {
            return Port::Comp {
                component: component,
                port: port,
            };
        }
    }
}

impl From<&Sexp> for Compinst {
    fn from(e: &Sexp) -> Self {
        let (name, e1) = get_str(e);
        let lst = get_rest(&e1);
        let params = lst.into_iter().map(|exp| sexp_to_int(&exp)).collect();
        return Compinst {
            name: name,
            params: params,
        };
    }
}

impl From<&Sexp> for Structure {
    fn from(e: &Sexp) -> Self {
        let (s, e1) = get_str(e);
        let lst = get_rest(&e1);
        match s.as_ref() {
            "new" => {
                let name = sexp_to_str(&lst[0]);
                let comp = sexp_to_str(&lst[1]);
                return Structure::Decl {
                    name: name,
                    component: comp,
                };
            }
            "new-std" => {
                let name = sexp_to_str(&lst[0]);
                let inst = Compinst::from(&lst[1]);
                return Structure::Std {
                    name: name,
                    instance: inst,
                };
            }
            "->" => {
                let src = Port::from(&lst[0]);
                let dest = Port::from(&lst[1]);
                return Structure::Wire {
                    src: src,
                    dest: dest,
                };
            }
            _ => {
                panic!("AHHH in structure");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portdef_test() {
        let s = "( port hi 3)";
        println!("{}", s);
        match sexp::parse(s) {
            Ok(e) => {
                let pd = Portdef::from(&e);
                println!("{:#?}", pd);
                assert_eq!(pd.name, "hi");
                assert_eq!(pd.width, 3);
            }
            Err(e) => {
                println!("Error: {:#?}", e);
            }
        }
    }
}

// ===============================================
//                  Main Parser
// ===============================================
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
            structure: structure.into_iter().map(|x| Structure::from(x)).collect(),
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
