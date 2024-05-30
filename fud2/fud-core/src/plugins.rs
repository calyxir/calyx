use crate::{
    exec::StateRef,
    run::{EmitRcSetup, EmitResult, Emitter},
    DriverBuilder,
};
use std::{cell::RefCell, rc::Rc};

pub trait LoadPlugins {
    fn load_plugins(self) -> Self;
}

fn to_str_slice(arr: &rhai::Array) -> Vec<String> {
    arr.into_iter()
        .map(|x| x.clone().into_string().unwrap())
        .collect()
    // v.iter()
}

type RhaiResult<T> = Result<T, Box<rhai::EvalAltResult>>;

#[derive(Clone)]
struct RhaiSetupCtx {
    ast: rhai::AST,
    name: String,
}

#[derive(Clone)]
struct RhaiEmitter(Rc<RefCell<Emitter>>);

impl From<Rc<RefCell<Emitter>>> for RhaiEmitter {
    fn from(value: Rc<RefCell<Emitter>>) -> Self {
        Self(value)
    }
}

fn to_rhai_err<E: std::error::Error + 'static>(
    e: E,
) -> Box<rhai::EvalAltResult> {
    Box::new(rhai::EvalAltResult::ErrorSystem(
        "Emitter error:".to_string(),
        Box::new(e),
    ))
}

impl RhaiEmitter {
    fn config_val(&mut self, key: &str) -> RhaiResult<String> {
        Ok(self.0.borrow().config_val(key).map_err(to_rhai_err)?)
    }

    fn config_or(&mut self, key: &str, default: &str) -> String {
        self.0.borrow().config_or(key, default)
    }

    fn config_var(&mut self, name: &str, key: &str) -> RhaiResult<()> {
        Ok(self
            .0
            .borrow_mut()
            .config_var(name, key)
            .map_err(to_rhai_err)?)
    }

    fn config_var_or(
        &mut self,
        name: &str,
        key: &str,
        default: &str,
    ) -> RhaiResult<()> {
        Ok(self
            .0
            .borrow_mut()
            .config_var_or(name, key, default)
            .map_err(to_rhai_err)?)
    }

    fn var(&mut self, name: &str, value: &str) -> RhaiResult<()> {
        Ok(self.0.borrow_mut().var(name, value).map_err(to_rhai_err)?)
    }

    fn rule(&mut self, name: &str, command: &str) -> RhaiResult<()> {
        Ok(self
            .0
            .borrow_mut()
            .rule(name, command)
            .map_err(to_rhai_err)?)
    }

    fn build(
        &mut self,
        rule: &str,
        input: &str,
        output: &str,
    ) -> RhaiResult<()> {
        Ok(self
            .0
            .borrow_mut()
            .build(rule, input, output)
            .map_err(to_rhai_err)?)
    }

    fn comment(&mut self, text: &str) -> RhaiResult<()> {
        Ok(self.0.borrow_mut().comment(text).map_err(to_rhai_err)?)
    }

    fn arg(&mut self, name: &str, value: &str) -> RhaiResult<()> {
        Ok(self.0.borrow_mut().arg(name, value).map_err(to_rhai_err)?)
    }

    fn rsrc(&mut self, filename: &str) -> RhaiResult<()> {
        Ok(self.0.borrow_mut().rsrc(filename).map_err(to_rhai_err)?)
    }
}

impl EmitRcSetup for RhaiSetupCtx {
    fn setup_rc(&self, emitter: Rc<RefCell<Emitter>>) -> EmitResult {
        let mut engine = rhai::Engine::new();
        let mut scope = rhai::Scope::new();

        // annoying that I have to do this for every setup function
        engine
            .register_type_with_name::<RhaiEmitter>("RhaiEmitter")
            .register_fn("config_val", RhaiEmitter::config_val)
            .register_fn("config_or", RhaiEmitter::config_or)
            .register_fn("config_var", RhaiEmitter::config_var)
            .register_fn("config_var_or", RhaiEmitter::config_var_or)
            .register_fn("var", RhaiEmitter::var)
            .register_fn("rule", RhaiEmitter::rule)
            .register_fn("build", RhaiEmitter::build)
            .register_fn("comment", RhaiEmitter::comment)
            .register_fn("arg", RhaiEmitter::arg)
            .register_fn("rsrc", RhaiEmitter::rsrc);

        engine
            .call_fn::<()>(
                &mut scope,
                &self.ast,
                &self.name,
                (RhaiEmitter::from(emitter),),
            )
            .unwrap();

        Ok(())
    }
}

impl LoadPlugins for DriverBuilder {
    fn load_plugins(self) -> Self {
        let this = Rc::new(RefCell::new(self));

        // scope rhai engine code so that all references to `this`
        // are dropped before the end of the function
        {
            let mut engine = rhai::Engine::new();

            // compile the file into an Ast
            let ast = engine.compile_file("test.rhai".into()).unwrap();

            // register functions
            let t = this.clone();
            engine.register_fn(
                "state",
                move |name: &str, extensions: rhai::Array| {
                    let v = to_str_slice(&extensions);
                    let v = v.iter().map(|x| &**x).collect::<Vec<_>>();
                    t.borrow_mut().state(name, &v)
                },
            );

            let t = this.clone();
            engine.register_fn(
                "rule",
                move |setups: rhai::Array,
                      input: StateRef,
                      output: StateRef,
                      rule_name: &str| {
                    let setups = setups
                        .into_iter()
                        .map(|s| s.try_cast::<rhai::FnPtr>().unwrap())
                        .map(|fnptr| {
                            t.borrow_mut().add_setup(
                                &format!("{} setup", fnptr.fn_name()),
                                RhaiSetupCtx {
                                    ast: ast.clone_functions_only(),
                                    name: fnptr.fn_name().to_string(),
                                },
                            )
                        })
                        .collect::<Vec<_>>();
                    t.borrow_mut().rule(&setups, input, output, rule_name)
                },
            );

            // execute a file
            engine.run_file("test.rhai".into()).unwrap();
        }

        Rc::into_inner(this).expect("Back into inner").into_inner()
    }
}
