use crate::lang::ast;
use crate::lang::component;
use crate::lang::library;
use crate::lang::library::ast::Library;
use std::collections::HashMap;

struct Context {
    instances: HashMap<ast::Id, ast::Structure>,
    definitions: HashMap<String, ast::Component>,
    library: HashMap<String, library::ast::Primitive>,
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

trait Backend {
    fn init_context(libs: Vec<String>) -> Context {
        Context {
            instances: HashMap::new(),
            definitions: HashMap::new(),
            library: init_library(libs),
        }
    }

    fn perform_passes(c: ast::Component) -> ast::Component;
    fn run_backend();
}
