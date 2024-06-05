use itertools::Itertools;

use crate::{
    exec::{OpRef, SetupRef, StateRef},
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
                Some(fnptr) => {
                    let rctx = RhaiSetupCtx {
                        path: Rc::clone(&self.path),
                        ast: Rc::new(self.ast.clone_functions_only()),
                        name: fnptr.fn_name().to_string(),
                    };
                    Ok(self.builder.borrow_mut().add_setup(
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

pub struct ScriptRunner {
    builder: Rc<RefCell<DriverBuilder>>,
    engine: rhai::Engine,
    files: HashMap<PathBuf, rhai::AST>,
}

impl ScriptRunner {
    pub fn new(builder: DriverBuilder) -> Self {
        let mut this = Self {
            builder: Rc::new(RefCell::new(builder)),
            engine: rhai::Engine::new(),
            files: HashMap::default(),
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
        self.files.extend(files.map(|f| {
            (
                f.file_name().unwrap().into(),
                self.engine.compile_file(f).unwrap(),
            )
        }));
        self
    }

    pub fn add_static_files(
        &mut self,
        static_files: impl Iterator<Item = (&'static str, &'static [u8])>,
    ) -> &mut Self {
        self.files.extend(static_files.map(|(name, data)| {
            (
                PathBuf::from(name),
                self.engine
                    .compile(String::from_utf8(data.to_vec()).unwrap())
                    .unwrap(),
            )
        }));
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
            move |name: &str, extensions: rhai::Array| {
                let v = to_str_slice(&extensions);
                let v = v.iter().map(|x| &**x).collect::<Vec<_>>();
                bld.borrow_mut().state(name, &v)
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
        self.engine.register_fn::<_, 4, true, OpRef, true, _>(
            "rule",
            move |ctx: rhai::NativeCallContext,
                  setups: rhai::Array,
                  input: StateRef,
                  output: StateRef,
                  rule_name: &str| {
                let setups = sctx.setups_array(&ctx, setups)?;
                Ok(bld.borrow_mut().rule(&setups, input, output, rule_name))
            },
        );
    }

    fn reg_op(&mut self, sctx: ScriptContext) {
        let bld = Rc::clone(&self.builder);
        self.engine.register_fn::<_, 5, true, OpRef, true, _>(
            "op",
            move |ctx: rhai::NativeCallContext,
                  name: &str,
                  setups: rhai::Array,
                  input: StateRef,
                  output: StateRef,
                  build: rhai::FnPtr| {
                let setups = sctx.setups_array(&ctx, setups)?;
                let rctx = RhaiSetupCtx {
                    path: sctx.path.clone(),
                    ast: Rc::new(sctx.ast.clone_functions_only()),
                    name: build.fn_name().to_string(),
                };
                Ok(bld.borrow_mut().add_op(name, &setups, input, output, rctx))
            },
        );
    }

    fn script_context(&self, path: PathBuf, ast: rhai::AST) -> ScriptContext {
        ScriptContext {
            builder: Rc::clone(&self.builder),
            path: Rc::new(path),
            ast: Rc::new(ast),
        }
    }

    fn run_file(&mut self, path: &Path, ast: rhai::AST) {
        let sctx = self.script_context(path.to_path_buf(), ast);
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
        self.engine
            .set_module_resolver(Resolver::new(self.files.clone()));

        let files: Vec<_> = self
            .files
            .iter()
            .sorted_by_key(|&(p, _)| p)
            .map(|(p, a)| (p.clone(), a.clone()))
            .collect();
        for (p, ast) in files {
            self.run_file(p.as_path(), ast);
        }

        self.into_builder()
    }
}
