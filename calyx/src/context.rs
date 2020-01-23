use crate::cmdline::Opts;
use crate::errors;
use crate::lang::ast;
use crate::lang::library;
use crate::lang::structure;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Context {
    definitions: HashMap<ast::Id, (ast::Control, structure::StructureGraph)>,
    lib_definitions: HashMap<ast::Id, library::ast::Primitive>,
}

impl Context {
    pub fn from_opts(opts: &Opts) -> Result<Self, errors::Error> {
        // parse file
        let namespace = ast::parse_file(&opts.file)?;

        // build hashmap from components in `namespace`
        let mut definitions = HashMap::new();
        for comp in namespace.components {
            let graph = comp.structure_graph()?;
            definitions.insert(comp.name.clone(), (comp.control, graph));
        }

        // build hashmap for primitives in provided libraries
        let mut lib_definitions = HashMap::new();
        match &opts.libraries {
            Some(libs) => {
                for filename in libs {
                    let def = library::ast::parse_file(&filename)?;
                    for prim in def.primitives {
                        lib_definitions.insert(prim.name.clone(), prim.clone());
                    }
                }
            }
            None => (),
        }

        Ok(Context {
            definitions,
            lib_definitions,
        })
    }
}
