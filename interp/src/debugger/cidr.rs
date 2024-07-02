use super::{
    commands::{Command, PrintMode},
    debugging_context::context::DebuggingContext,
    interactive_errors::DebuggerError,
    io_utils::Input,
    source::structures::NewSourceMap,
};
use crate::{
    debugger::{source::SourceMap, unwrap_error_message},
    errors::{InterpreterError, InterpreterResult},
    flatten::{
        flat_ir::prelude::GroupIdx,
        setup_simulation, setup_simulation_with_metadata,
        structures::{context::Context, environment::Simulator},
    },
    serialization::PrintCode,
};

use std::{collections::HashSet, path::PathBuf, rc::Rc};

use calyx_ir::Id;

use owo_colors::OwoColorize;
use std::path::Path;

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
/// [Debugger::main_loop] function while this struct holds auxiliary
/// information used to coordinate the debugging process.
pub struct Debugger<C: AsRef<Context> + Clone> {
    interpreter: Simulator<C>,
    // this is technically redundant but is here for mutability reasons
    program_context: C,
    debugging_context: DebuggingContext,
    source_map: Option<SourceMap>,
}

pub type OwnedDebugger = Debugger<Rc<Context>>;

impl OwnedDebugger {
    /// construct a debugger instance from the target calyx file
    /// todo: add support for data files
    pub fn from_file(
        file: &Path,
        lib_path: &Path,
    ) -> InterpreterResult<(Self, NewSourceMap)> {
        let (ctx, map) = setup_simulation_with_metadata(
            &Some(PathBuf::from(file)),
            lib_path,
            false,
        )?;

        let debugger: Debugger<Rc<Context>> =
            Self::new(Rc::new(ctx), None, None)?;

        Ok((debugger, map))
    }
}

impl<C: AsRef<Context> + Clone> Debugger<C> {
    pub fn new(
        program_context: C,
        source_map: Option<SourceMap>,
        data_file: Option<std::path::PathBuf>,
    ) -> InterpreterResult<Self> {
        let interpreter =
            Simulator::build_simulator(program_context.clone(), &data_file)?;

        Ok(Self {
            interpreter,
            program_context,
            debugging_context: DebuggingContext::new(),
            source_map,
        })
    }

    pub fn status(&self) -> ProgramStatus {
        todo!()
    }

    // Go to next step
    pub fn step(&mut self, n: u32) -> InterpreterResult<ProgramStatus> {
        self.do_step(n)?;

        Ok(self.status())
    }

    #[inline]
    fn do_step(&mut self, n: u32) -> InterpreterResult<()> {
        for _ in 0..n {
            self.interpreter.step()?;
        }
        self.interpreter.converge()?;
        Ok(())
    }

    fn do_continue(&mut self) -> InterpreterResult<()> {
        self.debugging_context
            .set_current_time(self.interpreter.get_currently_running_groups());

        let mut breakpoints: Vec<GroupIdx> = vec![];

        while breakpoints.is_empty() && !self.interpreter.is_done() {
            self.interpreter.step()?;
            self.debugging_context
                .advance_time(self.interpreter.get_currently_running_groups());

            for watch in self.debugging_context.process_watchpoints() {
                // for target in watch.target() {
                //     if let Ok(msg) = Self::do_print(
                //         self.main_component.name,
                //         target,
                //         watch.print_code(),
                //         self.interpreter.get_env(),
                //         watch.print_mode(),
                //     ) {
                //         println!("{}", msg.on_black().yellow().bold());
                //     }
                // }
            }

            breakpoints.extend(self.debugging_context.hit_breakpoints());
        }

        if !self.interpreter.is_done() {
            for group in breakpoints {
                println!(
                    "Hit breakpoint: {}",
                    self.program_context
                        .as_ref()
                        .lookup_name(group)
                        .bright_purple()
                        .underline()
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
                Command::Step(n) => self.do_step(n)?,
                Command::StepOver(target) => {
                    self.do_step_over(target)?;
                }
                Command::Continue => self.do_continue()?,
                Command::Empty => {}
                Command::Display => {
                    todo!()
                }
                Command::Print(print_lists, code, print_mode) => {
                    self.do_print(print_lists, code, print_mode);
                }
                Command::Help => {
                    print!("{}", Command::get_help_string().cyan())
                }
                Command::Break(targets) => self.create_breakpoints(targets),

                // breakpoints
                comm @ (Command::Delete(_)
                | Command::Enable(_)
                | Command::Disable(_)) => self.manipulate_breakpoint(comm),

                Command::Exit => return Err(InterpreterError::Exit.into()),

                Command::InfoBreak => {
                    self.debugging_context.print_breakpoints()
                }

                Command::DeleteWatch(targets) => {
                    for target in targets {
                        let target = target
                            .parse_to_watch_ids(self.program_context.as_ref());
                        unwrap_error_message!(target);
                        self.debugging_context.remove_watchpoint(target)
                    }
                }

                Command::Watch(
                    group,
                    watch_pos,
                    print_target,
                    print_code,
                    print_mode,
                ) => self.create_watchpoint(
                    print_target,
                    print_code,
                    print_mode,
                    group,
                    watch_pos,
                ),
                Command::InfoWatch => {
                    self.debugging_context.print_watchpoints()
                }
                Command::PrintPC(override_flag) => {
                    // if self.source_map.is_some() && !override_flag {
                    //     let map = self.source_map.as_ref().unwrap();
                    //     let mut printed = false;
                    //     for x in self
                    //         .interpreter
                    //         .get_active_tree()
                    //         .remove(0)
                    //         .flat_set()
                    //         .into_iter()
                    //     {
                    //         if let Some(output) = map.lookup(x) {
                    //             printed = true;
                    //             println!("{}", output);
                    //         }
                    //     }

                    //     if !printed {
                    //         println!("Falling back to Calyx");
                    //         print!(
                    //             "{}",
                    //             self.interpreter
                    //                 .get_active_tree()
                    //                 .remove(0)
                    //                 .format_tree::<true>(0)
                    //         );
                    //     }
                    // } else {
                    //     print!(
                    //         "{}",
                    //         self.interpreter
                    //             .get_active_tree()
                    //             .remove(0)
                    //             .format_tree::<true>(0)
                    //     );
                    // }
                    todo!()
                }

                Command::Explain => {
                    print!("{}", Command::get_explain_string().blue())
                }
            }
        }

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
                    // let state = final_env.as_state_view();
                    // println!("{}", state.state_as_str().purple());
                    todo!()
                }
                Command::Print(print_lists, code, print_mode) => {
                    for target in print_lists {
                        // match Self::do_print(
                        //     self.main_component.name,
                        //     &target,
                        //     &code,
                        //     final_env.as_state_view(),
                        //     &print_mode,
                        // ) {
                        //     Ok(msg) => println!("{}", msg.green()),
                        //     Err(e) => {
                        //         println!("{}", e.red().underline().bold())
                        //     }
                        // }
                        todo!()
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

    fn create_watchpoint(
        &mut self,
        print_target: Vec<Vec<Id>>,
        print_code: Option<PrintCode>,
        print_mode: PrintMode,
        group: super::commands::ParsedGroupName,
        watch_pos: super::commands::WatchPosition,
    ) {
        let mut error_occurred = false;
        for target in print_target.iter() {
            if let Err(e) =
                self.construct_print_string(target, print_code, print_mode)
            {
                error_occurred = true;
                println!("{}", e.red().bold());
            }
        }

        if error_occurred {
            return;
        }

        let watch_target =
            match group.lookup_group(self.program_context.as_ref()) {
                Ok(v) => v,
                Err(e) => {
                    println!("Error: {}", owo_colors::OwoColorize::red(&e));
                    return;
                }
            };

        self.debugging_context.add_watchpoint(
            watch_target,
            watch_pos,
            (print_target, print_code, print_mode),
        );
    }

    fn do_step_over(
        &mut self,
        target: super::commands::ParsedGroupName,
    ) -> Result<(), crate::errors::BoxedInterpreterError> {
        let target = match target.lookup_group(self.program_context.as_ref()) {
            Ok(v) => v,
            Err(e) => {
                println!("Error: {}", owo_colors::OwoColorize::red(&e));
                return Ok(());
            }
        };

        if !self.interpreter.is_group_running(target) {
            println!("Group is not currently running")
        } else {
            while self.interpreter.is_group_running(target) {
                self.interpreter.step()?;
            }
        };
        Ok(())
    }

    fn create_breakpoints(
        &mut self,
        targets: Vec<super::commands::ParsedGroupName>,
    ) {
        for target in targets {
            let target = target.lookup_group(self.program_context.as_ref());
            unwrap_error_message!(target);

            if self.interpreter.is_group_running(target) {
                println!("Warning: the group {} is already running. This breakpoint will not trigger until the next time the group runs.",
                        self.program_context.as_ref().lookup_name(target).yellow().italic())
            }

            self.debugging_context.add_breakpoint(target);
        }
    }

    fn do_print(
        &self,
        print_lists: Vec<Vec<Id>>,
        code: Option<PrintCode>,
        print_mode: PrintMode,
    ) {
        for target in print_lists {
            todo!()
        }
    }

    fn manipulate_breakpoint(&mut self, command: Command) {
        match &command {
            Command::Disable(targets)
            | Command::Enable(targets)
            | Command::Delete(targets) => {
                for t in targets {
                    let target =
                        t.parse_to_break_ids(self.program_context.as_ref());
                    unwrap_error_message!(target);

                    match &command {
                        Command::Disable(_) => {
                            self.debugging_context.disable_breakpoint(target)
                        }
                        Command::Enable(_) => {
                            self.debugging_context.enable_breakpoint(target)
                        }
                        Command::Delete(_) => {
                            self.debugging_context.remove_breakpoint(target)
                        }
                        _ => unreachable!(),
                    }
                }
            }
            _ => unreachable!("improper use of manipulate_breakpoint"),
        }
    }

    fn construct_print_string(
        &self,
        print_list: &Vec<Id>,
        code: Option<PrintCode>,
        print_mode: PrintMode,
    ) -> Result<String, DebuggerError> {
        todo!()
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
