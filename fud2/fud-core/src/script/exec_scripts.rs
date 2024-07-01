use crate::run::{BufEmitter, EmitBuild, EmitResult, EmitSetup, StreamEmitter};
use camino::Utf8PathBuf;
use once_cell::unsync::Lazy;
use std::{cell::RefCell, path::PathBuf, rc::Rc};

use super::report::RhaiReport;

pub(super) type RhaiResult<T> = Result<T, Box<rhai::EvalAltResult>>;

#[derive(Clone)]
pub(super) struct RhaiEmitter(Rc<RefCell<BufEmitter>>);

impl RhaiEmitter {
    fn with<F>(emitter: &mut StreamEmitter, f: F) -> EmitResult
    where
        F: Fn(Self),
    {
        let buf_emit = emitter.buffer();
        let rhai_emit = Self(Rc::new(RefCell::new(buf_emit)));
        f(rhai_emit.clone());
        let buf_emit = Rc::into_inner(rhai_emit.0).unwrap().into_inner();
        emitter.unbuffer(buf_emit)
    }
}

pub(super) fn to_rhai_err<E: std::error::Error + 'static>(
    e: E,
) -> Box<rhai::EvalAltResult> {
    Box::new(rhai::EvalAltResult::ErrorSystem(
        "Emitter error".to_string(),
        Box::new(e),
    ))
}

pub(super) fn to_str_slice(arr: &rhai::Array) -> Vec<String> {
    arr.iter()
        .map(|x| x.clone().into_string().unwrap())
        .collect()
}

impl RhaiEmitter {
    fn config_val(&mut self, key: &str) -> RhaiResult<String> {
        self.0.borrow().config_val(key).map_err(to_rhai_err)
    }

    fn config_constrained_val(
        &mut self,
        key: &str,
        valid_values: rhai::Array,
    ) -> RhaiResult<String> {
        self.0
            .borrow()
            .config_constrained_val(
                key,
                to_str_slice(&valid_values)
                    .iter()
                    .map(|x| &**x)
                    .collect::<Vec<_>>(),
            )
            .map_err(to_rhai_err)
    }

    fn config_or(&mut self, key: &str, default: &str) -> String {
        self.0.borrow().config_or(key, default)
    }

    fn config_constrained_or(
        &mut self,
        key: &str,
        valid_values: rhai::Array,
        default: &str,
    ) -> RhaiResult<String> {
        self.0
            .borrow()
            .config_constrained_or(
                key,
                to_str_slice(&valid_values)
                    .iter()
                    .map(|x| &**x)
                    .collect::<Vec<_>>(),
                default,
            )
            .map_err(to_rhai_err)
    }

    fn config_var(&mut self, name: &str, key: &str) -> RhaiResult<()> {
        self.0
            .borrow_mut()
            .config_var(name, key)
            .map_err(to_rhai_err)
    }

    fn config_var_or(
        &mut self,
        name: &str,
        key: &str,
        default: &str,
    ) -> RhaiResult<()> {
        self.0
            .borrow_mut()
            .config_var_or(name, key, default)
            .map_err(to_rhai_err)
    }

    fn var(&mut self, name: &str, value: &str) -> RhaiResult<()> {
        self.0.borrow_mut().var(name, value).map_err(to_rhai_err)
    }

    fn rule(&mut self, name: &str, command: &str) -> RhaiResult<()> {
        self.0.borrow_mut().rule(name, command).map_err(to_rhai_err)
    }

    fn build(
        &mut self,
        rule: &str,
        input: &str,
        output: &str,
    ) -> RhaiResult<()> {
        self.0
            .borrow_mut()
            .build(rule, input, output)
            .map_err(to_rhai_err)
    }

    fn build_cmd(
        &mut self,
        targets: rhai::Array,
        rule: &str,
        deps: rhai::Array,
        implicit_deps: rhai::Array,
    ) -> RhaiResult<()> {
        self.0
            .borrow_mut()
            .build_cmd(
                &to_str_slice(&targets)
                    .iter()
                    .map(|x| &**x)
                    .collect::<Vec<_>>(),
                rule,
                &to_str_slice(&deps).iter().map(|x| &**x).collect::<Vec<_>>(),
                &to_str_slice(&implicit_deps)
                    .iter()
                    .map(|x| &**x)
                    .collect::<Vec<_>>(),
            )
            .map_err(to_rhai_err)
    }

    fn comment(&mut self, text: &str) -> RhaiResult<()> {
        self.0.borrow_mut().comment(text).map_err(to_rhai_err)
    }

    #[allow(unused)]
    fn add_file(&mut self, name: &str, contents: &[u8]) -> RhaiResult<()> {
        todo!()
    }

    fn external_path(&mut self, path: &str) -> String {
        let utf8_path = Utf8PathBuf::from(path);
        self.0.borrow().external_path(&utf8_path).into_string()
    }

    fn arg(&mut self, name: &str, value: &str) -> RhaiResult<()> {
        self.0.borrow_mut().arg(name, value).map_err(to_rhai_err)
    }

    fn rsrc(&mut self, filename: &str) -> RhaiResult<()> {
        self.0.borrow_mut().rsrc(filename).map_err(to_rhai_err)
    }
}

thread_local! {
    /// Construct the engine we will use to evaluate setup functions
    /// we do this with Lazy, so that we only create the engine once
    /// this also needs to be thread_local so that we don't need
    /// `rhai::Engine` to be `Sync`.
    static EMIT_ENGINE: Lazy<rhai::Engine> =
        Lazy::new(|| {
            let mut engine = rhai::Engine::new();

            engine
                .register_type_with_name::<RhaiEmitter>("RhaiEmitter")
                .register_fn("config_val", RhaiEmitter::config_val)
                .register_fn("config_constrained_val", RhaiEmitter::config_constrained_val)
                .register_fn("config_or", RhaiEmitter::config_or)
                .register_fn("config_constrained_or", RhaiEmitter::config_constrained_or)
                .register_fn("config_var", RhaiEmitter::config_var)
                .register_fn("config_var_or", RhaiEmitter::config_var_or)
                .register_fn("var_", RhaiEmitter::var)
                .register_fn("rule", RhaiEmitter::rule)
                .register_fn("build", RhaiEmitter::build)
                .register_fn("build_cmd", RhaiEmitter::build_cmd)
                .register_fn("comment", RhaiEmitter::comment)
                .register_fn("add_file", RhaiEmitter::add_file)
                .register_fn("external_path", RhaiEmitter::external_path)
                .register_fn("arg", RhaiEmitter::arg)
                .register_fn("rsrc", RhaiEmitter::rsrc);

            engine
    });
}

#[derive(Clone, Debug)]
pub(super) struct RhaiSetupCtx {
    pub path: Rc<PathBuf>,
    pub ast: Rc<rhai::AST>,
    pub name: String,
}

impl EmitSetup for RhaiSetupCtx {
    fn setup(&self, emitter: &mut StreamEmitter) -> EmitResult {
        RhaiEmitter::with(emitter, |rhai_emit| {
            EMIT_ENGINE.with(|e| {
                e.call_fn::<()>(
                    &mut rhai::Scope::new(),
                    &self.ast,
                    &self.name,
                    (rhai_emit.clone(),),
                )
                .report(self.path.as_ref())
            });
        })?;

        Ok(())
    }
}

impl EmitBuild for RhaiSetupCtx {
    fn build(
        &self,
        emitter: &mut StreamEmitter,
        input: &[&str],
        output: &[&str],
    ) -> EmitResult {
        RhaiEmitter::with(emitter, |rhai_emit| {
            EMIT_ENGINE.with(|e| {
                e.call_fn::<()>(
                    &mut rhai::Scope::new(),
                    &self.ast,
                    &self.name,
                    (
                        rhai_emit.clone(),
                        input[0].to_string(),
                        output[0].to_string(),
                    ),
                )
                .report(self.path.as_ref())
            });
        })?;

        Ok(())
    }
}
