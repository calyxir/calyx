use std::cell::Ref;
use std::collections::HashMap;
use std::rc::Rc;

use super::commands::{Command, PrintCode};
use super::context::DebuggingContext;
use super::io_utils::Input;
use crate::environment::{InterpreterState, PrimitiveMap, StateView};
use crate::errors::{InterpreterError, InterpreterResult};
use crate::interpreter::{ComponentInterpreter, Interpreter};
use crate::interpreter_ir as iir;
use crate::primitives::Serializeable;
use crate::structures::names::ComponentQIN;
use crate::utils::AsRaw;
use calyx::ir::{self, Id, RRC};
pub(super) const SPACING: &str = "    ";
use super::interactive_errors::DebuggerError;
use crate::interpreter::ConstCell;
use std::fmt::Write;

/// The interactive Calyx debugger. The debugger itself is run with the
/// [main_loop] function while this struct holds auxilliary information used to
/// coordinate the debugging process.
pub struct Debugger {
    _context: iir::ComponentCtx,
    main_component: Rc<iir::Component>,
    debugging_ctx: DebuggingContext,
}

impl Debugger {
    pub fn new(
        context: &iir::ComponentCtx,
        main_component: &Rc<iir::Component>,
    ) -> Self {
        Self {
            _context: Rc::clone(context),
            main_component: Rc::clone(main_component),
            debugging_ctx: DebuggingContext::new(context, &main_component.name),
        }
    }

    pub fn main_loop(
        &mut self,
        env: InterpreterState,
        pass_through: bool, //flag to just evaluate the debugger version (non-interactive mode)
    ) -> InterpreterResult<InterpreterState> {
        let mut printed = false;
        let qin = ComponentQIN::new_single(
            &self.main_component,
            &self.main_component.name,
        );
        let mut component_interpreter = ComponentInterpreter::from_component(
            &self.main_component,
            env,
            qin,
        );
        component_interpreter.set_go_high();

        if pass_through {
            component_interpreter.run()?;
            return component_interpreter.deconstruct();
        }

        let mut input_stream = Input::default();
        println!("== Calyx Interactive Debugger ==");
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
                Command::Step => {
                    component_interpreter.step()?;
                }
                Command::Continue => {
                    let mut breakpoints = self.debugging_ctx.hit_breakpoints(
                        component_interpreter.currently_executing_group(),
                    );

                    while breakpoints.is_empty()
                        && !component_interpreter.is_done()
                    {
                        component_interpreter.step()?;
                        let current_exec =
                            component_interpreter.currently_executing_group();

                        let mut ctx = std::mem::replace(
                            &mut self.debugging_ctx,
                            DebuggingContext::new(
                                &self._context,
                                &self.main_component.name,
                            ),
                        );

                        for watch in
                            ctx.process_watchpoints(current_exec.clone())
                        {
                            if let Ok(msg) = self.do_print(
                                &watch.0,
                                &watch.1,
                                &component_interpreter,
                                &watch.2,
                            ) {
                                println!("{}", msg);
                            }
                        }

                        self.debugging_ctx = ctx;

                        breakpoints =
                            self.debugging_ctx.hit_breakpoints(current_exec);
                    }
                    if !component_interpreter.is_done() {
                        for breakpoint in breakpoints {
                            println!("Hit breakpoint: {}", breakpoint);
                        }
                    }
                }
                Command::Empty => {}
                Command::Display => {
                    let state = component_interpreter.get_env();
                    println!("{}", state.state_as_str());
                }
                Command::Print(print_lists, code) => match self.do_print(
                    &print_lists,
                    &code,
                    &component_interpreter,
                    &PrintMode::Port,
                ) {
                    Ok(msg) => println!("{}", msg),
                    Err(e) => println!("{}", e),
                },
                Command::PrintState(print_lists, code) => match self.do_print(
                    &print_lists,
                    &code,
                    &component_interpreter,
                    &PrintMode::State,
                ) {
                    Ok(msg) => println!("{}", msg),
                    Err(e) => println!("{}", e),
                },
                Command::Help => {
                    print!("{}", Command::get_help_string())
                }
                Command::Break(targets) => {
                    if targets.is_empty() {
                        println!("Error: command requires a target");
                        continue;
                    }

                    for target in targets {
                        self.debugging_ctx.add_breakpoint(target)
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
                        self.debugging_ctx.remove_breakpoint(&t)
                    }
                }

                Command::Disable(targets) => {
                    if targets.is_empty() {
                        println!("Error: command requires a target");
                        continue;
                    }
                    for t in targets {
                        self.debugging_ctx.disable_breakpoint(&t)
                    }
                }
                Command::Enable(targets) => {
                    if targets.is_empty() {
                        println!("Error: command requires a target");
                        continue;
                    }
                    for t in targets {
                        self.debugging_ctx.enable_breakpoint(&t)
                    }
                }
                Command::StepOver(target) => {
                    let mut current =
                        component_interpreter.currently_executing_group();

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
                Command::Watch(group, print_target, print_code, print_mode) => {
                    if let Err(e) = self.do_print(
                        &print_target,
                        &print_code,
                        &component_interpreter,
                        &print_mode,
                    ) {
                        println!("{}", e); // print is bad
                    } else {
                        self.debugging_ctx.add_watchpoint(
                            group,
                            (print_target, print_code, print_mode),
                        )
                    }
                }
            }

            if component_interpreter.is_done() && !printed {
                println!("Main component has finished executing");
                printed = true;
            }
        }
    }

    fn do_print(
        &mut self,
        print_lists: &Option<Vec<Vec<Id>>>,
        code: &Option<PrintCode>,
        component_interpreter: &ComponentInterpreter,
        print_mode: &PrintMode,
    ) -> Result<String, DebuggerError> {
        if print_lists.is_none() {
            return Err(DebuggerError::RequiresTarget);
        }

        for print_list in print_lists.as_ref().unwrap() {
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

            let mut current_target = CurrentTarget::Env(component_interpreter);

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
                                return Err(DebuggerError::CannotFind(
                                    orig_string,
                                ));
                                // cannot find
                            }
                        } else if let Some(port) = current_env
                            .get_comp()
                            .signature
                            .borrow()
                            .find(target)
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
                        if map.borrow()[&rrc_cell.as_raw()]
                            .get_state()
                            .is_some()
                        {
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
        }
        unreachable!()
    }
}

pub enum PrintMode {
    State,
    Port,
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
            let code = code.as_ref().copied().unwrap_or(PrintCode::Binary);
            let cell_state = state.get_cell_state(&cell_ref, &code);
            if matches!(&cell_state, &Serializeable::Empty) {
                format!(
                    "{} cell {} has no internal state",
                    SPACING,
                    cell_ref.name()
                )
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
    Env(&'a dyn crate::primitives::Primitive),
    Target { name: ConstCell, map: PrimitiveMap },
}

impl<'a> CurrentTarget<'a> {
    pub fn borrow(&self) -> TargetRef<'_, '_> {
        match self {
            CurrentTarget::Env(e) => TargetRef::Env(*e),
            CurrentTarget::Target { name, map } => {
                TargetRef::Target(*name, map.borrow())
            }
        }
    }
}

enum TargetRef<'a, 'c> {
    Env(&'a dyn crate::primitives::Primitive),
    Target(
        ConstCell,
        Ref<'c, HashMap<ConstCell, Box<dyn crate::primitives::Primitive>>>,
    ),
}

impl<'a, 'c> TargetRef<'a, 'c> {
    pub fn get_env(&self) -> Option<StateView<'_>> {
        match self {
            TargetRef::Env(e) => e.get_state(),
            TargetRef::Target(target, map) => map[target].get_state(),
        }
    }
}
