use crate::cmdline::Opts;
use crate::errors;
use crate::lang::pretty_print::PrettyPrint;
use crate::lang::{
    ast, component::Component, library::ast as lib, structure::StructureGraph,
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
    /// Enable debugging output.
    pub debug_mode: bool,
    /// Library containing primitive definitions.
    library_context: LibraryContext,
    /// Maps Ids to in-memory representation of the component.
    definitions: RefCell<HashMap<ast::Id, Component>>,
    /// Keeps track of components that we need to insert. We need
    /// this because `definitions_iter` allows multiple mutable
    /// references to `self.definitions` to be given away. If we
    /// insert components inside a call to `definitions_iter`, things
    /// will break.
    definitions_to_insert: RefCell<Vec<Component>>,
}

impl Context {
    pub fn from_ast(
        namespace: ast::NamespaceDef,
        libs: &[lib::Library],
    ) -> Result<Self, errors::Error> {
        // build hashmap for primitives in provided libraries
        let mut lib_definitions = HashMap::new();
        for def in libs {
            for prim in &def.primitives {
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
            let mut graph = StructureGraph::default();
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
            debug_mode: false,
            library_context: libctx,
            definitions: RefCell::new(definitions),
            definitions_to_insert: RefCell::new(vec![]),
        })
    }

    pub fn from_opts(opts: &Opts) -> Result<Self, errors::Error> {
        // parse file
        let namespace = ast::parse_file(&opts.file)?;

        // parse library files
        let libs: Vec<_> = opts
            .libraries
            .iter()
            .map(lib::parse_file)
            .collect::<Result<Vec<_>, _>>()?;

        // build context
        let mut context = Self::from_ast(namespace, &libs)?;

        // set debug mode according to opts
        context.debug_mode = opts.enable_debug;

        Ok(context)
    }

    // XXX(sam) maybe implement this as an iterator?
    /// Iterates over the context definitions, giving mutable access the components
    pub fn definitions_iter(
        &self,
        mut func: impl FnMut(&ast::Id, &mut Component) -> Result<(), errors::Error>,
    ) -> Result<(), errors::Error> {
        let mut definitions = self.definitions.borrow_mut();

        // do main iteration
        let ret = definitions
            .iter_mut()
            .map(|(id, comp)| func(id, comp))
            .collect();

        // if there are new definitions to insert, insert them now
        let mut defns_to_insert = self.definitions_to_insert.borrow_mut();
        for new_defn in defns_to_insert.drain(..) {
            definitions.insert(new_defn.name.clone(), new_defn);
        }

        ret
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

    /// Insert the component `comp` into `self`.
    pub fn insert_component(&self, comp: Component) {
        // It's possible that this method will be called inside the
        // `definitions_iter` function. In that case, the borrow will
        // fail and we temporarily move `comp` to `self.definitions.to_insert`.
        // When the iteration finishes, `definitions_iter` is responsible for
        // applying these changes. If we successfully borrow `self.definitions`
        // we can insert immediately.
        match self.definitions.try_borrow_mut() {
            Ok(mut defns) => {
                defns.insert(comp.name.clone(), comp);
            }
            Err(_) => self.definitions_to_insert.borrow_mut().push(comp),
        };
    }
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
    definitions: HashMap<ast::Id, lib::Primitive>,
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
}

/* =============== Context Printing ================ */
impl PrettyPrint for Context {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let def = self.definitions.borrow();
        RcDoc::intersperse(
            def.values().map(|x| x.clone().prettify(&arena)),
            RcDoc::line(),
        )
    }
}
