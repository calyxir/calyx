use crate::lang::ast::*;
use crate::lang::utils::*;
use sexp::Sexp;
use std::fs;

pub fn parse_file(filename: &str) -> Namespace {
    let content = &fs::read_to_string(filename)
        .expect("Something went wrong reading the file");
    parse(content)
}

// ===============================================
//                  Main Parser
// ===============================================

impl From<&Sexp> for Portdef {
    fn from(e: &Sexp) -> Self {
        let (_port, e1) = get_str(e);
        let (name, e2) = get_str(&e1);
        let (width, _e3) = get_int(&e2);
        // TODO verify e3 is empty and port == "port"
        Portdef { name, width }
    }
}

impl From<&Sexp> for Port {
    fn from(e: &Sexp) -> Self {
        let (_at, e1) = get_str(e);
        let (component, e2) = get_str(&e1);
        let (port, _e3) = get_str(&e2);
        // TODO error checking
        if component == "this" {
            Port::This { port }
        } else {
            Port::Comp { component, port }
        }
    }
}

impl From<&Sexp> for Compinst {
    fn from(e: &Sexp) -> Self {
        let (name, e1) = get_str(e);
        let lst = get_rest(&e1);
        let params = lst.into_iter().map(|exp| sexp_to_int(&exp)).collect();
        Compinst { name, params }
    }
}

impl From<Vec<Sexp>> for Decl {
    fn from(e: Vec<Sexp>) -> Self {
        Decl {
            name: sexp_to_str(&e[0]),
            component: sexp_to_str(&e[1]),
        }
    }
}

impl From<Vec<Sexp>> for Std {
    fn from(e: Vec<Sexp>) -> Self {
        Std {
            name: sexp_to_str(&e[0]),
            instance: Compinst::from(&e[1]),
        }
    }
}

impl From<Vec<Sexp>> for Wire {
    fn from(e: Vec<Sexp>) -> Self {
        Wire {
            src: Port::from(&e[0]),
            dest: Port::from(&e[1]),
        }
    }
}

impl From<&Sexp> for Structure {
    fn from(e: &Sexp) -> Self {
        let (s, e1) = get_str(e);
        let lst = get_rest(&e1);
        match s.as_ref() {
            "new" => Structure::Decl {
                data: Decl::from(lst),
            },
            "new-std" => Structure::Std {
                data: Std::from(lst),
            },
            "->" => Structure::Wire {
                data: Wire::from(lst),
            },
            _ => panic!("AHHH in structure"),
        }
    }
}

impl From<Vec<Sexp>> for Seq {
    fn from(e: Vec<Sexp>) -> Self {
        Seq {
            stmts: e.into_iter().map(|e| Control::from(&e)).collect(),
        }
    }
}

impl From<Vec<Sexp>> for Par {
    fn from(e: Vec<Sexp>) -> Self {
        Par {
            stmts: e.into_iter().map(|e| Control::from(&e)).collect(),
        }
    }
}

impl From<Vec<Sexp>> for If {
    fn from(e: Vec<Sexp>) -> Self {
        If {
            cond: Port::from(&e[0]),
            tbranch: Box::new(Control::from(&e[1])),
            fbranch: Box::new(Control::from(&e[2])),
        }
    }
}

impl From<Vec<Sexp>> for Ifen {
    fn from(e: Vec<Sexp>) -> Self {
        Ifen {
            cond: Port::from(&e[0]),
            tbranch: Box::new(Control::from(&e[1])),
            fbranch: Box::new(Control::from(&e[2])),
        }
    }
}

impl From<Vec<Sexp>> for While {
    fn from(e: Vec<Sexp>) -> Self {
        While {
            cond: Port::from(&e[0]),
            body: Box::new(Control::from(&e[1])),
        }
    }
}

impl From<Vec<Sexp>> for Print {
    fn from(e: Vec<Sexp>) -> Self {
        Print {
            var: sexp_to_str(&e[0]),
        }
    }
}

impl From<Vec<Sexp>> for Enable {
    fn from(e: Vec<Sexp>) -> Self {
        Enable {
            comps: e.into_iter().map(|exp| sexp_to_str(&exp)).collect(),
        }
    }
}

impl From<Vec<Sexp>> for Disable {
    fn from(e: Vec<Sexp>) -> Self {
        Disable {
            comps: e.into_iter().map(|exp| sexp_to_str(&exp)).collect(),
        }
    }
}

impl From<&Sexp> for Control {
    fn from(e: &Sexp) -> Self {
        let (keyword, e1) = get_str(e);
        let lst = get_rest(&e1);
        match keyword.as_ref() {
            "seq" => Control::Seq {
                data: Seq::from(lst),
            },
            "par" => Control::Par {
                data: Par::from(lst),
            },
            "if" => Control::If {
                data: If::from(lst),
            },
            "ifen" => Control::Ifen {
                data: Ifen::from(lst),
            },
            "while" => Control::While {
                data: While::from(lst),
            },
            "print" => Control::Print {
                data: Print::from(lst),
            },
            "enable" => Control::Enable {
                data: Enable::from(lst),
            },
            "disable" => Control::Disable {
                data: Disable::from(lst),
            },
            "empty" => Control::Empty { data: Empty {} },
            _ => panic!("Unexpected Control Keyword!"),
        }
    }
}

impl From<&Sexp> for Component {
    fn from(e: &Sexp) -> Self {
        let (_def, e1) = get_str(e);
        let lst = get_rest(&e1);

        let name = sexp_to_str(&lst[0]);
        let inputs = get_rest(&lst[1])
            .into_iter()
            .map(|exp| Portdef::from(&exp))
            .collect();
        let outputs = get_rest(&lst[2])
            .into_iter()
            .map(|exp| Portdef::from(&exp))
            .collect();
        let structure = get_rest(&lst[3])
            .into_iter()
            .map(|exp| Structure::from(&exp))
            .collect();
        let control = Control::from(&lst[4]);
        Component {
            name,
            inputs,
            outputs,
            structure,
            control,
        }
    }
}

impl From<&Sexp> for Namespace {
    fn from(e: &Sexp) -> Self {
        let (_def, e1) = get_str(e);
        let lst = get_rest(&e1);

        let name = sexp_to_str(&lst[0]);
        let components: Vec<Component> = lst[1..]
            .to_vec()
            .into_iter()
            .map(|exp| Component::from(&exp))
            .collect();

        Namespace { name, components }
    }
}

fn parse(prog: &str) -> Namespace {
    match sexp::parse(prog) {
        Ok(exp) => Namespace::from(&exp),
        e => panic!("Error parsing program: {:?}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Test Strings
    const PORTDEF1: &str = "( port hi 3 )";
    const PORT1: &str = "( @ this input_port)";
    const PORT2: &str = "( @ c1 in2 )";
    const COMPINST1: &str = "( comp 1 2 3 4 5 )";

    #[test]
    fn portdef_test() {
        match sexp::parse(PORTDEF1) {
            Ok(e) => {
                let pd = Portdef::from(&e);
                println!("{:#?}", pd);
                assert_eq!(pd.name, "hi");
                assert_eq!(pd.width, 3);
            }
            Err(_) => {
                panic!("Error parsing string");
            }
        }
    }

    #[test]
    fn port_test1() {
        match sexp::parse(PORT1) {
            Ok(e) => {
                let p = Port::from(&e);
                println!("{:#?}", p);
                match p {
                    Port::This { port } => assert_eq!(port, "input_port"),
                    _ => panic!("Parsed Wrong AST Type"),
                }
            }
            Err(_) => panic!("Error parsing string"),
        }
    }
    #[test]
    fn port_test2() {
        match sexp::parse(PORT2) {
            Ok(e) => {
                let p = Port::from(&e);
                println!("{:#?}", p);
                match p {
                    Port::Comp { component, port } => {
                        assert_eq!(port, "in2");
                        assert_eq!(component, "c1");
                    }
                    _ => panic!("Parsed Wrong AST Type"),
                }
            }
            Err(_) => panic!("Error parsing string"),
        }
    }

    #[test]
    fn compinst_test1() {
        match sexp::parse(COMPINST1) {
            Ok(e) => {
                let p = Compinst::from(&e);
                println!("{:#?}", p);
                assert_eq!(p.name, "comp");
                assert_eq!(p.params, [1, 2, 3, 4, 5]);
            }
            Err(_) => panic!("Error parsing string"),
        }
    }
}
