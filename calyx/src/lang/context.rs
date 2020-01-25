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
    library_context: LibraryContext,
}

#[derive(Debug)]
pub struct LibraryContext {
    definitions: HashMap<ast::Id, library::ast::Primitive>,
}

impl LibraryContext {
    pub fn resolve(
        &self,
        id: &ast::Id,
        params: &[i64],
    ) -> Result<ast::Signature, errors::Error> {
        match self.definitions.get(id) {
            Some(prim) => {
                // zip param ids with passed in params into hashmap
                let param_map: HashMap<&ast::Id, i64> = prim
                    .params
                    .iter()
                    .zip(params)
                    .map(|(id, &width)| (id, width))
                    .collect();
                let inputs_res: Result<Vec<ast::Portdef>, errors::Error> = prim
                    .signature
                    .inputs()
                    .map(|pd| pd.resolve(&param_map))
                    .collect();
                let outputs_res: Result<Vec<ast::Portdef>, errors::Error> =
                    prim.signature
                        .outputs()
                        .map(|pd| pd.resolve(&param_map))
                        .collect();
                let inputs = inputs_res?;
                let outputs = outputs_res?;
                Ok(ast::Signature { inputs, outputs })
            }
            None => {
                Err(errors::Error::SignatureResolutionFailed(id.to_string()))
            }
        }
    }
}

impl Context {
    pub fn from_opts(opts: &Opts) -> Result<Self, errors::Error> {
        // parse file
        let namespace = ast::parse_file(&opts.file)?;

        // build a map from ids to component signatures
        let mut signatures = HashMap::new();
        for comp in &namespace.components {
            signatures.insert(comp.name.clone(), comp.signature.clone());
        }

        // build hashmap for primitives in provided libraries
        let mut lib_definitions = HashMap::new();
        if let Some(libs) = &opts.libraries {
            for filename in libs {
                let def = library::ast::parse_file(&filename)?;
                for prim in def.primitives {
                    lib_definitions.insert(prim.name.clone(), prim.clone());
                }
            }
        }
        let libctx = LibraryContext {
            definitions: lib_definitions,
        };

        // build hashmap from components in `namespace`
        let mut definitions = HashMap::new();
        for comp in &namespace.components {
            let graph = comp.structure_graph(&signatures, &libctx)?;
            definitions.insert(
                comp.name.clone(),
                Component {
                    signature: comp.signature.clone(),
                    control: comp.control.clone(),
                    structure: graph,
                },
            );
        }

        Ok(Context {
            definitions,
            library_context: libctx,
        })
    }

    // pub fn add_component(&mut self, comp: Component) {}
}
