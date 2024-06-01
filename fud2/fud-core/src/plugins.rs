use crate::{
    config,
    exec::{SetupRef, StateRef},
    run::{EmitBuild, EmitResult, EmitSetup, Emitter},
    DriverBuilder,
};
use ariadne::{Color, ColorGenerator, Fmt, Label, Report, ReportKind, Source};
use camino::Utf8PathBuf;
use once_cell::unsync::Lazy;
use rhai::{EvalAltResult, FnNamespace};
use serde::ser::Impossible;
use std::{cell::RefCell, ops::Range, path::Path, rc::Rc};
use std::{fs, path::PathBuf};

fn to_str_slice(arr: &rhai::Array) -> Vec<String> {
    arr.iter()
        .map(|x| x.clone().into_string().unwrap())
        .collect()
}

fn to_setup_refs(
    setups: rhai::Array,
    ast: rhai::AST,
    this: Rc<RefCell<DriverBuilder>>,
) -> Vec<SetupRef> {
    setups
        .into_iter()
        .map(|s| match s.clone().try_cast::<rhai::FnPtr>() {
            Some(fnptr) => this.borrow_mut().add_setup(
                &format!("{} (plugin)", fnptr.fn_name()),
                RhaiSetupCtx {
                    ast: ast.clone(),
                    name: fnptr.fn_name().to_string(),
                },
            ),
            // if we can't cast as a FnPtr, try casting as a SetupRef directly
            None => s.try_cast::<SetupRef>().unwrap(),
        })
        .collect::<Vec<_>>()
}

type RhaiResult<T> = Result<T, Box<rhai::EvalAltResult>>;

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
        self.0.borrow().config_val(key).map_err(to_rhai_err)
    }

    fn config_or(&mut self, key: &str, default: &str) -> String {
        self.0.borrow().config_or(key, default)
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

    fn external_path(&mut self, path: &str) -> Utf8PathBuf {
        let utf8_path = Utf8PathBuf::from(path);
        self.0.borrow().external_path(&utf8_path)
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
                .register_fn("config_or", RhaiEmitter::config_or)
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

#[derive(Clone)]
struct RhaiSetupCtx {
    ast: rhai::AST,
    name: String,
}

impl EmitSetup for RhaiSetupCtx {
    fn setup_rc(&self, emitter: Rc<RefCell<Emitter>>) -> EmitResult {
        let mut scope = rhai::Scope::new();

        EMIT_ENGINE.with(|e| {
            e.call_fn::<()>(
                &mut scope,
                &self.ast,
                &self.name,
                (RhaiEmitter::from(emitter),),
            )
            .unwrap()
        });

        Ok(())
    }
}

impl EmitBuild for RhaiSetupCtx {
    fn build_rc(
        &self,
        emitter: Rc<RefCell<Emitter>>,
        input: &str,
        output: &str,
    ) -> EmitResult {
        let mut scope = rhai::Scope::new();

        EMIT_ENGINE.with(|e| {
            e.call_fn::<()>(
                &mut scope,
                &self.ast,
                &self.name,
                (
                    RhaiEmitter::from(emitter),
                    input.to_string(),
                    output.to_string(),
                ),
            )
            .unwrap()
        });

        Ok(())
    }
}

pub trait LoadPlugins {
    fn load_plugins(self) -> Self;
}

pub trait ToReport {
    fn report<P: AsRef<Path>, S: AsRef<str>>(
        &self,
        path: P,
        len: usize,
        msg: S,
    );
}

impl ToReport for rhai::Position {
    fn report<P: AsRef<Path>, S: AsRef<str>>(
        &self,
        path: P,
        len: usize,
        msg: S,
    ) {
        let source =
            fs::read_to_string(path.as_ref()).expect("Failed to open file");
        let name = path.as_ref().to_str().unwrap();

        if let (Some(line), Some(position)) = (self.line(), self.position()) {
            // translate a line offset into a char offset
            let line_offset = source
                .lines()
                // take all the lines up to pos.line()
                .take(line - 1)
                // add one to all the line lengths because `\n` chars are rmeoved with `.lines()`
                .map(|line| line.len() + 1)
                .sum::<usize>();

            // add the column offset to get the beginning of the error
            // we subtract 1, because the positions are 1 indexed
            let err_offset = line_offset + (position - 1);

            Report::build(ReportKind::Error, name, err_offset)
                .with_message("Failed to load plugin")
                .with_label(
                    Label::new((name, err_offset..err_offset + len))
                        .with_message(msg.as_ref().fg(Color::Red)),
                )
                .finish()
                .eprint((name, Source::from(source)))
                .unwrap()
        } else {
            eprintln!("Failed to load plugin {name}");
            eprintln!("  {}", msg.as_ref());
        }
    }
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
                engine.register_fn(
                    "rule",
                    move |setups: rhai::Array,
                          input: StateRef,
                          output: StateRef,
                          rule_name: &str| {
                        let setups = to_setup_refs(
                            setups,
                            rule_ast.clone(),
                            Rc::clone(&t),
                        );
                        t.borrow_mut().rule(&setups, input, output, rule_name)
                    },
                );

                let t = Rc::clone(&this);
                let rule_ast = ast.clone_functions_only();
                engine.register_fn(
                    "op",
                    move |name: &str,
                          setups: rhai::Array,
                          input: StateRef,
                          output: StateRef,
                          build: rhai::FnPtr| {
                        let setups = to_setup_refs(
                            setups,
                            rule_ast.clone(),
                            Rc::clone(&t),
                        );
                        t.borrow_mut().add_op(
                            name,
                            &setups,
                            input,
                            output,
                            RhaiSetupCtx {
                                ast: rule_ast.clone(),
                                name: build.fn_name().to_string(),
                            },
                        );
                    },
                );

                // execute a file
                match engine.run_ast(&ast) {
                    Ok(_) => (),
                    // format error in a nice way
                    Err(x) => match *x {
                        EvalAltResult::ErrorVariableNotFound(variable, pos) => {
                            pos.report(
                                &path,
                                variable.len(),
                                "Undefined variable",
                            )
                        }
                        EvalAltResult::ErrorFunctionNotFound(msg, pos) => {
                            let (fn_name, args) = msg.split_once(' ').unwrap();
                            pos.report(
                                &path,
                                fn_name.len(),
                                format!("{fn_name} {args}"),
                            )
                        }
                        // for errors that we don't have custom processing, just point
                        // to the beginning of the error, and use the error Display as message
                        e => e.position().report(&path, 0, format!("{e}")),
                    },
                }
            }
        }

        Rc::into_inner(this).expect("Back into inner").into_inner()
    }
}
