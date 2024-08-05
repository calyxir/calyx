use rhai::{Dynamic, ImmutableString, ParseError, Position};

use crate::{
    exec::{SetupRef, StateRef},
    DriverBuilder,
};
use std::{
    cell::{RefCell, RefMut},
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

/// The signature and implementation of an operation specified in Rhai.
struct RhaiOp {
    /// Operation name.
    name: String,
    /// Inputs states of the op.
    input_states: Vec<StateRef>,
    /// Output states of the op.
    output_states: Vec<StateRef>,
    /// An ordered list of the commands run when this op is required.
    cmds: Vec<String>,
    /// A list of the values required from the config.
    config_vars: Vec<crate::run::ConfigVar>,
}

#[derive(Clone)]
struct ScriptContext {
    builder: Rc<RefCell<DriverBuilder>>,
    path: Rc<PathBuf>,
    ast: Rc<rhai::AST>,
    setups: Rc<RefCell<HashMap<String, SetupRef>>>,

    /// An op currently being built. `None` means no op is currently being built.
    cur_op: Rc<RefCell<Option<RhaiOp>>>,
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

    /// Begins building an op. This fails if an op is already being built or `input` or `output`
    /// are not arrays of `StateRef`.
    fn begin_op(
        &self,
        pos: Position,
        name: &str,
        inputs: rhai::Array,
        outputs: rhai::Array,
    ) -> RhaiResult<()> {
        let inputs = inputs
            .into_iter()
            .map(|i| match i.clone().try_cast::<StateRef>() {
                Some(state) => Ok(state),
                None => Err(RhaiSystemError::state_ref(i).with_pos(pos).into()),
            })
            .collect::<RhaiResult<Vec<_>>>()?;
        let outputs = outputs
            .into_iter()
            .map(|i| match i.clone().try_cast::<StateRef>() {
                Some(state) => Ok(state),
                None => Err(RhaiSystemError::state_ref(i).with_pos(pos).into()),
            })
            .collect::<RhaiResult<Vec<_>>>()?;

        let mut cur_op = self.cur_op.borrow_mut();
        match *cur_op {
            None => {
                *cur_op = Some(RhaiOp {
                    name: name.to_string(),
                    input_states: inputs,
                    output_states: outputs,
                    cmds: vec![],
                    config_vars: vec![],
                });
                Ok(())
            }
            Some(RhaiOp {
                name: ref old_name,
                input_states: _,
                output_states: _,
                cmds: _,
                config_vars: _,
            }) => Err(RhaiSystemError::began_op(old_name, name)
                .with_pos(pos)
                .into()),
        }
    }

    /// Adds a shell command to the `cur_op`.Returns and error if `begin_op` has not been called
    /// before this `end_op` and after any previous `end_op`
    fn add_shell(&self, pos: Position, cmd: String) -> RhaiResult<()> {
        let mut cur_op = self.cur_op.borrow_mut();
        match *cur_op {
            Some(ref mut op_sig) => {
                op_sig.cmds.push(cmd);
                Ok(())
            }
            None => Err(RhaiSystemError::no_op().with_pos(pos).into()),
        }
    }

    /// Adds a config var. Returns an error if `begin_op` has not been called
    /// before this `end_op` and after any previous `end_op`.
    fn add_config_var(
        &self,
        pos: Position,
        var: crate::run::ConfigVar,
    ) -> RhaiResult<()> {
        let mut cur_op = self.cur_op.borrow_mut();
        match *cur_op {
            Some(ref mut op_sig) => {
                op_sig.config_vars.push(var);
                Ok(())
            }
            None => Err(RhaiSystemError::no_op().with_pos(pos).into()),
        }
    }

    /// Collects an op currently being built and adds it to `bld`. Returns and error if `begin_op`
    /// has not been called before this `end_op` and after any previous `end_op`.
    fn end_op(
        &self,
        pos: Position,
        mut bld: RefMut<DriverBuilder>,
    ) -> RhaiResult<()> {
        let mut cur_op = self.cur_op.borrow_mut();
        match *cur_op {
            Some(RhaiOp {
                ref name,
                ref input_states,
                ref output_states,
                ref cmds,
                ref config_vars,
            }) => {
                // Create the emitter.
                let cmds = cmds.clone();
                let op_name = name.clone();
                let config_vars = config_vars.clone();
                let op_emitter = crate::run::RulesOp {
                    rule_name: op_name,
                    cmds,
                    config_vars,
                };

                // Add the op.
                bld.add_op(name, &[], input_states, output_states, op_emitter);

                // Now no op is being built.
                *cur_op = None;
                Ok(())
            }
            None => Err(RhaiSystemError::no_op().with_pos(pos).into()),
        }
    }
}

/// All nodes in the parsing state machine.
#[derive(Debug, Copy, Clone)]
enum ParseNode {
    DefopS,
    IdentS,
    OpenParenS,
    IdentP1,
    ColonP1,
    ExprP1,
    CommaP1,
    CloseParenP1,
    ArrowS,
    IdentP2,
    ColonP2,
    ExprP2,
    CommaP2,
    BlockS,
}

/// All of the state of the parser.
#[derive(Debug, Clone)]
struct ParseState {
    node: ParseNode,
    inputs: usize,
    outputs: usize,
}

impl ParseState {
    pub fn new() -> Self {
        Self {
            node: ParseNode::DefopS,
            inputs: 0,
            outputs: 0,
        }
    }

    pub fn node(&self, node: ParseNode) -> Self {
        Self {
            node,
            inputs: self.inputs,
            outputs: self.outputs,
        }
    }

    pub fn inputs(&self, inputs: usize) -> Self {
        Self {
            node: self.node,
            inputs,
            outputs: self.outputs,
        }
    }

    pub fn outputs(&self, outputs: usize) -> Self {
        Self {
            node: self.node,
            inputs: self.inputs,
            outputs,
        }
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
        this.reg_defop_syntax_nop();
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

    /// Registers a Rhai function which starts the parser listening for shell commands, how an op
    /// does its transformation.
    fn reg_start_op_stmts(&mut self, sctx: ScriptContext) {
        self.engine.register_fn(
            "start_op_stmts",
            move |ctx: rhai::NativeCallContext,
                  name: &str,
                  inputs: rhai::Array,
                  outputs: rhai::Array|
                  -> RhaiResult<_> {
                sctx.begin_op(ctx.position(), name, inputs, outputs)
            },
        );
    }

    /// Registers a Rhai function which adds shell commands to be used by an op.
    fn reg_shell(&mut self, sctx: ScriptContext) {
        self.engine.register_fn(
            "shell",
            move |ctx: rhai::NativeCallContext, cmd: &str| -> RhaiResult<_> {
                sctx.add_shell(ctx.position(), cmd.to_string())
            },
        );
    }

    /// Registers a Rhai function for getting values from the config file.
    fn reg_config(&mut self, sctx: ScriptContext) {
        self.engine.register_fn(
            "config",
            move |ctx: rhai::NativeCallContext, key: &str| -> RhaiResult<_> {
                sctx.add_config_var(
                    ctx.position(),
                    crate::run::ConfigVar::Required(key.to_string()),
                )?;
                Ok(format!("${{{}}}", key))
            },
        );
    }

    /// Registers a Rhai function for getting values from the config file or using a provided
    /// string if the key is not found.
    fn reg_config_or(&mut self, sctx: ScriptContext) {
        self.engine.register_fn(
            "config_or",
            move |ctx: rhai::NativeCallContext,
                  key: &str,
                  default: &str|
                  -> RhaiResult<_> {
                sctx.add_config_var(
                    ctx.position(),
                    crate::run::ConfigVar::Optional(
                        key.to_string(),
                        default.to_string(),
                    ),
                )?;
                Ok(format!("${{{}}}", key))
            },
        );
    }

    /// Registers a Rhai function which stops the parser listening to shell commands and adds the
    /// created op to `self.builder`.
    fn reg_end_op_stmts(&mut self, sctx: ScriptContext) {
        let bld = Rc::clone(&self.builder);
        self.engine.register_fn(
            "end_op_stmts",
            move |ctx: rhai::NativeCallContext| -> RhaiResult<_> {
                sctx.end_op(ctx.position(), bld.borrow_mut())
            },
        );
    }

    /// A parse function to add custom syntax for defining ops to rhai.
    fn parse_defop(
        symbols: &[ImmutableString],
        look_ahead: &str,
        state: &mut Dynamic,
    ) -> Result<Option<ImmutableString>, ParseError> {
        if symbols.len() == 1 {
            *state = Dynamic::from(ParseState::new());
        }
        let s = state.clone_cast::<ParseState>();
        match s.node {
            ParseNode::DefopS => {
                *state = Dynamic::from(s.node(ParseNode::IdentS));
                Ok(Some("$ident$".into()))
            }
            ParseNode::IdentS => {
                *state = Dynamic::from(s.node(ParseNode::OpenParenS));
                Ok(Some("(".into()))
            }
            ParseNode::OpenParenS => {
                *state = Dynamic::from(s.node(ParseNode::IdentP1));
                Ok(Some("$ident$".into()))
            }
            ParseNode::IdentP1 => {
                *state = Dynamic::from(
                    s.node(ParseNode::ColonP1).inputs(s.inputs + 1),
                );
                Ok(Some(":".into()))
            }
            ParseNode::ColonP1 => {
                *state = Dynamic::from(s.node(ParseNode::ExprP1));
                Ok(Some("$expr$".into()))
            }
            ParseNode::ExprP1 => {
                if look_ahead == "," {
                    *state = Dynamic::from(s.node(ParseNode::CommaP1));
                    Ok(Some(",".into()))
                } else {
                    *state = Dynamic::from(s.node(ParseNode::CloseParenP1));
                    Ok(Some(")".into()))
                }
            }
            ParseNode::CommaP1 => {
                *state = Dynamic::from(s.node(ParseNode::IdentP1));
                Ok(Some("$ident$".into()))
            }
            ParseNode::CloseParenP1 => {
                *state = Dynamic::from(s.node(ParseNode::ArrowS));
                Ok(Some(">>".into()))
            }
            ParseNode::ArrowS => {
                *state = Dynamic::from(s.node(ParseNode::IdentP2));
                Ok(Some("$ident$".into()))
            }
            ParseNode::IdentP2 => {
                *state = Dynamic::from(
                    s.node(ParseNode::ColonP2).outputs(s.outputs + 1),
                );
                Ok(Some(":".into()))
            }
            ParseNode::ColonP2 => {
                *state = Dynamic::from(s.node(ParseNode::ExprP2));
                Ok(Some("$expr$".into()))
            }
            ParseNode::ExprP2 => {
                if look_ahead == "," {
                    *state = Dynamic::from(s.node(ParseNode::CommaP2));
                    Ok(Some(",".into()))
                } else {
                    *state = Dynamic::from(s.node(ParseNode::BlockS));
                    Ok(Some("$block$".into()))
                }
            }
            ParseNode::CommaP2 => {
                *state = Dynamic::from(s.node(ParseNode::IdentP2));
                Ok(Some("$ident$".into()))
            }
            ParseNode::BlockS => Ok(None),
        }
    }

    /// Registers custom syntax for defining op without actually defining the op.
    fn reg_defop_syntax_nop(&mut self) {
        self.engine.register_custom_syntax_with_state_raw(
            "defop",
            Self::parse_defop,
            false,
            move |_context, _inputs, _state| Ok(().into()),
        );
    }

    /// Registers a custom syntax for creating ops using `start_op_stmts` and `end_op_stmts`.
    fn reg_defop_syntax(&mut self, sctx: ScriptContext) {
        let bld = Rc::clone(&self.builder);
        self.engine.register_custom_syntax_with_state_raw(
            "defop",
            Self::parse_defop,
            true,
            move |context, inputs, state| {
                let state = state.clone_cast::<ParseState>();
                // Collect name of op and input/output states.
                let op_name =
                    inputs.first().unwrap().get_string_value().unwrap();
                let input_names: Vec<_> = inputs
                    .iter()
                    .skip(1)
                    .step_by(2)
                    .take(state.inputs)
                    .map(|n| {
                        Dynamic::from(n.get_string_value().unwrap().to_string())
                    })
                    .collect();
                let input_states = inputs
                    .iter()
                    .skip(2)
                    .step_by(2)
                    .take(state.inputs)
                    .map(|s| s.eval_with_context(context))
                    .collect::<RhaiResult<Vec<_>>>()?;
                let output_names: Vec<_> = inputs
                    .iter()
                    .skip(1 + 2 * state.inputs)
                    .step_by(2)
                    .take(state.outputs)
                    .map(|n| {
                        Dynamic::from(n.get_string_value().unwrap().to_string())
                    })
                    .collect();
                let output_states: Vec<_> = inputs
                    .iter()
                    .skip(2 + 2 * state.inputs)
                    .step_by(2)
                    .take(state.outputs)
                    .map(|s| s.eval_with_context(context))
                    .collect::<RhaiResult<Vec<_>>>()?;
                let body = inputs.last().unwrap();

                let orig_scope_size = context.scope().len();

                for (i, name) in input_names.clone().into_iter().enumerate() {
                    context.scope_mut().push(
                        name.into_string().unwrap(),
                        format!("${}", crate::run::io_file_var_name(i, true)),
                    );
                }

                for (i, name) in output_names.clone().into_iter().enumerate() {
                    context.scope_mut().push(
                        name.into_string().unwrap(),
                        format!("${}", crate::run::io_file_var_name(i, false)),
                    );
                }

                // Position to note error.
                let op_pos = inputs.first().unwrap().position();

                // Begin listening for `shell` functions. Execute definition body and collect into
                // an op.
                sctx.begin_op(op_pos, op_name, input_states, output_states)?;
                let _ = body.eval_with_context(context)?;
                let res =
                    sctx.end_op(op_pos, bld.borrow_mut()).map(Dynamic::from);
                context.scope_mut().rewind(orig_scope_size);
                res
            },
        );
    }

    fn script_context(&self, path: PathBuf) -> ScriptContext {
        ScriptContext {
            builder: Rc::clone(&self.builder),
            path: Rc::new(path),
            ast: Rc::new(self.rhai_functions.clone()),
            setups: Rc::clone(&self.setups),
            cur_op: Rc::new(None.into()),
        }
    }

    fn run_file(&mut self, path: &Path) {
        let sctx = self.script_context(path.to_path_buf());
        self.reg_rule(sctx.clone());
        self.reg_op(sctx.clone());
        self.reg_start_op_stmts(sctx.clone());
        self.reg_shell(sctx.clone());
        self.reg_end_op_stmts(sctx.clone());
        self.reg_defop_syntax(sctx.clone());
        self.reg_config(sctx.clone());
        self.reg_config_or(sctx.clone());

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
