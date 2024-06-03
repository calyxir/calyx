use crate::{
    config,
    exec::{OpRef, SetupRef, StateRef},
    DriverBuilder,
};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use super::{
    error::RhaiSystemError,
    exec_scripts::{to_rhai_err, to_str_slice, RhaiResult, RhaiSetupCtx},
    report::RhaiReport,
};

struct ScriptContext {
    builder: DriverBuilder,
    path: PathBuf,
    ast: rhai::AST,
}

impl ScriptContext {
    /// Take a Rhai array value that is supposed to contain setups and produce
    /// an array of actual references to setups. The array might contain string names
    /// for the setups, or it might be function references that define those setups.
    fn to_setup_refs(
        &mut self,
        ctx: &rhai::NativeCallContext,
        setups: rhai::Array,
    ) -> RhaiResult<Vec<SetupRef>> {
        setups
            .into_iter()
            .map(|s| match s.clone().try_cast::<rhai::FnPtr>() {
                Some(fnptr) => {
                    // TODO: Do we really need to clone stuff here, or can we continue to thread through
                    // the `Rc`?
                    let rctx = RhaiSetupCtx {
                        path: self.path.clone(),
                        ast: self.ast.clone(),
                        name: fnptr.fn_name().to_string(),
                    };
                    Ok(self.builder.add_setup(
                        &format!("{} (plugin)", fnptr.fn_name()),
                        rctx,
                    ))
                }
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
}

struct ScriptRunner {
    ctx: Rc<RefCell<ScriptContext>>,
    engine: rhai::Engine,
}

impl ScriptRunner {
    fn from_file(builder: DriverBuilder, path: &Path) -> Self {
        // Compile the script's source code.
        let engine = rhai::Engine::new();
        let ast = engine.compile_file(path.into()).unwrap();

        // TODO: Consider removing the clones here. We can probably just recover the stuff.
        Self {
            ctx: Rc::new(RefCell::new(ScriptContext {
                builder,
                path: path.into(),
                ast: ast.clone(),
            })),
            engine,
        }
    }

    /// Obtain the wrapped `DriverBuilder`. Panic if other references (from the
    /// script, for example) still exist.
    fn unwrap_builder(self) -> DriverBuilder {
        std::mem::drop(self.engine); // Drop references to the context.
        Rc::into_inner(self.ctx)
            .expect("script references still live")
            .into_inner()
            .builder
    }

    fn reg_state(&mut self) {
        let sctx = self.ctx.clone();
        self.engine.register_fn(
            "state",
            move |name: &str, extensions: rhai::Array| {
                let v = to_str_slice(&extensions);
                let v = v.iter().map(|x| &**x).collect::<Vec<_>>();
                sctx.borrow_mut().builder.state(name, &v)
            },
        );
    }

    fn reg_get_state(&mut self) {
        let sctx = self.ctx.clone();
        self.engine
            .register_fn("get_state", move |state_name: &str| {
                sctx.borrow()
                    .builder
                    .find_state(state_name)
                    .map_err(to_rhai_err)
            });
    }

    fn reg_get_setup(&mut self) {
        let sctx = self.ctx.clone();
        self.engine
            .register_fn("get_setup", move |setup_name: &str| {
                sctx.borrow()
                    .builder
                    .find_setup(setup_name)
                    .map_err(to_rhai_err)
            });
    }

    fn reg_rule(&mut self) {
        let sctx = self.ctx.clone();
        self.engine.register_fn::<_, 4, true, OpRef, true, _>(
            "rule",
            move |ctx: rhai::NativeCallContext,
                  setups: rhai::Array,
                  input: StateRef,
                  output: StateRef,
                  rule_name: &str| {
                let mut sctx = sctx.borrow_mut();
                let setups = sctx.to_setup_refs(&ctx, setups)?;
                Ok(sctx.builder.rule(&setups, input, output, rule_name))
            },
        );
    }

    fn reg_op(&mut self) {
        let sctx = self.ctx.clone();
        self.engine.register_fn::<_, 5, true, OpRef, true, _>(
            "op",
            move |ctx: rhai::NativeCallContext,
                  name: &str,
                  setups: rhai::Array,
                  input: StateRef,
                  output: StateRef,
                  build: rhai::FnPtr| {
                let mut sctx = sctx.borrow_mut();
                let setups = sctx.to_setup_refs(&ctx, setups)?;
                let rctx = RhaiSetupCtx {
                    path: sctx.path.clone(),
                    ast: sctx.ast.clone(),
                    name: build.fn_name().to_string(),
                };
                Ok(sctx.builder.add_op(name, &setups, input, output, rctx))
            },
        );
    }

    /// Register all the builder functions in the engine.
    fn register(&mut self) {
        self.reg_state();
        self.reg_get_state();
        self.reg_get_setup();
        self.reg_rule();
        self.reg_op();
    }

    /// Run the script.
    fn run(&self) {
        let sctx = self.ctx.borrow(); // TODO seems unnecessary?
        self.engine.run_ast(&sctx.ast).report(&sctx.path);
    }

    /// Execute a script from a file, adding to the builder.
    fn run_file(builder: DriverBuilder, path: &Path) -> DriverBuilder {
        let mut runner = ScriptRunner::from_file(builder, path);
        runner.register();
        runner.run();
        runner.unwrap_builder()
    }
}

pub trait LoadPlugins {
    /// Load all the plugins specified in the configuration file.
    fn load_plugins(self) -> Self;
}

impl LoadPlugins for DriverBuilder {
    fn load_plugins(self) -> Self {
        // Get a list of plugins (paths to Rhai scripts) from the config file, if any.
        // TODO: Let's try to avoid loading/parsing the configuration file here and
        // somehow reusing it from wherever we do that elsewhere.
        let config = config::load_config(&self.name);
        let plugin_files = match config.extract_inner::<Vec<PathBuf>>("plugins")
        {
            Ok(v) => v,
            Err(_) => {
                // No plugins to load.
                return self;
            }
        };

        let mut bld = self;
        for path in plugin_files {
            bld = ScriptRunner::run_file(bld, path.as_path());
        }
        bld
    }
}
