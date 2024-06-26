use super::{
    commands::{Command, PrintMode},
    context::DebuggingContext,
    interactive_errors::DebuggerError,
    io_utils::Input,
    source::structures::NewSourceMap,
};
use crate::{
    debugger::source::SourceMap,
    errors::{InterpreterError, InterpreterResult},
    flatten::structures::{context::Context, environment::Simulator},
    serialization::{PrintCode, Serializable},
};

use std::collections::HashSet;

use calyx_ir::{Id, RRC};

use owo_colors::OwoColorize;
use std::path::Path;
use std::{cell::Ref, collections::HashMap, rc::Rc};
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
    pub fn generate() -> Self {
        todo!()
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
pub struct Debugger<'a> {
    simulator: Simulator<'a>,
    // this is technically redundant but is here for mutability reasons
    program_context: &'a Context,
    debugging_context: DebuggingContext,
    source_map: Option<SourceMap>,
}

impl<'a> Debugger<'a> {
    /// construct a debugger instance from the target calyx file
    pub fn from_file(
        file: &Path,
        lib_path: &Path,
    ) -> InterpreterResult<(Self, NewSourceMap)> {
        // Make NewSourceMap, if we can't then we explode
        // let mapping = ctx
        //     .metadata
        //     .map(|metadata| parse_metadata(&metadata))
        //     .unwrap_or_else(|| Err(InterpreterError::MissingMetaData.into()))?;

        // Ok((
        //     Debugger::new(&components, main_component, None, env).unwrap(),
        //     mapping,
        // ))
        todo!();
    }

    pub fn new(
        program_context: &'a Context,
        source_map: Option<SourceMap>,
        data_file: Option<std::path::PathBuf>,
    ) -> InterpreterResult<Self> {
        let simulator =
            Simulator::build_simulator(&program_context, &data_file)?;

        Ok(Self {
            simulator,
            program_context,
            debugging_context: todo!(),
            source_map,
        })
    }

    pub fn status(&self) -> ProgramStatus {
        todo!()
    }

    // Go to next step
    pub fn step(&mut self, n: u64) -> InterpreterResult<ProgramStatus> {
        self.do_step(n)?;

        Ok(self.status())
    }

    #[inline]
    fn do_step(
        &mut self,
        n: u64,
    ) -> Result<(), crate::errors::BoxedInterpreterError> {
        for _ in 0..n {
            self.interpreter.step()?;
        }
        self.interpreter.converge()?;
        Ok(())
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

    pub fn main_loop(mut self) -> InterpreterResult<()> {
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

    // fn do_print(
    //     main_comp_name: Id,
    //     print_list: &[Id],
    //     code: &Option<PrintCode>,
    //     root: StateView,
    //     print_mode: &PrintMode,
    // ) -> Result<String, DebuggerError> {
    //     let orig_string = print_list
    //         .iter()
    //         .map(|s| s.id.as_str())
    //         .collect::<Vec<_>>()
    //         .join(".");

    //     let mut iter = print_list.iter();

    //     let length = if main_comp_name == print_list[0] {
    //         iter.next();
    //         print_list.len() - 1
    //     } else {
    //         print_list.len()
    //     };

    //     let mut current_target = CurrentTarget::Env(&root);

    //     for (idx, target) in iter.enumerate() {
    //         let current_ref = current_target.borrow();
    //         let current_env = current_ref.get_env().unwrap();

    //         // lowest level
    //         if idx == length - 1 {
    //             // first look for cell
    //             let cell = current_env.get_cell(*target);
    //             if let Some(cell) = cell {
    //                 return Ok(print_cell(
    //                     &cell,
    //                     &current_env,
    //                     code,
    //                     print_mode,
    //                 ));
    //             } else if idx != 0 {
    //                 let prior = &print_list[idx - 1];

    //                 if let Some(parent) = current_env.get_cell(*prior) {
    //                     let parent_ref = parent.borrow();
    //                     let pt = parent_ref
    //                         .ports()
    //                         .iter()
    //                         .find(|x| x.borrow().name == target);
    //                     if let Some(port) = pt {
    //                         return Ok(print_port(
    //                             port,
    //                             &current_env,
    //                             None,
    //                             code,
    //                         ));
    //                     } else {
    //                         return Err(DebuggerError::CannotFind(orig_string));
    //                         // cannot find
    //                     }
    //                 } else if let Some(port) =
    //                     current_env.get_comp().signature.borrow().find(target)
    //                 {
    //                     return Ok(print_port(
    //                         &port,
    //                         &current_env,
    //                         Some(print_list[idx - 1]),
    //                         code,
    //                     ));
    //                 } else {
    //                     // cannot find
    //                     return Err(DebuggerError::CannotFind(orig_string));
    //                 }
    //             } else {
    //                 return Err(DebuggerError::CannotFind(orig_string));
    //             }
    //         }
    //         // still walking
    //         else {
    //             let map = Rc::clone(current_env.get_cell_map());
    //             let cell = current_env.get_cell(*target);
    //             if let Some(rrc_cell) = cell {
    //                 // need to release these references to replace current
    //                 // target
    //                 if map.borrow()[&rrc_cell.as_raw()].get_state().is_some() {
    //                     drop(current_env);
    //                     drop(current_ref);

    //                     current_target = CurrentTarget::Target {
    //                         name: rrc_cell.as_raw(),
    //                         map,
    //                     }
    //                 }
    //                 // otherwise leave the same
    //             } else {
    //                 // cannot find
    //                 return Err(DebuggerError::CannotFind(orig_string));
    //             }
    //         }
    //     }

    //     unreachable!()
    // }
}

// fn print_cell(
//     target: &RRC<ir::Cell>,
//     state: &StateView,
//     code: &Option<PrintCode>,
//     mode: &PrintMode,
// ) -> String {
//     let cell_ref = target.borrow();

//     match mode {
//         PrintMode::State => {
//             let actual_code =
//                 code.as_ref().copied().unwrap_or(PrintCode::Binary);
//             let cell_state = state.get_cell_state(&cell_ref, &actual_code);
//             if matches!(&cell_state, &Serializable::Empty) {
//                 print_cell(target, state, code, &PrintMode::Port)
//             } else {
//                 format!(
//                     "{}{} = {}",
//                     SPACING,
//                     cell_ref.name().green().bold(),
//                     cell_state.blue().bold()
//                 )
//             }
//         }

//         PrintMode::Port => {
//             let mut output: String = String::new();
//             writeln!(output, "{}{}", SPACING, cell_ref.name().red())
//                 .expect("Something went wrong trying to print the port");
//             for port in cell_ref.ports.iter() {
//                 let v = state.lookup(port.as_raw());
//                 writeln!(
//                     output,
//                     "{}  {} = {}",
//                     SPACING,
//                     port.borrow().name.red(),
//                     if let Some(code) = code {
//                         match code {
//                             PrintCode::Unsigned => {
//                                 format!("{}", v.as_unsigned())
//                             }
//                             PrintCode::Signed => {
//                                 format!("{}", v.as_signed().green())
//                             }
//                             PrintCode::UFixed(num) => {
//                                 format!("{}", v.as_ufp(*num).blue())
//                             }
//                             PrintCode::SFixed(num) => {
//                                 format!("{}", v.as_sfp(*num).purple())
//                             }
//                             PrintCode::Binary => format!("{}", v.cyan()),
//                         }
//                     } else {
//                         format!("{}", &v.magenta())
//                     }
//                 )
//                 .expect("Something went wrong trying to print the port");
//             }
//             output
//         }
//     }
// }

// fn print_port(
//     target: &RRC<ir::Port>,
//     state: &StateView,
//     prior_name: Option<ir::Id>,
//     code: &Option<PrintCode>,
// ) -> String {
//     let port_ref = target.borrow();
//     let parent_name = if let Some(prior) = prior_name {
//         prior
//     } else {
//         port_ref.get_parent_name()
//     };

//     let v = state.lookup(port_ref.as_raw());
//     let code = code.as_ref().copied().unwrap_or(PrintCode::Binary);

//     format!(
//         "{}{}.{} = {}",
//         SPACING,
//         parent_name.red(),
//         port_ref.name.green(),
//         match code {
//             PrintCode::Unsigned => format!("{}", v.as_unsigned()),
//             PrintCode::Signed => format!("{}", v.as_signed()),
//             PrintCode::UFixed(num) => format!("{}", v.as_ufp(num)),
//             PrintCode::SFixed(num) => format!("{}", v.as_sfp(num)),
//             PrintCode::Binary => format!("{}", v),
//         }
//     )
// }
