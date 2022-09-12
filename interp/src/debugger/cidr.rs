use std::{cell::Ref, collections::HashMap, rc::Rc};

use super::{
    commands::{Command, PrintCode, PrintMode},
    context::DebuggingContext,
    interactive_errors::DebuggerError,
    io_utils::Input,
};
use crate::debugger::source::SourceMap;
use crate::environment::{InterpreterState, PrimitiveMap};
use crate::errors::{InterpreterError, InterpreterResult};
use crate::interpreter::{ComponentInterpreter, ConstCell, Interpreter};
use crate::structures::names::{CompGroupName, ComponentQualifiedInstanceName};
use crate::structures::state_views::StateView;
use crate::utils::AsRaw;
use crate::{interpreter_ir as iir, primitives::Serializable};
use calyx::ir::{self, Id, RRC};
use std::fmt::Write;

/// Constant amount of space used for debugger messages
pub(super) const SPACING: &str = "    ";

/// The interactive Calyx debugger. The debugger itself is run with the
/// [Debugger::main_loop] function while this struct holds auxilliary
/// information used to coordinate the debugging process.
pub struct Debugger {
    _context: iir::ComponentCtx,
    main_component: Rc<iir::Component>,
    debugging_ctx: DebuggingContext,
    source_map: Option<SourceMap>,
}

impl Debugger {
    pub fn new(
        context: &iir::ComponentCtx,
        main_component: &Rc<iir::Component>,
        source_map: Option<SourceMap>,
    ) -> Self {
        Self {
            _context: Rc::clone(context),
            main_component: Rc::clone(main_component),
            debugging_ctx: DebuggingContext::new(context, &main_component.name),
            source_map,
        }
    }

    pub fn main_loop(
        &mut self,
        env: InterpreterState,
    ) -> InterpreterResult<InterpreterState> {
        let qin = ComponentQualifiedInstanceName::new_single(
            &self.main_component,
            &self.main_component.name,
        );
        let mut component_interpreter = ComponentInterpreter::from_component(
            &self.main_component,
            env,
            qin,
        );
        component_interpreter.set_go_high();

        component_interpreter.converge()?;

        let mut input_stream = Input::new()?;
        println!("== Calyx Interactive Debugger ==");
        while !component_interpreter.is_done() {
            let comm = input_stream.next_command();
            let comm = match comm {
                Ok(c) => c,
                Err(e) => match &e {
                    InterpreterError::InvalidCommand(_)
                    | InterpreterError::UnknownCommand(_)
                    | InterpreterError::ParseError(_) => {
                        println!("Error: {}", e);
                        continue;
                    }
                    _ => return Err(e),
                },
            };

            match comm {
                Command::Step(n) => {
                    for _ in 0..n {
                        component_interpreter.step()?;
                    }
                    component_interpreter.converge()?;
                }
                Command::Continue => {
                    self.debugging_ctx.set_current_time(
                        component_interpreter.currently_executing_group(),
                    );

                    let mut ctx = std::mem::replace(
                        &mut self.debugging_ctx,
                        DebuggingContext::new(
                            &self._context,
                            &self.main_component.name,
                        ),
                    );

                    let mut breakpoints: Vec<CompGroupName> = vec![];

                    while breakpoints.is_empty()
                        && !component_interpreter.is_done()
                    {
                        component_interpreter.step()?;
                        let current_exec =
                            component_interpreter.currently_executing_group();

                        ctx.advance_time(current_exec);

                        for watch in ctx.process_watchpoints() {
                            for target in watch.target() {
                                if let Ok(msg) = self.do_print(
                                    target,
                                    watch.print_code(),
                                    component_interpreter.get_env(),
                                    watch.print_mode(),
                                ) {
                                    println!("{}", msg);
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

                    if !component_interpreter.is_done() {
                        for breakpoint in breakpoints {
                            println!("Hit breakpoint: {}", breakpoint);
                        }
                        component_interpreter.converge()?;
                    }
                }
                Command::Empty => {}
                Command::Display => {
                    let state = component_interpreter.get_env();
                    println!("{}", state.state_as_str());
                }
                Command::Print(print_lists, code, print_mode) => {
                    for target in print_lists {
                        match self.do_print(
                            &target,
                            &code,
                            component_interpreter.get_env(),
                            &print_mode,
                        ) {
                            Ok(msg) => println!("{}", msg),
                            Err(e) => println!("{}", e),
                        }
                    }
                }
                Command::Help => {
                    print!("{}", Command::get_help_string())
                }
                Command::Break(targets) => {
                    if targets.is_empty() {
                        println!("Error: command requires a target");
                        continue;
                    }

                    for target in targets {
                        let currently_executing =
                            component_interpreter.currently_executing_group();
                        let target =
                            self.debugging_ctx.concretize_group_name(target);

                        if self
                            .debugging_ctx
                            .is_group_running(currently_executing, &target)
                        {
                            println!("Warning: the group {} is already running. This breakpoint will not trigger until the next time the group runs.", &target)
                        }

                        self.debugging_ctx.add_breakpoint(target);
                    }
                }
                Command::Exit => return Err(InterpreterError::Exit),
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
                        component_interpreter.currently_executing_group();
                    let target =
                        self.debugging_ctx.concretize_group_name(target);

                    if !self.debugging_ctx.is_group_running(current, &target) {
                        println!("Group is not running")
                    } else {
                        component_interpreter.step()?;
                        current =
                            component_interpreter.currently_executing_group();
                        while self
                            .debugging_ctx
                            .is_group_running(current, &target)
                        {
                            component_interpreter.step()?;
                            current = component_interpreter
                                .currently_executing_group();
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
                        if let Err(e) = self.do_print(
                            target,
                            &print_code,
                            component_interpreter.get_env(),
                            &print_mode,
                        ) {
                            error_occurred = true;
                            println!("{}", e);
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
                Command::PrintPC => {
                    if let Some(map) = &self.source_map {
                        let mut printed = false;
                        for x in component_interpreter
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
                                component_interpreter
                                    .get_active_tree()
                                    .remove(0)
                                    .format_tree::<true>(0)
                            )
                        }
                    } else {
                        print!(
                            "{}",
                            component_interpreter
                                .get_active_tree()
                                .remove(0)
                                .format_tree::<true>(0)
                        )
                    }
                }
                Command::Explain => print!("{}", Command::get_explain_string()),
            }
        }

        let final_env = component_interpreter.deconstruct()?;

        println!("Main component has finished executing. Debugger is now in inspection mode.");

        loop {
            let comm = input_stream.next_command();
            let comm = match comm {
                Ok(c) => c,
                Err(e) => match &e {
                    InterpreterError::InvalidCommand(_)
                    | InterpreterError::UnknownCommand(_)
                    | InterpreterError::ParseError(_) => {
                        println!("Error: {}", e);
                        continue;
                    }
                    _ => return Err(e),
                },
            };

            match comm {
                Command::Empty => {}
                Command::Display => {
                    let state = final_env.as_state_view();
                    println!("{}", state.state_as_str());
                }
                Command::Print(print_lists, code, print_mode) => {
                    for target in print_lists {
                        match self.do_print(
                            &target,
                            &code,
                            final_env.as_state_view(),
                            &print_mode,
                        ) {
                            Ok(msg) => println!("{}", msg),
                            Err(e) => println!("{}", e),
                        }
                    }
                }

                Command::Help => {
                    print!("{}", Command::get_help_string())
                }
                Command::Exit => return Err(InterpreterError::Exit),
                Command::Explain => print!("{}", Command::get_explain_string()),
                _ => {
                    println!(
                        "This command is unavailable after program termination"
                    )
                }
            }
        }
    }

    fn do_print(
        &mut self,
        print_list: &[Id],
        code: &Option<PrintCode>,
        root: StateView,
        print_mode: &PrintMode,
    ) -> Result<String, DebuggerError> {
        let orig_string = print_list
            .iter()
            .map(|s| s.id.clone())
            .collect::<Vec<_>>()
            .join(".");

        let mut iter = print_list.iter();

        let length = if self.main_component.name == print_list[0] {
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
                let cell = current_env.get_cell(target);
                if let Some(cell) = cell {
                    return Ok(print_cell(
                        &cell,
                        &current_env,
                        code,
                        print_mode,
                    ));
                } else if idx != 0 {
                    let prior = &print_list[idx - 1];

                    if let Some(parent) = current_env.get_cell(&prior) {
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
                            Some(print_list[idx - 1].clone()),
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
                let cell = current_env.get_cell(target);
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
                format!("{}{} = {}", SPACING, cell_ref.name(), cell_state)
            }
        }
        PrintMode::Port => {
            let mut output: String = String::new();
            writeln!(output, "{}{}", SPACING, cell_ref.name())
                .expect("Something went wrong trying to print the port");
            for port in cell_ref.ports.iter() {
                let v = state.lookup(port.as_raw());
                writeln!(
                    output,
                    "{}  {} = {}",
                    SPACING,
                    port.borrow().name,
                    if let Some(code) = code {
                        match code {
                            PrintCode::Unsigned => {
                                format!("{}", v.as_unsigned())
                            }
                            PrintCode::Signed => format!("{}", v.as_signed()),
                            PrintCode::UFixed(num) => {
                                format!("{}", v.as_ufp(*num))
                            }
                            PrintCode::SFixed(num) => {
                                format!("{}", v.as_sfp(*num))
                            }
                            PrintCode::Binary => format!("{}", v),
                        }
                    } else {
                        format!("{}", &v)
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
        parent_name,
        port_ref.name,
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
