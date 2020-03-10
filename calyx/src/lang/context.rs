use crate::cmdline::Opts;
use crate::errors;
use crate::lang::pretty_print::PrettyPrint;
use crate::lang::{
    ast, component::Component, library, structure::StructureGraph,
};
use pretty::{termcolor::ColorSpec, RcDoc};
use std::cell::RefCell;
use std::collections::HashMap;

/// Represents an entire Futil program. We use an unsafe `RefCell` for definitions
/// so that we can provide inplace mutable access to the component for Visitors while
/// also allowing Visitors to add definitions. We ensure safety through the Context
/// interface.
#[derive(Debug, Clone)]
pub struct Context {
    definitions: RefCell<HashMap<ast::Id, Component>>,
    library_context: LibraryContext,
}

impl Context {
    pub fn from_opts(opts: &Opts) -> Result<Self, errors::Error> {
        // parse file
        let namespace = ast::parse_file(&opts.file)?;

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

        // gather signatures from all components
        let mut signatures = HashMap::new();
        for comp in &namespace.components {
            signatures.insert(comp.name.clone(), comp.signature.clone());
        }

        let mut definitions = HashMap::new();
        for comp in &namespace.components {
            let prim_sigs = comp.resolve_primitives(&libctx)?;
            let mut graph = StructureGraph::new();
            graph.add_component_def(&comp, &signatures, &prim_sigs)?;
            definitions.insert(
                comp.name.clone(),
                Component {
                    name: comp.name.clone(),
                    signature: comp.signature.clone(),
                    control: comp.control.clone(),
                    structure: graph,
                    resolved_sigs: prim_sigs,
                },
            );
        }

        Ok(Context {
            definitions: RefCell::new(definitions),
            library_context: libctx,
        })
    }

    /// Maps over the context definitions, giving mutable access the components
    pub fn definitions_map(
        &self,
        mut func: impl FnMut(&ast::Id, &mut Component) -> Result<(), errors::Error>,
    ) -> Result<(), errors::Error> {
        self.definitions
            .borrow_mut()
            .iter_mut()
            .map(|(id, comp)| func(id, comp))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct LibraryContext {
    definitions: HashMap<ast::Id, library::ast::Primitive>,
}

impl LibraryContext {
    /// Given the id of a library primitive and a list of values for the params,
    /// attempt to resolve a `ParamSignature` into a `Signature`
    pub fn resolve(
        &self,
        id: &ast::Id,
        params: &[u64],
    ) -> Result<ast::Signature, errors::Error> {
        match self.definitions.get(id) {
            Some(prim) => {
                // zip param ids with passed in params into hashmap
                let param_map: HashMap<&ast::Id, u64> = prim
                    .params
                    .iter()
                    .zip(params)
                    .map(|(id, &width)| (id, width))
                    .collect();
                // resolve inputs
                let inputs_res: Result<Vec<ast::Portdef>, errors::Error> = prim
                    .signature
                    .inputs()
                    .map(|pd| pd.resolve(&param_map))
                    .collect();
                // resolve outputs
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

/* =============== Context Printing ================ */
impl PrettyPrint for Context {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let def = self.definitions.borrow();
        RcDoc::intersperse(
            def.iter()
                .map(|(k, v)| (k.clone(), v.clone()).prettify(&arena)),
            RcDoc::hardline(),
        )
    }
}
