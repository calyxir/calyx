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
    /// Maps Ids to in-memory representation of the component.
    definitions: RefCell<HashMap<ast::Id, Component>>,
    /// Library containing primitive definitions.
    library_context: LibraryContext,
    /// Enable debugging output.
    pub debug_mode: bool,
}

impl Context {
    pub fn from_opts(opts: &Opts) -> Result<Self, errors::Error> {
        // parse file
        let namespace = ast::parse_file(&opts.file)?;

        // build hashmap for primitives in provided libraries
        let mut lib_definitions = HashMap::new();
        let libs = &opts.libraries;
        for filename in libs {
            let def = library::ast::parse_file(&filename)?;
            for prim in def.primitives {
                lib_definitions.insert(prim.name.clone(), prim.clone());
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
            debug_mode: opts.enable_debug,
        })
    }

    // XXX(sam) maybe implement this as an iterator?
    /// Iterates over the context definitions, giving mutable access the components
    pub fn definitions_iter(
        &self,
        mut func: impl FnMut(&ast::Id, &mut Component) -> Result<(), errors::Error>,
    ) -> Result<(), errors::Error> {
        self.definitions
            .borrow_mut()
            .iter_mut()
            .map(|(id, comp)| func(id, comp))
            .collect()
    }

    pub fn instantiate_primitive<S: AsRef<str>>(
        &self,
        name: S,
        id: &ast::Id,
        params: &[u64],
    ) -> Result<Component, errors::Error> {
        let sig = self.library_context.resolve(id, params)?;
        Ok(Component::from_signature(name, sig))
    }

    pub fn get_component(
        &self,
        name: &ast::Id,
    ) -> Result<Component, errors::Error> {
        match self.definitions.borrow().get(name) {
            Some(comp) => Ok(comp.clone()),
            None => Err(errors::Error::UndefinedComponent(name.clone())),
        }
    }

    pub fn get_lib_component(
        &self,
        name: &ast::Id,
        params: &[u64],
    ) -> Result<ast::Signature, errors::Error> {
        self.library_context.resolve(name, params)
    }

    pub fn is_lib(&self, name: &ast::Id) -> bool {
        self.library_context.lib_contains(name)
    }

    // XXX(sam) need a way to insert components
}

impl Into<ast::NamespaceDef> for Context {
    fn into(self) -> ast::NamespaceDef {
        let name = "placeholder";
        let mut components: Vec<ast::ComponentDef> = vec![];
        for comp in self.definitions.borrow().values() {
            components.push(comp.clone().into())
        }
        ast::NamespaceDef {
            name: name.into(),
            components,
        }
    }
}

/// Map library signatures to "real" Futil signatures. Since library components
/// can have parameters while futil components cannot, we define helpers methods
/// to make this easier.
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
            None => Err(errors::Error::SignatureResolutionFailed(id.clone())),
        }
    }

    /// Checks whether a component type is in the library or not
    fn lib_contains(&self, id: &ast::Id) -> bool {
        self.definitions.contains_key(id)
    }
}

/* =============== Context Printing ================ */
impl PrettyPrint for Context {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let def = self.definitions.borrow();
        RcDoc::intersperse(
            def.iter()
                .map(|(k, v)| (k.clone(), v.clone()).prettify(&arena)),
            RcDoc::line(),
        )
    }
}
