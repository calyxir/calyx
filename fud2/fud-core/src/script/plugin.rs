use crate::{
    config,
    exec::{SetupRef, StateRef},
    DriverBuilder,
};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

use super::{
    exec_scripts::{to_rhai_err, to_str_slice, RhaiSetupCtx},
    report::RhaiReport,
};

fn to_setup_refs(
    setups: rhai::Array,
    path: PathBuf,
    ast: rhai::AST,
    this: Rc<RefCell<DriverBuilder>>,
) -> Vec<SetupRef> {
    setups
        .into_iter()
        .map(|s| match s.clone().try_cast::<rhai::FnPtr>() {
            Some(fnptr) => this.borrow_mut().add_setup(
                &format!("{} (plugin)", fnptr.fn_name()),
                RhaiSetupCtx {
                    path: path.clone(),
                    ast: ast.clone(),
                    name: fnptr.fn_name().to_string(),
                },
            ),
            // if we can't cast as a FnPtr, try casting as a SetupRef directly
            None => s.try_cast::<SetupRef>().unwrap(),
        })
        .collect::<Vec<_>>()
}

pub trait LoadPlugins {
    fn load_plugins(self) -> Self;
}

impl LoadPlugins for DriverBuilder {
    fn load_plugins(self) -> Self {
        // get list of plugins
        let config = config::load_config(&self.name);
        let plugin_files =
            config.extract_inner::<Vec<PathBuf>>("plugins").unwrap();

        // wrap driver in a ref cell, so that we can call it from a
        // Rhai context
        let this = Rc::new(RefCell::new(self));

        // scope rhai engine code so that all references to `this`
        // are dropped before the end of the function
        {
            let mut engine = rhai::Engine::new();

            // register AST independent functions
            let t = this.clone();
            engine.register_fn(
                "state",
                move |name: &str, extensions: rhai::Array| {
                    let v = to_str_slice(&extensions);
                    let v = v.iter().map(|x| &**x).collect::<Vec<_>>();
                    t.borrow_mut().state(name, &v)
                },
            );

            let t = Rc::clone(&this);
            engine.register_fn("get_state", move |state_name: &str| {
                t.borrow().find_state(state_name).map_err(to_rhai_err)
            });

            let t = Rc::clone(&this);
            engine.register_fn("get_setup", move |setup_name: &str| {
                t.borrow().find_setup(setup_name).map_err(to_rhai_err)
            });

            // go through each plugin file, and execute the script which adds a plugin
            // we need to define the following two functions in the loop because they
            // need the ast of the current file
            for path in plugin_files {
                // compile the file into an Ast
                let ast = engine.compile_file(path.clone()).unwrap();

                let t = Rc::clone(&this);
                let rule_ast = ast.clone_functions_only();
                let path_copy = path.clone();
                engine.register_fn(
                    "rule",
                    move |setups: rhai::Array,
                          input: StateRef,
                          output: StateRef,
                          rule_name: &str| {
                        let setups = to_setup_refs(
                            setups,
                            path_copy.clone(),
                            rule_ast.clone(),
                            Rc::clone(&t),
                        );
                        t.borrow_mut().rule(&setups, input, output, rule_name)
                    },
                );

                let t = Rc::clone(&this);
                let rule_ast = ast.clone_functions_only();
                let path_copy = path.clone();
                engine.register_fn(
                    "op",
                    move |name: &str,
                          setups: rhai::Array,
                          input: StateRef,
                          output: StateRef,
                          build: rhai::FnPtr| {
                        let setups = to_setup_refs(
                            setups,
                            path_copy.clone(),
                            rule_ast.clone(),
                            Rc::clone(&t),
                        );
                        t.borrow_mut().add_op(
                            name,
                            &setups,
                            input,
                            output,
                            RhaiSetupCtx {
                                path: path_copy.clone(),
                                ast: rule_ast.clone(),
                                name: build.fn_name().to_string(),
                            },
                        );
                    },
                );

                engine.run_ast(&ast).report(&path);
            }
        }

        Rc::into_inner(this).expect("Back into inner").into_inner()
    }
}
