use super::commands::Command;
use super::context::DebuggingContext;
use super::io_utils::Input;
use crate::environment::InterpreterState;
use crate::errors::{InterpreterError, InterpreterResult};
use crate::interpreter::{ComponentInterpreter, Interpreter};
use calyx::ir::{self, RRC};

pub(super) const SPACING: &str = "    ";

/// The interactive Calyx debugger. The debugger itself is run with the
/// [main_loop] function while this struct holds auxilliary information used to
/// coordinate the debugging process.
pub struct Debugger<'a> {
    context: &'a ir::Context,
    main_component: &'a ir::Component,
    debugging_ctx: DebuggingContext,
}

impl<'a> Debugger<'a> {
    pub fn new(
        context: &'a ir::Context,
        main_component: &'a ir::Component,
    ) -> Self {
        Self {
            context,
            main_component,
            debugging_ctx: DebuggingContext::default(),
        }
    }

    pub fn main_loop(
        &mut self,
        env: InterpreterState,
        pass_through: bool, //flag to just evaluate the debugger version (non-interactive mode)
    ) -> InterpreterResult<InterpreterState> {
        let control: &ir::Control = &self.main_component.control.borrow();
        let mut component_interpreter = ComponentInterpreter::from_component(
            self.main_component,
            control,
            env,
        );

        if pass_through {
            component_interpreter.run();
            return Ok(component_interpreter.deconstruct());
        }

        let mut input_stream = Input::default();
        println!("== Calyx Interactive Debugger ==");
        loop {
            let comm = input_stream.next_command();
            let comm = match comm {
                Ok(c) => c,
                Err(e) => match &e {
                    InterpreterError::InvalidCommand(_)
                    | InterpreterError::UnknownCommand(_) => {
                        println!("{:?}", e);
                        continue;
                    }
                    _ => return Err(e),
                },
            };

            match comm {
                Command::Step => component_interpreter.step(),
                Command::Continue => {
                    let mut breakpoints = self.debugging_ctx.hit_breakpoints(
                        &component_interpreter.currently_executing_group(),
                    );

                    while breakpoints.is_empty()
                        && !component_interpreter.is_done()
                    {
                        component_interpreter.step();
                        breakpoints = self.debugging_ctx.hit_breakpoints(
                            &component_interpreter.currently_executing_group(),
                        );
                    }
                    if !component_interpreter.is_done() {
                        for breakpoint in breakpoints {
                            println!("Hit breakpoint: {}", breakpoint);
                        }
                    }
                }
                Command::Empty => {}
                Command::Display => {
                    let states = component_interpreter.get_env();
                    println!(
                        "{}",
                        if states.len() == 1 {
                            states[0].state_as_str()
                        } else {
                            "There are mutliple states".into()
                        }
                    )
                }
                Command::PrintCell(cell) => {
                    let env: Vec<_> = component_interpreter
                        .get_env()
                        .into_iter()
                        .map(|x| (x, x.get_cell(&cell)))
                        .collect();
                    if env.iter().any(|(_, x)| x.len() > 1) {
                        println!(
                            "{}Unable to print. '{}' is ambiguous",
                            SPACING, &cell
                        )
                    } else if env.iter().all(|(_, x)| x.is_empty()) {
                        println!(
                            "{}Unable to print. No cell named '{}'",
                            SPACING, &cell
                        )
                    } else {
                        for (state, cells) in env {
                            if let Some(cell_ref) = cells.first() {
                                print_cell(cell_ref, state)
                            }
                        }
                    }
                }
                Command::PrintCellOrPort(first, second) => {
                    // component & cell/port
                    if let Some(comp) =
                        self.context.components.iter().find(|x| x.name == first)
                    {
                        if let Some(cell) = comp.find_cell(&second) {
                            for env in component_interpreter.get_env() {
                                print_cell(&cell, env)
                            }
                        } else if let Some(port) =
                            comp.signature.borrow().find(&second)
                        {
                            for env in component_interpreter.get_env() {
                                println!(
                                    "{}{}.{} = {}",
                                    SPACING,
                                    &first,
                                    &second,
                                    env.get_from_port(&port)
                                )
                            }
                        } else {
                            println!("{}Unable to print. Component '{}' has no cell named '{}'", SPACING, &second, &first)
                        }
                    }
                    // cell & port
                    else {
                        let envs: Vec<_> = component_interpreter
                            .get_env()
                            .into_iter()
                            .map(|x| (x, x.get_cell(&first)))
                            .collect();

                        // multiple possible cells
                        if envs.iter().any(|(_, x)| x.len() > 1) {
                            println!(
                                "{}Unable to print. '{}' is ambiguous",
                                SPACING, &first
                            )
                        } else if envs.iter().all(|(_, x)| x.is_empty()) {
                            println!(
                                "{}Unable to print. There is no component/cell named '{}'",
                                SPACING,
                                &first,
                            )
                        } else if envs
                            .iter()
                            .flat_map(|(_, x)| x.iter())
                            .all(|x| x.borrow().find(&second).is_none())
                        {
                            println!(
                                "{}Unable to print. Component '{}' has no cell named '{}'",
                                SPACING,
                                &first, &second
                            )
                        } else {
                            for (state, cells) in envs {
                                if let Some(cell_ref) = cells.first() {
                                    if let Some(port) =
                                        cell_ref.borrow().find(&second)
                                    {
                                        print_port(&port, state)
                                    }
                                }
                            }
                        }
                    }
                }
                Command::PrintFullySpecified(comp, cell, port) => {
                    if let Some(comp_ref) =
                        self.context.components.iter().find(|x| x.name == comp)
                    {
                        if let Some(cell_rrc) = comp_ref.find_cell(&cell) {
                            let cell_ref = cell_rrc.borrow();
                            if let Some(port_ref) = cell_ref.find(&port) {
                                for state in component_interpreter.get_env() {
                                    print_port(&port_ref, state)
                                }
                            } else {
                                println!("{}Unable to print. Cell '{}' has no port named '{}'", SPACING, cell, port)
                            }
                        } else {
                            println!("{}Unable to print. Component '{}' has no cell named '{}'", SPACING, comp, cell)
                        }
                    } else {
                        println!("{}Unable to print. There is no component named '{}'", SPACING, comp)
                    }
                }
                Command::Help => {
                    print!("{}", Command::get_help_string())
                }
                Command::Break(target) => {
                    if self
                        .context
                        .components
                        .iter()
                        .any(|x| x.groups.find(&target).is_some())
                    {
                        self.debugging_ctx.add_breakpoint(target)
                    } else {
                        println!(
                            "{}There is no group named: {}",
                            SPACING, target
                        )
                    }
                }
                Command::Exit => return Err(InterpreterError::Interrupted),
                Command::InfoBreak => self.debugging_ctx.print_breakpoints(),
                Command::DelBreakpointByNum(target) => {
                    self.debugging_ctx.remove_breakpoint_by_number(target)
                }
                Command::DelBreakpointByName(target) => {
                    self.debugging_ctx.remove_breakpoint(target)
                }
                Command::EnableBreakpointByNum(target) => {
                    self.debugging_ctx.enable_breakpoint_by_num(target)
                }
                Command::EnableBreakpointByName(target) => {
                    self.debugging_ctx.enable_breakpoint(&target)
                }
                Command::DisableBreakpointByNum(target) => {
                    self.debugging_ctx.disable_breakpoint_by_num(target)
                }
                Command::DisableBreakpointByName(target) => {
                    self.debugging_ctx.disable_breakpoint(&target)
                }
            }

            if component_interpreter.is_done() {
                return Ok(component_interpreter.deconstruct());
            }
        }
    }
}

fn print_cell(target: &RRC<ir::Cell>, state: &InterpreterState) {
    let cell_ref = target.borrow();
    println!("{}{}", SPACING, cell_ref.name());
    for port in cell_ref.ports.iter() {
        println!(
            "{}  {} = {}",
            SPACING,
            port.borrow().name,
            state.get_from_port(port)
        )
    }
}

fn print_port(target: &RRC<ir::Port>, state: &InterpreterState) {
    let port_ref = target.borrow();
    let parent_name = port_ref.get_parent_name();

    println!(
        "{}{}.{} = {}",
        SPACING,
        parent_name,
        port_ref.name,
        state.get_from_port(&port_ref)
    )
}
