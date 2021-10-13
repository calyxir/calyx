use std::cell::Ref;
use std::collections::HashMap;
use std::rc::Rc;

use super::commands::Command;
use super::context::DebuggingContext;
use super::io_utils::Input;
use crate::environment::{InterpreterState, PrimitiveMap, State, StateView};
use crate::errors::{InterpreterError, InterpreterResult};
use crate::interpreter::{ComponentInterpreter, Interpreter};
use crate::interpreter_ir as iir;
use crate::utils::AsRaw;
use calyx::ir::{self, RRC};
pub(super) const SPACING: &str = "    ";
use crate::interpreter::ConstCell;

/// The interactive Calyx debugger. The debugger itself is run with the
/// [main_loop] function while this struct holds auxilliary information used to
/// coordinate the debugging process.
pub struct Debugger {
    context: iir::ComponentCtx,
    main_component: Rc<iir::Component>,
    debugging_ctx: DebuggingContext,
}

impl Debugger {
    pub fn new(
        context: &iir::ComponentCtx,
        main_component: &Rc<iir::Component>,
    ) -> Self {
        Self {
            context: Rc::clone(context),
            main_component: Rc::clone(main_component),
            debugging_ctx: DebuggingContext::default(),
        }
    }

    pub fn main_loop(
        &mut self,
        env: InterpreterState,
        pass_through: bool, //flag to just evaluate the debugger version (non-interactive mode)
    ) -> InterpreterResult<InterpreterState> {
        let mut component_interpreter =
            ComponentInterpreter::from_component(&self.main_component, env);
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
                    | InterpreterError::UnknownCommand(_) => {
                        println!("{:?}", e);
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
                        &component_interpreter.currently_executing_group(),
                    );

                    while breakpoints.is_empty()
                        && !component_interpreter.is_done()
                    {
                        component_interpreter.step()?;
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
                    let state = component_interpreter.get_env();
                    println!("{}", state.state_as_str());
                }
                Command::Print(mut print_list) => {
                    if self.main_component.name == print_list[0] {
                        print_list.remove(0);
                    }

                    let mut current_target =
                        CurrentTarget::Env(&component_interpreter);

                    for (idx, target) in print_list.iter().enumerate() {
                        let current_ref = current_target.borrow();
                        let current_env = current_ref.get_env().unwrap();

                        // lowest level
                        if idx == print_list.len() - 1 {
                            // first look for cell
                            let cell = current_env.get_cell(target);
                            if let Some(cell) = cell {
                                print_cell(&cell, &current_env)
                            } else {
                                let prior = &print_list[idx - 1];

                                if let Some(parent) =
                                    current_env.get_cell(&prior)
                                {
                                    let parent_ref = parent.borrow();
                                    let pt = parent_ref
                                        .ports()
                                        .iter()
                                        .find(|x| x.borrow().name == target);
                                    if let Some(port) = pt {
                                        print_port(port, &current_env)
                                    } else {
                                        // cannot find
                                    }
                                } else {
                                    // cannot find
                                }
                            }
                        }
                        // still walking
                        else {
                            let map = Rc::clone(current_env.get_cell_map());
                            let cell = current_env.get_cell(target);
                            if let Some(rrc_cell) = cell {
                                // need to release these references to replace current
                                // target
                                drop(current_env);
                                drop(current_ref);

                                current_target = CurrentTarget::Target {
                                    name: rrc_cell.as_raw(),
                                    map,
                                }
                            } else {
                                // cannot find

                                break;
                            }
                        }
                    }
                }

                // Command::PrintCell(cell) => {
                //     let cells: Vec<_> =
                //         component_interpreter.get_env().get_cells(&cell);

                //     match cells.len() {
                //         0 => {
                //             println!(
                //                 "{}Unable to print. No cell named '{}'",
                //                 SPACING, &cell
                //             )
                //         }
                //         1 => {
                //             let cell_ref = &cells[0];
                //             print_cell(
                //                 cell_ref,
                //                 &component_interpreter.get_env(),
                //             );
                //         }
                //         _ => {
                //             println!(
                //                 "{}Unable to print. '{}' is ambiguous",
                //                 SPACING, &cell
                //             )
                //         }
                //     }
                // }
                // Command::PrintCellOrPort(first, second) => {
                //     // component & cell/port
                //     if let Some(comp) =
                //         self.context.iter().find(|x| x.name == first)
                //     {
                //         if let Some(cell) = comp.find_cell(&second) {
                //             print_cell(&cell, &component_interpreter.get_env())
                //         } else if let Some(port) =
                //             comp.signature.borrow().find(&second)
                //         {
                //             let env = component_interpreter.get_env();
                //             println!(
                //                 "{}{}.{} = {}",
                //                 SPACING,
                //                 &first,
                //                 &second,
                //                 env.lookup(port.as_raw())
                //             )
                //         } else {
                //             println!("{}Unable to print. Component '{}' has no cell named '{}'", SPACING, &second, &first)
                //         }
                //     }
                //     // cell & port
                //     else {
                //         let cells =
                //             component_interpreter.get_env().get_cells(&first);

                //         // multiple possible cells
                //         if cells.len() > 1 {
                //             println!(
                //                 "{}Unable to print. '{}' is ambiguous",
                //                 SPACING, &first
                //             )
                //         } else if cells.is_empty() {
                //             println!(
                //                 "{}Unable to print. There is no component/cell named '{}'",
                //                 SPACING,
                //                 &first,
                //             )
                //         // past this point the cell vec has one possible entry
                //         } else if cells
                //             .iter()
                //             .all(|x| x.borrow().find(&second).is_none())
                //         {
                //             println!(
                //                 "{}Unable to print. Component '{}' has no cell named '{}'",
                //                 SPACING,
                //                 &first, &second
                //             )
                //         } else {
                //             let entry = &cells[0];

                //             if let Some(port) = entry.borrow().find(&second) {
                //                 print_port(
                //                     &port,
                //                     &component_interpreter.get_env(),
                //                 )
                //             }
                //         }
                //     }
                // }
                // Command::PrintFullySpecified(comp, cell, port) => {
                //     if let Some(comp_ref) =
                //         self.context.iter().find(|x| x.name == comp)
                //     {
                //         if let Some(cell_rrc) = comp_ref.find_cell(&cell) {
                //             let cell_ref = cell_rrc.borrow();
                //             if let Some(port_ref) = cell_ref.find(&port) {
                //                 let state = component_interpreter.get_env();
                //                 print_port(&port_ref, &state)
                //             } else {
                //                 println!("{}Unable to print. Cell '{}' has no port named '{}'", SPACING, cell, port)
                //             }
                //         } else {
                //             println!("{}Unable to print. Component '{}' has no cell named '{}'", SPACING, comp, cell)
                //         }
                //     } else {
                //         println!("{}Unable to print. There is no component named '{}'", SPACING, comp)
                //     }
                // }
                Command::Help => {
                    print!("{}", Command::get_help_string())
                }
                Command::Break(target) => {
                    if self
                        .context
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
                Command::Exit => return Err(InterpreterError::Exit),
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
                component_interpreter.set_go_low();
                return component_interpreter.deconstruct();
            }
        }
    }
}

fn print_cell(target: &RRC<ir::Cell>, state: &StateView) {
    let cell_ref = target.borrow();
    println!("{}{}", SPACING, cell_ref.name());
    for port in cell_ref.ports.iter() {
        println!(
            "{}  {} = {}",
            SPACING,
            port.borrow().name,
            state.lookup(port.as_raw())
        )
    }
}

fn print_port(target: &RRC<ir::Port>, state: &StateView) {
    let port_ref = target.borrow();
    let parent_name = port_ref.get_parent_name();

    println!(
        "{}{}.{} = {}",
        SPACING,
        parent_name,
        port_ref.name,
        state.lookup(port_ref.as_raw())
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
