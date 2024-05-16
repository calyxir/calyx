use super::{
    commands::{Command, PrintCode, PrintMode},
    context::DebuggingContext,
    interactive_errors::DebuggerError,
    io_utils::Input,
    new_parser::parse_metadata,
    source::structures::NewSourceMap,
};
use crate::interpreter::{ComponentInterpreter, ConstCell, Interpreter};
use crate::structures::names::{CompGroupName, ComponentQualifiedInstanceName};
use crate::structures::state_views::StateView;
use crate::utils::AsRaw;
use crate::{configuration, debugger::source::SourceMap};
use crate::{
    environment::{InterpreterState, PrimitiveMap},
    MemoryMap,
};
use crate::{
    errors::{InterpreterError, InterpreterResult},
    structures::names::GroupQIN,
};
use crate::{interpreter_ir as iir, serialization::Serializable};
use std::collections::HashSet;

use calyx_frontend::Workspace;
use calyx_ir::{self as ir, Id, RRC};

use calyx_opt::pass_manager::PassManager;
use owo_colors::OwoColorize;
use std::{cell::Ref, collections::HashMap, rc::Rc};
use std::{fmt::Write, path::Path};
/// Constant amount of space used for debugger messages
pub(super) const SPACING: &str = "    ";

/// ProgramStatus returns the status of the program, helpful
/// status contains the set of running groups, done states if the program
/// is finished or not. If program is done then the debugger is exited
pub struct ProgramStatus {
    /// all groups currently running
    status: HashSet<Id>,
    /// states whether the program has finished
    done: bool,
}

impl ProgramStatus {
    /// Create a new program status on the fly
    pub fn generate(current_groups: HashSet<GroupQIN>, is_done: bool) -> Self {
        let mut set: HashSet<Id> = HashSet::new();
        for item in current_groups {
            set.insert(item.get_suffix());
        }

        ProgramStatus {
            status: set,
            done: is_done,
        }
    }

    /// get status
    pub fn get_status(&self) -> &HashSet<Id> {
        &self.status
    }

    /// see if program is done
    pub fn get_done(&self) -> bool {
        self.done
    }
}

/// The interactive Calyx debugger. The debugger itself is run with the
/// [Debugger::main_loop] function while this struct holds auxilliary
/// information used to coordinate the debugging process.
pub struct Debugger {
    _context: iir::ComponentCtx,
    main_component: Rc<iir::Component>,
    debugging_ctx: DebuggingContext,
    source_map: Option<SourceMap>,
    interpreter: ComponentInterpreter,
}

impl Debugger {
    /// construct a debugger instance from the target calyx file
    pub fn from_file(
        file: &Path,
        lib_path: &Path,
    ) -> InterpreterResult<(Self, NewSourceMap)> {
        // create a workspace using the file and lib_path, run the standard
        // passes (see main.rs). Construct the initial environment then use that
        // to create a new debugger instance with new

        let builder = configuration::ConfigBuilder::new();

        let config = builder
            .quiet(false)
            .allow_invalid_memory_access(false)
            .error_on_overflow(false)
            .allow_par_conflicts(false)
            .build();

        let ws = Workspace::construct(&Some(file.to_path_buf()), lib_path)?;
        let mut ctx = ir::from_ast::ast_to_ir(ws)?;
        let pm = PassManager::default_passes()?;

        // if !opts.skip_verification
        pm.execute_plan(&mut ctx, &["validate".to_string()], &[], false)?;

        let entry_point = ctx.entrypoint;

        let components: iir::ComponentCtx = Rc::new(
            ctx.components
                .into_iter()
                .map(|x| Rc::new(x.into()))
                .collect(),
        );

        let main_component = components
            .iter()
            .find(|&cm| cm.name == entry_point)
            .ok_or(InterpreterError::MissingMainComponent)?;

        let mut mems = MemoryMap::inflate_map(&None)?;

        let env = InterpreterState::init_top_level(
            &components,
            main_component,
            &mut mems,
            &config,
        )?;

        // Make NewSourceMap, if we can't then we explode
        let mapping = ctx
            .metadata
            .map(|metadata| parse_metadata(&metadata))
            .unwrap_or_else(|| Err(InterpreterError::MissingMetaData.into()))?;

        Ok((
            Debugger::new(&components, main_component, None, env).unwrap(),
            mapping,
        ))
    }

    pub fn new(
        context: &iir::ComponentCtx,
        main_component: &Rc<iir::Component>,
        source_map: Option<SourceMap>,
        env: InterpreterState,
    ) -> InterpreterResult<Self> {
        let qin = ComponentQualifiedInstanceName::new_single(
            main_component,
            main_component.name,
        );
        let mut component_interpreter =
            ComponentInterpreter::from_component(main_component, env, qin);
        component_interpreter.set_go_high();

        component_interpreter.converge()?;

        Ok(Self {
            _context: Rc::clone(context),
            main_component: Rc::clone(main_component),
            debugging_ctx: DebuggingContext::new(context, &main_component.name),
            source_map,
            interpreter: component_interpreter,
        })
    }

    // Go to next step
    pub fn step(&mut self, n: u64) -> InterpreterResult<ProgramStatus> {
        for _ in 0..n {
            self.interpreter.step()?;
        }
        self.interpreter.converge()?;

        // Create new HashSet with Ids
        Ok(ProgramStatus::generate(
            self.interpreter.currently_executing_group(),
            self.interpreter.is_done(),
        ))
    }

    pub fn cont(&mut self) -> InterpreterResult<()> {
        self.debugging_ctx
            .set_current_time(self.interpreter.currently_executing_group());

        let mut ctx = std::mem::replace(
            &mut self.debugging_ctx,
            DebuggingContext::new(&self._context, &self.main_component.name),
        );

        let mut breakpoints: Vec<CompGroupName> = vec![];

        while breakpoints.is_empty() && !self.interpreter.is_done() {
            self.interpreter.step()?;
            let current_exec = self.interpreter.currently_executing_group();

            ctx.advance_time(current_exec);

            for watch in ctx.process_watchpoints() {
                for target in watch.target() {
                    if let Ok(msg) = Self::do_print(
                        self.main_component.name,
                        target,
                        watch.print_code(),
                        self.interpreter.get_env(),
                        watch.print_mode(),
                    ) {
                        println!("{}", msg.on_black().yellow().bold());
                    }
                }
            }

            breakpoints = ctx.hit_breakpoints().into_iter().cloned().collect();
        }

        self.debugging_ctx = ctx;

        if !self.interpreter.is_done() {
            for breakpoint in breakpoints {
                println!(
                    "Hit breakpoint: {}",
                    breakpoint.bright_purple().underline()
                );
            }
            self.interpreter.converge()?;
        };
        Ok(())
    }

    // so on and so forth

    pub fn main_loop(mut self) -> InterpreterResult<InterpreterState> {
        let mut input_stream = Input::new()?;

        println!("== Calyx Interactive Debugger ==");

        while !self.interpreter.is_done() {
            let comm = input_stream.next_command();
            let comm = match comm {
                Ok(c) => c,
                Err(e) => match *e {
                    InterpreterError::InvalidCommand(_)
                    | InterpreterError::UnknownCommand(_)
                    | InterpreterError::ParseError(_) => {
                        println!("Error: {}", e.red().bold());
                        continue;
                    }
                    _ => return Err(e),
                },
            };

            match comm {
                Command::Step(n) => {
                    for _ in 0..n {
                        self.interpreter.step()?;
                    }
                    self.interpreter.converge()?;
                }
                Command::Continue => {
                    self.debugging_ctx.set_current_time(
                        self.interpreter.currently_executing_group(),
                    );

                    let mut ctx = std::mem::replace(
                        &mut self.debugging_ctx,
                        DebuggingContext::new(
                            &self._context,
                            &self.main_component.name,
                        ),
                    );

                    let mut breakpoints: Vec<CompGroupName> = vec![];

                    while breakpoints.is_empty() && !self.interpreter.is_done()
                    {
                        self.interpreter.step()?;
                        let current_exec =
                            self.interpreter.currently_executing_group();

                        ctx.advance_time(current_exec);

                        for watch in ctx.process_watchpoints() {
                            for target in watch.target() {
                                if let Ok(msg) = Self::do_print(
                                    self.main_component.name,
                                    target,
                                    watch.print_code(),
                                    self.interpreter.get_env(),
                                    watch.print_mode(),
                                ) {
                                    println!(
                                        "{}",
                                        msg.on_black().yellow().bold()
                                    );
                                }
                            }
                        }

                        breakpoints = ctx
                            .hit_breakpoints()
                            .into_iter()
                            .cloned()
                            .collect();
                    }

                    self.debugging_ctx = ctx;

                    if !self.interpreter.is_done() {
                        for breakpoint in breakpoints {
                            println!(
                                "Hit breakpoint: {}",
                                breakpoint.bright_purple().underline()
                            );
                        }
                        self.interpreter.converge()?;
                    }
                }
                Command::Empty => {}
                Command::Display => {
                    let state = self.interpreter.get_env();
                    println!("{}", state.state_as_str().green().bold());
                }
                Command::Print(print_lists, code, print_mode) => {
                    for target in print_lists {
                        match Self::do_print(
                            self.main_component.name,
                            &target,
                            &code,
                            self.interpreter.get_env(),
                            &print_mode,
                        ) {
                            Ok(msg) => println!("{}", msg.magenta()),
                            Err(e) => println!("{}", e.bright_red().bold()),
                        }
                    }
                }
                Command::Help => {
                    print!("{}", Command::get_help_string().cyan())
                }
                Command::Break(targets) => {
                    if targets.is_empty() {
                        println!("Error: command requires a target");
                        continue;
                    }

                    for target in targets {
                        let currently_executing =
                            self.interpreter.currently_executing_group();
                        let target =
                            self.debugging_ctx.concretize_group_name(target);

                        if self
                            .debugging_ctx
                            .is_group_running(currently_executing, &target)
                        {
                            println!("Warning: the group {} is already running. This breakpoint will not trigger until the next time the group runs.", &target.yellow().italic())
                        }

                        self.debugging_ctx.add_breakpoint(target);
                    }
                }
                Command::Exit => return Err(InterpreterError::Exit.into()),
                Command::InfoBreak => self.debugging_ctx.print_breakpoints(),
                Command::Delete(targets) => {
                    if targets.is_empty() {
                        println!("Error: command requires a target");
                        continue;
                    }
                    for t in targets {
                        self.debugging_ctx.remove_breakpoint(t)
                    }
                }
                Command::DeleteWatch(targets) => {
                    if targets.is_empty() {
                        println!("Error: command requires a target");
                        continue;
                    }
                    for target in targets {
                        self.debugging_ctx.remove_watchpoint(target)
                    }
                }
                Command::Disable(targets) => {
                    if targets.is_empty() {
                        println!("Error: command requires a target");
                        continue;
                    }
                    for t in targets {
                        self.debugging_ctx.disable_breakpoint(t)
                    }
                }
                Command::Enable(targets) => {
                    if targets.is_empty() {
                        println!("Error: command requires a target");
                        continue;
                    }
                    for t in targets {
                        self.debugging_ctx.enable_breakpoint(t)
                    }
                }
                Command::StepOver(target) => {
                    let mut current =
                        self.interpreter.currently_executing_group();
                    let target =
                        self.debugging_ctx.concretize_group_name(target);

                    if !self.debugging_ctx.is_group_running(current, &target) {
                        println!("Group is not running")
                    } else {
                        self.interpreter.step()?;
                        current = self.interpreter.currently_executing_group();
                        while self
                            .debugging_ctx
                            .is_group_running(current, &target)
                        {
                            self.interpreter.step()?;
                            current =
                                self.interpreter.currently_executing_group();
                        }
                    }
                }
                Command::Watch(
                    group,
                    watch_pos,
                    print_target,
                    print_code,
                    print_mode,
                ) => {
                    let mut error_occurred = false;

                    for target in print_target.iter() {
                        if let Err(e) = Self::do_print(
                            self.main_component.name,
                            target,
                            &print_code,
                            self.interpreter.get_env(),
                            &print_mode,
                        ) {
                            error_occurred = true;
                            println!("{}", e.red().bold());
                        }
                    }

                    if error_occurred {
                        continue;
                    }

                    self.debugging_ctx.add_watchpoint(
                        group,
                        watch_pos,
                        (print_target, print_code, print_mode),
                    )
                }
                Command::InfoWatch => self.debugging_ctx.print_watchpoints(),
                Command::PrintPC(override_flag) => {
                    if self.source_map.is_some() && !override_flag {
                        let map = self.source_map.as_ref().unwrap();
                        let mut printed = false;
                        for x in self
                            .interpreter
                            .get_active_tree()
                            .remove(0)
                            .flat_set()
                            .into_iter()
                        {
                            if let Some(output) = map.lookup(x) {
                                printed = true;
                                println!("{}", output);
                            }
                        }

                        if !printed {
                            println!("Falling back to Calyx");
                            print!(
                                "{}",
                                self.interpreter
                                    .get_active_tree()
                                    .remove(0)
                                    .format_tree::<true>(0)
                            );
                        }
                    } else {
                        print!(
                            "{}",
                            self.interpreter
                                .get_active_tree()
                                .remove(0)
                                .format_tree::<true>(0)
                        );
                    }
                }

                Command::Explain => {
                    print!("{}", Command::get_explain_string().blue())
                }
            }
        }

        let final_env = self.interpreter.deconstruct()?;

        println!("Main component has finished executing. Debugger is now in inspection mode.");

        loop {
            let comm = input_stream.next_command();
            let comm = match comm {
                Ok(c) => c,
                Err(e) => match *e {
                    InterpreterError::InvalidCommand(_)
                    | InterpreterError::UnknownCommand(_)
                    | InterpreterError::ParseError(_) => {
                        println!("Error: {}", e.red().bold());
                        continue;
                    }
                    _ => return Err(e),
                },
            };

            match comm {
                Command::Empty => {}
                Command::Display => {
                    let state = final_env.as_state_view();
                    println!("{}", state.state_as_str().purple());
                }
                Command::Print(print_lists, code, print_mode) => {
                    for target in print_lists {
                        match Self::do_print(
                            self.main_component.name,
                            &target,
                            &code,
                            final_env.as_state_view(),
                            &print_mode,
                        ) {
                            Ok(msg) => println!("{}", msg.green()),
                            Err(e) => {
                                println!("{}", e.red().underline().bold())
                            }
                        }
                    }
                }

                Command::Help => {
                    print!("{}", Command::get_help_string().blue())
                }
                Command::Exit => return Err(InterpreterError::Exit.into()),
                Command::Explain => {
                    print!("{}", Command::get_explain_string().blue().bold())
                }
                _ => {
                    println!(
                        "This command is unavailable after program termination"
                    )
                }
            }
        }
    }

    fn do_print(
        main_comp_name: Id,
        print_list: &[Id],
        code: &Option<PrintCode>,
        root: StateView,
        print_mode: &PrintMode,
    ) -> Result<String, DebuggerError> {
        let orig_string = print_list
            .iter()
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>()
            .join(".");

        let mut iter = print_list.iter();

        let length = if main_comp_name == print_list[0] {
            iter.next();
            print_list.len() - 1
        } else {
            print_list.len()
        };

        let mut current_target = CurrentTarget::Env(&root);

        for (idx, target) in iter.enumerate() {
            let current_ref = current_target.borrow();
            let current_env = current_ref.get_env().unwrap();

            // lowest level
            if idx == length - 1 {
                // first look for cell
                let cell = current_env.get_cell(*target);
                if let Some(cell) = cell {
                    return Ok(print_cell(
                        &cell,
                        &current_env,
                        code,
                        print_mode,
                    ));
                } else if idx != 0 {
                    let prior = &print_list[idx - 1];

                    if let Some(parent) = current_env.get_cell(*prior) {
                        let parent_ref = parent.borrow();
                        let pt = parent_ref
                            .ports()
                            .iter()
                            .find(|x| x.borrow().name == target);
                        if let Some(port) = pt {
                            return Ok(print_port(
                                port,
                                &current_env,
                                None,
                                code,
                            ));
                        } else {
                            return Err(DebuggerError::CannotFind(orig_string));
                            // cannot find
                        }
                    } else if let Some(port) =
                        current_env.get_comp().signature.borrow().find(target)
                    {
                        return Ok(print_port(
                            &port,
                            &current_env,
                            Some(print_list[idx - 1]),
                            code,
                        ));
                    } else {
                        // cannot find
                        return Err(DebuggerError::CannotFind(orig_string));
                    }
                } else {
                    return Err(DebuggerError::CannotFind(orig_string));
                }
            }
            // still walking
            else {
                let map = Rc::clone(current_env.get_cell_map());
                let cell = current_env.get_cell(*target);
                if let Some(rrc_cell) = cell {
                    // need to release these references to replace current
                    // target
                    if map.borrow()[&rrc_cell.as_raw()].get_state().is_some() {
                        drop(current_env);
                        drop(current_ref);

                        current_target = CurrentTarget::Target {
                            name: rrc_cell.as_raw(),
                            map,
                        }
                    }
                    // otherwise leave the same
                } else {
                    // cannot find
                    return Err(DebuggerError::CannotFind(orig_string));
                }
            }
        }

        unreachable!()
    }
}

fn print_cell(
    target: &RRC<ir::Cell>,
    state: &StateView,
    code: &Option<PrintCode>,
    mode: &PrintMode,
) -> String {
    let cell_ref = target.borrow();

    match mode {
        PrintMode::State => {
            let actual_code =
                code.as_ref().copied().unwrap_or(PrintCode::Binary);
            let cell_state = state.get_cell_state(&cell_ref, &actual_code);
            if matches!(&cell_state, &Serializable::Empty) {
                print_cell(target, state, code, &PrintMode::Port)
            } else {
                format!(
                    "{}{} = {}",
                    SPACING,
                    cell_ref.name().green().bold(),
                    cell_state.blue().bold()
                )
            }
        }

        PrintMode::Port => {
            let mut output: String = String::new();
            writeln!(output, "{}{}", SPACING, cell_ref.name().red())
                .expect("Something went wrong trying to print the port");
            for port in cell_ref.ports.iter() {
                let v = state.lookup(port.as_raw());
                writeln!(
                    output,
                    "{}  {} = {}",
                    SPACING,
                    port.borrow().name.red(),
                    if let Some(code) = code {
                        match code {
                            PrintCode::Unsigned => {
                                format!("{}", v.as_unsigned())
                            }
                            PrintCode::Signed => {
                                format!("{}", v.as_signed().green())
                            }
                            PrintCode::UFixed(num) => {
                                format!("{}", v.as_ufp(*num).blue())
                            }
                            PrintCode::SFixed(num) => {
                                format!("{}", v.as_sfp(*num).purple())
                            }
                            PrintCode::Binary => format!("{}", v.cyan()),
                        }
                    } else {
                        format!("{}", &v.magenta())
                    }
                )
                .expect("Something went wrong trying to print the port");
            }
            output
        }
    }
}

fn print_port(
    target: &RRC<ir::Port>,
    state: &StateView,
    prior_name: Option<ir::Id>,
    code: &Option<PrintCode>,
) -> String {
    let port_ref = target.borrow();
    let parent_name = if let Some(prior) = prior_name {
        prior
    } else {
        port_ref.get_parent_name()
    };

    let v = state.lookup(port_ref.as_raw());
    let code = code.as_ref().copied().unwrap_or(PrintCode::Binary);

    format!(
        "{}{}.{} = {}",
        SPACING,
        parent_name.red(),
        port_ref.name.green(),
        match code {
            PrintCode::Unsigned => format!("{}", v.as_unsigned()),
            PrintCode::Signed => format!("{}", v.as_signed()),
            PrintCode::UFixed(num) => format!("{}", v.as_ufp(num)),
            PrintCode::SFixed(num) => format!("{}", v.as_sfp(num)),
            PrintCode::Binary => format!("{}", v),
        }
    )
}

enum CurrentTarget<'a> {
    Env(&'a StateView<'a>),
    Target { name: ConstCell, map: PrimitiveMap },
}

impl<'a> CurrentTarget<'a> {
    pub fn borrow(&self) -> TargetRef<'_, '_> {
        match self {
            CurrentTarget::Env(e) => TargetRef::Env(e),
            CurrentTarget::Target { name, map } => {
                TargetRef::Target(*name, map.borrow())
            }
        }
    }
}

enum TargetRef<'a, 'c> {
    Env(&'c StateView<'a>),
    Target(
        ConstCell,
        Ref<'c, HashMap<ConstCell, Box<dyn crate::primitives::Primitive>>>,
    ),
}

impl<'a, 'c> TargetRef<'a, 'c> {
    pub fn get_env(&self) -> Option<StateView<'_>> {
        match self {
            TargetRef::Env(e) => Some((*e).clone()),
            TargetRef::Target(target, map) => map[target].get_state(),
        }
    }
}
