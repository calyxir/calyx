use crate::{
    exec::{SetupRef, StateRef},
    DriverBuilder,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
};

use super::{
    error::RhaiSystemError,
    exec_scripts::{to_rhai_err, to_str_slice, RhaiResult, RhaiSetupCtx},
    report::RhaiReport,
    resolver::Resolver,
};

#[derive(Clone)]
struct ScriptContext {
    builder: Rc<RefCell<DriverBuilder>>,
    path: Rc<PathBuf>,
    ast: Rc<rhai::AST>,
    setups: Rc<RefCell<HashMap<String, SetupRef>>>,
}

impl ScriptContext {
    /// Take a Rhai array value that is supposed to contain setups and produce
    /// an array of actual references to setups. The array might contain string names
    /// for the setups, or it might be function references that define those setups.
    fn setups_array(
        &self,
        ctx: &rhai::NativeCallContext,
        setups: rhai::Array,
    ) -> RhaiResult<Vec<SetupRef>> {
        setups
            .into_iter()
            .map(|s| match s.clone().try_cast::<rhai::FnPtr>() {
                Some(fnptr) => Ok(self.make_or_get_setupref(fnptr)),
                // if we can't cast as a FnPtr, try casting as a SetupRef directly
                None => {
                    s.clone().try_cast::<SetupRef>().ok_or_else(move || {
                        RhaiSystemError::setup_ref(s)
                            .with_pos(ctx.position())
                            .into()
                    })
                }
            })
            .collect::<RhaiResult<Vec<_>>>()
    }

    /// Construct a SetupRef for a rhai function. If we already have a SetupRef,
    /// return the previously constructed version, otherwise make a new one, cache it
    /// and return that.
    fn make_or_get_setupref(&self, fnptr: rhai::FnPtr) -> SetupRef {
        // if we haven't seen this fnptr before, make a new setup context
        // for this function
        if !self.setups.borrow().contains_key(fnptr.fn_name()) {
            let rctx = RhaiSetupCtx {
                path: Rc::clone(&self.path),
                ast: Rc::new(self.ast.clone_functions_only()),
                name: fnptr.fn_name().to_string(),
            };
            let setup_ref = self
                .builder
                .borrow_mut()
                .add_setup(&format!("{} (plugin)", fnptr.fn_name()), rctx);
            self.setups
                .borrow_mut()
                .insert(fnptr.fn_name().to_string(), setup_ref);
        }

        *self.setups.borrow().get(fnptr.fn_name()).unwrap()
    }
}

pub struct ScriptRunner {
    builder: Rc<RefCell<DriverBuilder>>,
    engine: rhai::Engine,
    rhai_functions: rhai::AST,
    resolver: Option<Resolver>,
    setups: Rc<RefCell<HashMap<String, SetupRef>>>,
}

impl ScriptRunner {
    pub fn new(builder: DriverBuilder) -> Self {
        let mut this = Self {
            builder: Rc::new(RefCell::new(builder)),
            engine: rhai::Engine::new(),
            rhai_functions: rhai::AST::empty(),
            resolver: Some(Resolver::default()),
            setups: Rc::default(),
        };
        this.reg_state();
        this.reg_get_state();
        this.reg_get_setup();
        this
    }

    pub fn add_files(
        &mut self,
        files: impl Iterator<Item = PathBuf>,
    ) -> &mut Self {
        for f in files {
            let ast = self.engine.compile_file(f.clone()).unwrap();
            let functions =
                self.resolver.as_mut().unwrap().register_path(f, ast);
            self.rhai_functions = self.rhai_functions.merge(&functions);
        }
        self
    }

    pub fn add_static_files(
        &mut self,
        static_files: impl Iterator<Item = (&'static str, &'static [u8])>,
    ) -> &mut Self {
        for (name, data) in static_files {
            let ast = self
                .engine
                .compile(String::from_utf8(data.to_vec()).unwrap())
                .unwrap();
            let functions =
                self.resolver.as_mut().unwrap().register_data(name, ast);
            self.rhai_functions = self.rhai_functions.merge(&functions);
        }
        self
    }

    fn into_builder(self) -> DriverBuilder {
        std::mem::drop(self.engine); // Drop references to the context.
        Rc::into_inner(self.builder)
            .expect("script references still live")
            .into_inner()
    }

    fn reg_state(&mut self) {
        let bld = Rc::clone(&self.builder);
        self.engine.register_fn(
            "state",
            move |ctx: rhai::NativeCallContext,
                  name: &str,
                  extensions: rhai::Array| {
                let v = to_str_slice(&extensions);
                let v = v.iter().map(|x| &**x).collect::<Vec<_>>();
                let state = bld.borrow_mut().state(name, &v);

                #[cfg(not(debug_assertions))]
                // use ctx when we build in release mode
                // so that we don't get a warning
                {
                    _ = ctx;
                }

                // try to set state source
                #[cfg(debug_assertions)]
                if let Some(src) =
                    ctx.global_runtime_state().source().and_then(|p| {
                        PathBuf::from(p)
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                    })
                {
                    bld.borrow_mut().state_source(state, src);
                }
                state
            },
        );
    }

    fn reg_get_state(&mut self) {
        let bld = Rc::clone(&self.builder);
        self.engine
            .register_fn("get_state", move |state_name: &str| {
                bld.borrow().find_state(state_name).map_err(to_rhai_err)
            });
    }

    fn reg_get_setup(&mut self) {
        let bld = Rc::clone(&self.builder);
        self.engine
            .register_fn("get_setup", move |setup_name: &str| {
                bld.borrow().find_setup(setup_name).map_err(to_rhai_err)
            });
    }

    fn reg_rule(&mut self, sctx: ScriptContext) {
        let bld = Rc::clone(&self.builder);
        self.engine.register_fn(
            "rule",
            move |ctx: rhai::NativeCallContext,
                  setups: rhai::Array,
                  input: StateRef,
                  output: StateRef,
                  rule_name: &str|
                  -> RhaiResult<_> {
                let setups = sctx.setups_array(&ctx, setups)?;
                let op =
                    bld.borrow_mut().rule(&setups, input, output, rule_name);

                // try to set op source
                #[cfg(debug_assertions)]
                if let Some(name) = sctx.path.file_name() {
                    bld.borrow_mut().op_source(op, name.to_string_lossy());
                }
                Ok(op)
            },
        );
    }

    fn reg_op(&mut self, sctx: ScriptContext) {
        let bld = Rc::clone(&self.builder);
        self.engine.register_fn(
            "op",
            move |ctx: rhai::NativeCallContext,
                  name: &str,
                  setups: rhai::Array,
                  input: StateRef,
                  output: StateRef,
                  build: rhai::FnPtr|
                  -> RhaiResult<_> {
                let setups = sctx.setups_array(&ctx, setups)?;
                let rctx = RhaiSetupCtx {
                    path: sctx.path.clone(),
                    ast: Rc::new(sctx.ast.clone_functions_only()),
                    name: build.fn_name().to_string(),
                };
                let op = bld.borrow_mut().add_op(
                    name,
                    &setups,
                    &[input],
                    &[output],
                    rctx,
                );

                // try to set op source
                #[cfg(debug_assertions)]
                if let Some(name) = sctx.path.file_name() {
                    bld.borrow_mut().op_source(op, name.to_string_lossy());
                }
                Ok(op)
            },
        );
    }

    fn script_context(&self, path: PathBuf) -> ScriptContext {
        ScriptContext {
            builder: Rc::clone(&self.builder),
            path: Rc::new(path),
            ast: Rc::new(self.rhai_functions.clone()),
            setups: Rc::clone(&self.setups),
        }
    }

    fn run_file(&mut self, path: &Path) {
        let sctx = self.script_context(path.to_path_buf());
        self.reg_rule(sctx.clone());
        self.reg_op(sctx.clone());

        self.engine
            .module_resolver()
            .resolve(
                &self.engine,
                None,
                path.to_str().unwrap(),
                rhai::Position::NONE,
            )
            .report(path);
    }

    pub fn run(mut self) -> DriverBuilder {
        // take ownership of the resolver
        let resolver = self.resolver.take().unwrap();
        // grab the paths from the resolver
        let paths = resolver.paths();
        // set our engine to use this resolver
        self.engine.set_module_resolver(resolver);

        // run all the paths we've registered
        for p in paths {
            self.run_file(p.as_path());
        }

        // transform self back into a DriverBuilder
        self.into_builder()
    }
}
