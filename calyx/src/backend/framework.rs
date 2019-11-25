use crate::lang::ast;
use crate::lang::ast::{Id, Port};
use crate::lang::library;
use crate::lang::library::ast::Library;
use std::collections::HashMap;

pub struct Context {
    pub instances: HashMap<ast::Id, ast::Structure>,
    pub definitions: HashMap<String, ast::Component>,
    pub library: HashMap<String, library::ast::Primitive>,
}

fn init_library(libs: Vec<String>) -> HashMap<String, library::ast::Primitive> {
    let libraries = libs
        .into_iter()
        .map(|filename| library::parse::parse_file(filename.as_ref()))
        .collect();

    let lib = Library::merge(libraries);
    let mut prim_store: HashMap<String, library::ast::Primitive> =
        HashMap::new();
    for prim in lib.primitives {
        prim_store.insert(prim.name.clone(), prim);
    }
    prim_store
}

impl Context {
    fn init_context(file: String, libs: Vec<String>) -> Context {
        Context {
            instances: HashMap::new(),
            definitions: HashMap::new(),
            library: init_library(libs),
        }
    }

    fn lookup_prim(id: &Id, c: &Context) -> library::ast::Primitive {
        let inst = c.instances.get(id).unwrap();
        match inst {
            ast::Structure::Std { data } => {
                c.library.get(&data.instance.name).unwrap().clone()
            }
            _ => panic!("Prim Not found: {}", id),
        }
    }

    fn lookup_comp(id: &Id, c: &Context) -> ast::Component {
        let inst = c.instances.get(id).unwrap();
        match inst {
            ast::Structure::Decl { data } => {
                c.definitions.get(&data.component).unwrap().clone()
            }
            _ => panic!("Component Not found: {}", id),
        }
    }

    pub fn port_width(p: &Port, comp: &ast::Component, c: &Context) -> i64 {
        match p {
            Port::Comp { component, port } => {
                if c.definitions.contains_key(component) {
                    let comp = Context::lookup_comp(component, c);
                    comp.get_port_width(port)
                } else if c.library.contains_key(component) {
                    let prim = Context::lookup_prim(component, c);
                    let inst = c.instances.get(component).unwrap();
                    prim.get_port_width(inst.clone(), port)
                } else {
                    panic!(
                        "Nonexistent component: Port: {}, Component {}",
                        port, comp.name
                    );
                }
            }
            Port::This { port } => comp.get_port_width(port),
        }
    }
}
