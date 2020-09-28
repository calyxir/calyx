use crate::errors::{Error, Result};
use crate::frontend::pretty_print::PrettyPrint;
use crate::lang::{
    ast, ast::Signature, component::Component, library::ast as lib,
    structure::StructureGraph,
};
use pretty::{termcolor::ColorSpec, RcDoc};
use std::cell::RefCell;
use std::collections::HashMap;

/// Represents an entire Futil program. We are keeping all of the components in a `RefCell<HashMap>`.
/// We use the `RefCell` to provide our desired visitor interface
/// where each visitor gets mutable access to it's own component as well as immutable
/// access to the global context to allow looking up definitions and primitives. Mutable
/// access to it's own component is desirable because the structure is represented with a graph
/// and graphs are ill-suited for functional style interfaces.
///
/// However, we also need a way for visitors to add new component definitions to the context.
/// We can't just give the visitor mutable access to the context, because we
/// can't have mutable references to the context and mutable
/// references to the component (owned by the context) alive at the same time. We
/// get around this restriction using `RefCell`s to give a mutable style interface
/// to immutable references to the context.
///
/// `RefCell` is a Rust mechanism that allows an immutable reference to be turned into
/// a mutable reference. For example if we assume that `definitions` doesn't use a `RefCell`,
/// the following is disallowed by Rust:
/// ```rust
/// let mut context = Context::from_opts(&opts)?;
/// let comp = &mut context.definitions["main"];
/// // insert_comp borrows context mutably
/// context.insert_comp(new_comp); // <---- compile time error! can't have two mutable references to the same data
/// // mutate comp here
/// ...
/// ```
///
/// With a `RefCell`, the code looks like this:
///
/// ```rust
/// let context = Context::from_opts(&opts)?; // not declared as mutable
/// let comp = context.definitions.borrow_mut()["main"];
/// // insert_comp borrows context immmutably and uses borrow_mut()
/// // internally to gain mutably
/// context.insert_comp(new_comp); // <---- compiles fine, potentially run time error!
/// // mutate comp here
/// ...
/// ```
///
/// `RefCell`s in essence let us give controlled
/// mutable access to the context. However, we give up on some of Rust's compile-time safety guarantees
/// so we have to make sure to enforce these ourselves. In particular, in `insert_component` we
/// use `try_borrow_mut` to test if another mutable reference is alive. This will happen whenever
/// we call this method from a pass because `definitions_iter` also borrows `definitions` mutably.
/// If the borrow fails, then we put the new component
/// in `definitions_to_insert` instead of putting it in the HashMap directly. After `definitions_iter`
/// is done with it's mutable reference to `definitions`, then it inserts all the new components.
#[derive(Debug, Clone)]
pub struct Context {
    /// Enable debugging output.
    pub debug_mode: bool,
    /// Enable Verilator mode. This tells the backend to generate additional code for loading in memories.
    pub verilator_mode: bool,
    /// Force outputting in color.
    pub force_color: bool,
    /// Library containing primitive definitions.
    pub library_context: LibraryContext,
    /// Maps Ids to in-memory representation of the component.
    definitions: RefCell<HashMap<ast::Id, Component>>,
    /// Keeps track of components that we need to insert. We need
    /// this because `definitions_iter` allows multiple mutable
    /// references to `self.definitions` to be given away. If we
    /// insert components inside a call to `definitions_iter`, things
    /// will break.
    definitions_to_insert: RefCell<Vec<Component>>,
    /// Paths to the import statements. Used by the FuTIL pretty printer.
    imports: Vec<String>,
}
/// Add `go`/`done`/`clk` ports to a signature.
fn extend_sig(mut sig: Signature) -> Signature {
    sig.add_input("go", 1);
    sig.add_input("clk", 1);
    sig.add_output("done", 1);
    sig
}

impl Context {
    /// Generates a Context from a namespace and slice of libraries.
    ///
    /// # Arguments
    ///   * `namespace` - command line options
    ///   * `libs` - slice of library asts
    /// # Returns
    ///   Returns a Context object for the compilation unit,
    ///   or an error.
    pub fn from_ast(
        namespace: ast::NamespaceDef,
        libraries: &[lib::Library],
        debug_mode: bool,
        verilator_mode: bool,
        force_color: bool,
    ) -> Result<Self> {
        // build hashmap for primitives in provided libraries
        let mut lib_definitions = HashMap::new();
        for def in libraries {
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
            signatures
                .insert(comp.name.clone(), extend_sig(comp.signature.clone()));
        }

        let mut definitions = HashMap::new();
        for comp in namespace.components {
            let resolved_sigs = comp.resolve_primitives(&libctx)?;
            let ast::ComponentDef {
                name,
                signature,
                cells,
                connections,
                control,
            } = comp;
            let extended_sig = extend_sig(signature);
            let structure = StructureGraph::new(
                extended_sig.clone(),
                cells,
                connections,
                &signatures,
                &resolved_sigs,
            )?;
            definitions.insert(
                name.clone(),
                Component {
                    name: name.clone(),
                    signature: extended_sig,
                    control,
                    structure,
                    resolved_sigs,
                },
            );
        }

        Ok(Context {
            debug_mode,
            verilator_mode,
            force_color,
            library_context: libctx,
            definitions: RefCell::new(definitions),
            definitions_to_insert: RefCell::new(vec![]),
            imports: namespace.libraries,
        })
    }

    // XXX(sam) maybe implement this as an iterator?
    /// Iterates over the context definitions, giving mutable access the components
    pub fn definitions_iter(
        &self,
        mut func: impl FnMut(&ast::Id, &mut Component) -> Result<()>,
    ) -> Result<()> {
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

    /// Creates a concrete instance of a primitive component.
    /// Because primitive components can take in parameters, this
    /// function attempts to resolve supplied parameters with a
    /// primitive component to create a concrete component.
    ///
    /// # Arguments
    ///   * `name` - the type of primitive component to instance
    ///   * `id` - the identifier for the instance
    ///   * `params` - parameters to pass to the primitive component definition
    /// # Returns
    ///   Returns a concrete Component object or an error.
    pub fn instantiate_primitive<S: AsRef<str>>(
        &self,
        name: S,
        id: &ast::Id,
        params: &[u64],
    ) -> Result<Component> {
        let sig = self.library_context.resolve(id, params)?;
        Ok(Component::from_signature(name, sig))
    }

    /// Looks up the component for a component instance id.
    /// Does not provide mutable access to the Context.
    ///
    /// # Arguments
    ///   * `id` - the identifier for the instance
    /// # Returns
    ///   Returns the Component corresponding to `id` or an error.
    pub fn get_component(&self, id: &ast::Id) -> Result<Component> {
        match self.definitions.borrow().get(id) {
            Some(comp) => Ok(comp.clone()),
            None => Err(Error::UndefinedComponent(id.clone())),
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
        let mut components: Vec<ast::ComponentDef> = vec![];
        for comp in self.definitions.borrow().values() {
            components.push(comp.clone().into())
        }
        components.sort();
        ast::NamespaceDef {
            components,
            libraries: self.imports,
        }
    }
}

/// Map library signatures to "real" Futil signatures. Since library components
/// can have parameters while futil components cannot, we define helpers methods
/// to make this easier.
#[derive(Debug, Clone)]
pub struct LibraryContext {
    pub definitions: HashMap<ast::Id, lib::Primitive>,
}

impl LibraryContext {
    /// Given the id of a library primitive and a list of values for the params,
    /// attempt to resolve a `ParamSignature` into a `Signature`
    pub fn resolve(
        &self,
        id: &ast::Id,
        params: &[u64],
    ) -> Result<ast::Signature> {
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
                let inputs_res: Result<Vec<ast::Portdef>> = prim
                    .signature
                    .inputs()
                    .map(|pd| pd.resolve(&id, &param_map))
                    .collect();
                // resolve outputs
                let outputs_res: Result<Vec<ast::Portdef>> = prim
                    .signature
                    .outputs()
                    .map(|pd| pd.resolve(&id, &param_map))
                    .collect();
                let inputs = inputs_res?;
                let outputs = outputs_res?;
                Ok(ast::Signature { inputs, outputs })
            }
            None => Err(Error::UndefinedComponent(id.clone())),
        }
    }
}

/* =============== Context Printing ================ */
impl PrettyPrint for Context {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let namespace: ast::NamespaceDef = self.clone().into();
        namespace.prettify(&arena)
    }
}
