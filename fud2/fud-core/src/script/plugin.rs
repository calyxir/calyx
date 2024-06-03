use crate::{
    config,
    exec::{OpRef, SetupRef, StateRef},
    DriverBuilder,
};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

use super::{
    error::RhaiSystemError,
    exec_scripts::{to_rhai_err, to_str_slice, RhaiResult, RhaiSetupCtx},
    report::RhaiReport,
};

fn to_setup_refs(
    ctx: &rhai::NativeCallContext,
    setups: rhai::Array,
    path: PathBuf,
    ast: rhai::AST,
    this: Rc<RefCell<DriverBuilder>>,
) -> RhaiResult<Vec<SetupRef>> {
    setups
        .into_iter()
        .map(|s| match s.clone().try_cast::<rhai::FnPtr>() {
            Some(fnptr) => Ok(this.borrow_mut().add_setup(
                &format!("{} (plugin)", fnptr.fn_name()),
                RhaiSetupCtx {
                    path: path.clone(),
                    ast: ast.clone(),
                    name: fnptr.fn_name().to_string(),
                },
            )),
            // if we can't cast as a FnPtr, try casting as a SetupRef directly
            None => s.clone().try_cast::<SetupRef>().ok_or_else(move || {
                RhaiSystemError::setup_ref(s)
                    .with_pos(ctx.position())
                    .into()
            }),
        })
        .collect::<RhaiResult<Vec<_>>>()
}

struct ScriptBuilder(Rc<RefCell<DriverBuilder>>);

impl ScriptBuilder {
    fn new(builder: DriverBuilder) -> Self {
        Self(Rc::new(RefCell::new(builder)))
    }

    /// Obtain the wrapped `DriverBuilder`. Panic if other references (from the
    /// script, for example) still exist.
    fn unwrap(self) -> DriverBuilder {
        Rc::into_inner(self.0)
            .expect("script references still live")
            .into_inner()
    }

    fn reg_state(&self, engine: &mut rhai::Engine) {
        let t = self.0.clone();
        engine.register_fn(
            "state",
            move |name: &str, extensions: rhai::Array| {
                let v = to_str_slice(&extensions);
                let v = v.iter().map(|x| &**x).collect::<Vec<_>>();
                t.borrow_mut().state(name, &v)
            },
        );
    }

    fn reg_get_state(&self, engine: &mut rhai::Engine) {
        let t = self.0.clone();
        engine.register_fn("get_state", move |state_name: &str| {
            t.borrow().find_state(state_name).map_err(to_rhai_err)
        });
    }

    fn reg_get_setup(&self, engine: &mut rhai::Engine) {
        let t = self.0.clone();
        engine.register_fn("get_setup", move |setup_name: &str| {
            t.borrow().find_setup(setup_name).map_err(to_rhai_err)
        });
    }

    // TODO: Revisit whether these parameters can be in the struct.
    fn reg_rule(
        &self,
        engine: &mut rhai::Engine,
        path: &PathBuf,
        ast: &rhai::AST,
    ) {
        let t = self.0.clone();
        let rule_ast = ast.clone_functions_only();
        let path_copy = path.clone();
        engine.register_fn::<_, 4, true, OpRef, true, _>(
            "rule",
            move |ctx: rhai::NativeCallContext,
                  setups: rhai::Array,
                  input: StateRef,
                  output: StateRef,
                  rule_name: &str| {
                let setups = to_setup_refs(
                    &ctx,
                    setups,
                    path_copy.clone(),
                    rule_ast.clone(),
                    Rc::clone(&t),
                )?;
                Ok(t.borrow_mut().rule(&setups, input, output, rule_name))
            },
        );
    }

    fn reg_op(
        &self,
        engine: &mut rhai::Engine,
        path: &PathBuf,
        ast: &rhai::AST,
    ) {
        let t = self.0.clone();
        let rule_ast = ast.clone_functions_only();
        let path_copy = path.clone();
        engine.register_fn::<_, 5, true, OpRef, true, _>(
            "op",
            move |ctx: rhai::NativeCallContext,
                  name: &str,
                  setups: rhai::Array,
                  input: StateRef,
                  output: StateRef,
                  build: rhai::FnPtr| {
                let setups = to_setup_refs(
                    &ctx,
                    setups,
                    path_copy.clone(),
                    rule_ast.clone(),
                    Rc::clone(&t),
                )?;
                Ok(t.borrow_mut().add_op(
                    name,
                    &setups,
                    input,
                    output,
                    RhaiSetupCtx {
                        path: path_copy.clone(),
                        ast: rule_ast.clone(),
                        name: build.fn_name().to_string(),
                    },
                ))
            },
        );
    }
}

pub trait LoadPlugins {
    /// Run the scripts in the given paths, adding them to the driver's configuration.
    fn run_scripts(self, paths: &[PathBuf]) -> Self;

    /// Load all the plugins specified in the configuration file.
    fn load_plugins(self) -> Self;
}

impl LoadPlugins for DriverBuilder {
    fn run_scripts(self, paths: &[PathBuf]) -> Self {
        // wrap driver in a ref cell, so that we can call it from a
        // Rhai context
        let bld = ScriptBuilder::new(self);

        // scope rhai engine code so that all references to `this`
        // are dropped before the end of the function
        {
            let mut engine = rhai::Engine::new();

            // register AST independent functions
            bld.reg_state(&mut engine);
            bld.reg_get_state(&mut engine);
            bld.reg_get_setup(&mut engine);

            // go through each plugin file, and execute the script which adds a plugin
            // we need to define the following two functions in the loop because they
            // need the ast of the current file
            for path in paths {
                // compile the file into an Ast
                let ast = engine.compile_file(path.clone()).unwrap();

                bld.reg_rule(&mut engine, &path, &ast);
                bld.reg_op(&mut engine, &path, &ast);

                engine.run_ast(&ast).report(&path);
            }
        }

        bld.unwrap()
    }

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

        self.run_scripts(&plugin_files)
    }
}
