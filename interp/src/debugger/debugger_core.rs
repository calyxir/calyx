use super::{
    commands::{Command, ParsedBreakPointID, ParsedGroupName, PrintMode},
    debugging_context::context::DebuggingContext,
    io_utils::Input,
    source::structures::NewSourceMap,
};
use crate::{
    debugger::{source::SourceMap, unwrap_error_message},
    errors::{InterpreterError, InterpreterResult},
    flatten::{
        flat_ir::prelude::GroupIdx,
        setup_simulation_with_metadata,
        structures::{
            context::Context,
            environment::{Path as ParsePath, PathError, Simulator},
        },
    },
    serialization::PrintCode,
};

use std::{collections::HashSet, path::PathBuf, rc::Rc};

use itertools::Itertools;
use owo_colors::OwoColorize;
use std::path::Path as FilePath;

/// Constant amount of space used for debugger messages
pub(super) const SPACING: &str = "    ";

/// ProgramStatus returns the status of the program, helpful
/// status contains the set of running groups, done states if the program
/// is finished or not. If program is done then the debugger is exited
pub struct ProgramStatus {
    /// all groups currently running
    status: HashSet<(String, String)>,
    /// states whether the program has finished
    done: bool,
}

impl ProgramStatus {
    /// get status
    pub fn get_status(&self) -> &HashSet<(String, String)> {
        &self.status
    }

    /// see if program is done
    pub fn get_done(&self) -> bool {
        self.done
    }
}

/// An opaque wrapper type for internal debugging information
pub struct DebuggerInfo {
    ctx: DebuggingContext,
    input_stream: Input,
}

pub enum DebuggerReturnStatus {
    Restart(Box<DebuggerInfo>),
    Exit,
}

/// The interactive Calyx debugger. The debugger itself is run with the
/// [Debugger::main_loop] function while this struct holds auxiliary
/// information used to coordinate the debugging process.
pub struct Debugger<C: AsRef<Context> + Clone> {
    interpreter: Simulator<C>,
    // this is technically redundant but is here for mutability reasons
    program_context: C,
    debugging_context: DebuggingContext,
    _source_map: Option<SourceMap>,
}

pub type OwnedDebugger = Debugger<Rc<Context>>;

impl OwnedDebugger {
    /// construct a debugger instance from the target calyx file
    /// todo: add support for data files
    pub fn from_file(
        file: &FilePath,
        lib_path: &FilePath,
    ) -> InterpreterResult<(Self, NewSourceMap)> {
        let (ctx, map) = setup_simulation_with_metadata(
            &Some(PathBuf::from(file)),
            lib_path,
            false,
        )?;

        let debugger: Debugger<Rc<Context>> = Self::new(Rc::new(ctx), &None)?;

        Ok((debugger, map))
    }
}

impl<C: AsRef<Context> + Clone> Debugger<C> {
    pub fn new(
        program_context: C,
        data_file: &Option<std::path::PathBuf>,
    ) -> InterpreterResult<Self> {
        let mut interpreter =
            Simulator::build_simulator(program_context.clone(), data_file)?;
        interpreter.converge()?;

        Ok(Self {
            interpreter,
            program_context,
            debugging_context: DebuggingContext::new(),
            _source_map: None,
        })
    }

    pub fn status(&self) -> ProgramStatus {
        ProgramStatus {
            status: self
                .interpreter
                .get_currently_running_groups()
                .map(|x| {
                    let group_name =
                        self.program_context.as_ref().lookup_name(x).clone();
                    let parent_comp = self
                        .program_context
                        .as_ref()
                        .get_component_from_group(x);
                    let parent_name = self
                        .program_context
                        .as_ref()
                        .lookup_name(parent_comp)
                        .clone();
                    (parent_name, group_name)
                })
                .collect(),
            done: self.interpreter.is_done(),
        }
    }

    pub fn get_cells(
        &self,
    ) -> impl Iterator<Item = (String, Vec<String>)> + '_ {
        self.interpreter.env().iter_cells()
    }

    // Go to next step
    pub fn step(&mut self, n: u32) -> InterpreterResult<ProgramStatus> {
        self.do_step(n)?;

        Ok(self.status())
    }

    pub fn set_breakpoints(&mut self, breakpoints: Vec<ParsedGroupName>) {
        self.create_breakpoints(breakpoints)
    }

    pub fn delete_breakpoints(&mut self, breakpoints: Vec<ParsedGroupName>) {
        let parsed_bp_ids: Vec<ParsedBreakPointID> = breakpoints
            .into_iter()
            .map(ParsedBreakPointID::from)
            .collect_vec();
        self.manipulate_breakpoint(Command::Delete(parsed_bp_ids));
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
            // TODO griffin: figure out how to skip this convergence
            self.interpreter.converge()?;
            self.debugging_context
                .advance_time(self.interpreter.get_currently_running_groups());

            for (_idx, watch) in self.debugging_context.hit_watchpoints() {
                let print_tuple = watch.print_details();

                for target in print_tuple.target() {
                    if let Err(e) = self.print_from_path(
                        target,
                        print_tuple.print_code(),
                        *print_tuple.print_mode(),
                    ) {
                        println!("{}", e.red().bold());
                    };
                }
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

    pub fn main_loop(
        mut self,
        info: Option<DebuggerInfo>,
    ) -> InterpreterResult<DebuggerReturnStatus> {
        let (input_stream, dbg_ctx) = info
            .map(|x| (Some(x.input_stream), Some(x.ctx)))
            .unwrap_or_else(|| (None, None));

        if let Some(dbg_ctx) = dbg_ctx {
            self.debugging_context = dbg_ctx;
        }

        let mut input_stream =
            input_stream.map(Ok).unwrap_or_else(Input::new)?;

        println!(
            "==== {}: The {}alyx {}nterpreter and {}bugge{} ====",
            "Cider".bold(),
            "C".underline(),
            "I".underline(),
            "De".underline(),
            "r".underline()
        );

        let mut err_count = 0_u8;

        while !self.interpreter.is_done() {
            let comm = input_stream.next_command();
            let comm = match comm {
                Ok(c) => {
                    err_count = 0;
                    c
                }
                Err(e) => match *e {
                    InterpreterError::InvalidCommand(_)
                    | InterpreterError::UnknownCommand(_)
                    | InterpreterError::ParseError(_) => {
                        println!("Error: {}", e.red().bold());
                        err_count += 1;
                        if err_count == 3 {
                            println!(
                                "Type {} for a list of commands or {} for usage examples.",
                                "help".yellow().bold().underline(),
                                "explain".yellow().bold().underline()
                            );
                            err_count = 0;
                        }
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
                    println!("COMMAND NOT YET IMPLEMENTED");
                }
                Command::Print(print_lists, code, print_mode) => {
                    for target in print_lists {
                        if let Err(e) = self.do_print(&target, code, print_mode)
                        {
                            println!("{}", e.red().bold());
                        };
                    }
                }
                Command::Help => {
                    print!("{}", Command::get_help_string())
                }
                Command::Break(targets) => self.create_breakpoints(targets),

                // breakpoints
                comm @ (Command::Delete(_)
                | Command::Enable(_)
                | Command::Disable(_)) => self.manipulate_breakpoint(comm),

                Command::Exit => {
                    println!("Exiting.");
                    return Ok(DebuggerReturnStatus::Exit);
                }

                Command::InfoBreak => self
                    .debugging_context
                    .print_breakpoints(self.program_context.as_ref()),

                Command::DeleteWatch(targets) => {
                    for target in targets {
                        let target = target
                            .parse_to_watch_ids(self.program_context.as_ref());
                        unwrap_error_message!(target);
                        self.debugging_context.remove_watchpoint(target)
                    }
                }

                Command::EnableWatch(targets) => {
                    for target in targets {
                        let target = target
                            .parse_to_watch_ids(self.program_context.as_ref());
                        unwrap_error_message!(target);
                        self.debugging_context.enable_watchpoint(target)
                    }
                }

                Command::DisableWatch(targets) => {
                    for target in targets {
                        let target = target
                            .parse_to_watch_ids(self.program_context.as_ref());
                        unwrap_error_message!(target);
                        self.debugging_context.disable_watchpoint(target)
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
                Command::InfoWatch => self
                    .debugging_context
                    .print_watchpoints(self.interpreter.env()),
                Command::PrintPC(_override_flag) => {
                    self.interpreter.print_pc();
                }

                Command::Explain => {
                    print!("{}", Command::get_explain_string())
                }

                Command::Restart => {
                    return Ok(DebuggerReturnStatus::Restart(Box::new(
                        DebuggerInfo {
                            ctx: self.debugging_context,
                            input_stream,
                        },
                    )));
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
                    println!("COMMAND NOT YET IMPLEMENTED");
                }
                Command::Print(print_lists, code, print_mode) => {
                    for target in print_lists {
                        if let Err(e) = self.do_print(&target, code, print_mode)
                        {
                            println!("{}", e.red().bold());
                        };
                    }
                }

                Command::Help => {
                    print!("{}", Command::get_help_string())
                }
                Command::Exit => {
                    println!("Exiting.");
                    return Ok(DebuggerReturnStatus::Exit);
                }
                Command::Explain => {
                    print!("{}", Command::get_explain_string())
                }
                Command::Restart => {
                    return Ok(DebuggerReturnStatus::Restart(Box::new(
                        DebuggerInfo {
                            ctx: self.debugging_context,
                            input_stream,
                        },
                    )));
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
        print_target: Vec<Vec<String>>,
        print_code: Option<PrintCode>,
        print_mode: PrintMode,
        group: super::commands::ParsedGroupName,
        watch_pos: super::commands::WatchPosition,
    ) {
        let mut error_occurred = false;
        let mut paths = Vec::new();
        for target in print_target.iter() {
            match self.interpreter.traverse_name_vec(target) {
                Ok(path) => {
                    paths.push(path);
                }
                Err(e) => {
                    error_occurred = true;
                    println!("{}", e.red().bold());
                    continue;
                }
            }
        }

        if error_occurred {
            println!("{}", "No watchpoints have been added.".red());
            return;
        }

        let watch_target =
            match group.lookup_group(self.program_context.as_ref()) {
                Ok(v) => v,
                Err(e) => {
                    println!("Error: {}", e.red());
                    return;
                }
            };

        self.debugging_context.add_watchpoint(
            watch_target,
            watch_pos,
            (paths, print_code, print_mode),
        );
    }

    fn do_step_over(
        &mut self,
        target: super::commands::ParsedGroupName,
    ) -> Result<(), crate::errors::BoxedInterpreterError> {
        let target = match target.lookup_group(self.program_context.as_ref()) {
            Ok(v) => v,
            Err(e) => {
                println!("Error: {}", e.red());
                return Ok(());
            }
        };

        if !self.interpreter.is_group_running(target) {
            println!("Group is not currently running")
        } else {
            while self.interpreter.is_group_running(target) {
                self.interpreter.step()?;
            }
            self.interpreter.converge()?;
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
        target: &[String],
        code: Option<PrintCode>,
        print_mode: PrintMode,
    ) -> Result<(), PathError> {
        let traversal_res = self.interpreter.traverse_name_vec(target)?;

        self.print_from_path(&traversal_res, &code, print_mode)?;

        Ok(())
    }

    fn print_from_path(
        &self,
        path: &ParsePath,
        code: &Option<PrintCode>,
        mode: PrintMode,
    ) -> Result<(), PathError> {
        let code = code.unwrap_or(PrintCode::Binary);

        let name_override = match path {
            ParsePath::Cell(_) | ParsePath::Port(_) => None,
            ParsePath::AbstractCell(_) | ParsePath::AbstractPort { .. } => {
                Some(path.as_string(self.interpreter.env()))
            }
        };

        let resolved = path.resolve_path(self.interpreter.env())?;
        match resolved {
            crate::flatten::structures::environment::PathResolution::Cell(
                cell,
            ) => {
                if let PrintMode::State = mode {
                    if let Some(state) = self.interpreter.format_cell_state(
                        cell,
                        code,
                        name_override.as_deref(),
                    ) {
                        println!("{}", state);
                        return Ok(());
                    } else {
                        println!("{}","Target cell has no internal state, printing port information instead".red());
                    }
                }

                println!(
                    "{}",
                    self.interpreter.format_cell_ports(
                        cell,
                        code,
                        name_override.as_deref()
                    )
                )
            }
            crate::flatten::structures::environment::PathResolution::Port(
                port,
            ) => {
                let path_str = name_override
                    .unwrap_or_else(|| self.interpreter.get_full_name(port));

                println!(
                    "{path_str} = {}",
                    self.interpreter.format_port_value(port, code)
                )
            }
        }

        Ok(())
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
}
