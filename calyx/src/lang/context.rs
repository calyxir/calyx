use crate::cmdline::Opts;
use crate::errors;
use crate::lang::ast;
use crate::lang::library;
use crate::lang::structure;
use std::collections::HashMap;

#[derive(Debug)]
struct Component {
    signature: ast::Signature,
    control: ast::Control,
    structure: structure::StructureGraph,
}

// impl Component {
//     pub fn
// }

#[derive(Debug)]
pub struct Context {
    definitions: HashMap<ast::Id, Component>,
    lib_definitions: HashMap<ast::Id, library::ast::Primitive>,
}

impl Context {
    pub fn from_opts(opts: &Opts) -> Result<Self, errors::Error> {
        // parse file
        let namespace = ast::parse_file(&opts.file)?;

        // build a map from ids to component signatures
        let mut signatures = HashMap::new();
        for comp in namespace.components {
            signatures.insert(comp.name, comp.signature);
        }

        // build hashmap from components in `namespace`
        let mut definitions = HashMap::new();
        for comp in namespace.components {
            let graph = comp.structure_graph(&signatures)?;
            definitions.insert(
                comp.name.clone(),
                Component {
                    signature: comp.signature,
                    control: comp.control,
                    structure: graph,
                },
            );
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

    pub fn add_component(&mut self, comp: Component) {}
}
