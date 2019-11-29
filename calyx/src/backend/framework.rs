use crate::lang::ast;
use crate::lang::ast::{Component, Namespace, Port, Structure};
use crate::lang::library;
use crate::lang::library::ast::Library;
use crate::lang::parse;
use std::collections::HashMap;

#[allow(unused)]
#[derive(Debug)]
pub struct Context {
    pub toplevel: Component,
    pub instances: HashMap<ast::Id, ast::Structure>,
    pub definitions: HashMap<String, ast::Component>,
    pub library: HashMap<String, library::ast::Primitive>,
}

#[allow(unused)]
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
    pub fn init_context(
        file: String,
        toplevel: String,
        libs: Vec<String>,
    ) -> Context {
        let namespace: Namespace = parse::parse_file(file.as_ref());
        let comp: Component = namespace.get_component(toplevel);
        let store = comp.get_store();
        Context {
            toplevel: comp,
            instances: store,
            definitions: namespace.get_definitions(),
            library: init_library(libs),
        }
    }

    fn lookup_prim(id: &str, c: &Context) -> library::ast::Primitive {
        let inst = c.instances.get(id).unwrap();
        match inst {
            ast::Structure::Std { data } => {
                c.library.get(&data.instance.name).unwrap().clone()
            }
            _ => panic!("Prim Not found: {}", id),
        }
    }

    fn lookup_comp(id: &str, c: &Context) -> ast::Component {
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
                if *comp.name == *component {
                    comp.get_port_width(port.as_ref())
                } else {
                    match c.instances.get(component).unwrap() {
                        Structure::Decl { data } => {
                            let comp =
                                Context::lookup_comp(data.name.as_ref(), c);
                            comp.get_port_width(port)
                        }
                        Structure::Std { data } => {
                            let prim =
                                Context::lookup_prim(data.name.as_ref(), c);
                            let inst = c.instances.get(&data.name).unwrap();
                            prim.get_port_width(inst.clone(), port)
                        }
                        _ => panic!("Wire in component instances store"),
                    }
                }
            }
            Port::This { port } => comp.get_port_width(port),
        }
    }
}
