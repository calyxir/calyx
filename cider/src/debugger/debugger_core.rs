use super::{
    commands::{
        Command, ParseNodes, ParsePath, ParsedBreakPointID, ParsedControlName,
        PrintMode,
    },
    debugging_context::context::DebuggingContext,
    io_utils::Input,
    source::structures::NewSourceMap,
};
use crate::{
    configuration::RuntimeConfig,
    debugger::{
        commands::PrintCommand, source::SourceMap, unwrap_error_message,
    },
    errors::{BoxedCiderError, CiderError, CiderResult},
    flatten::{
        flat_ir::{
            base::ComponentIdx,
            base::{GlobalCellIdx, PortValue},
            prelude::{Control, ControlIdx, GroupIdx},
        },
        setup_simulation_with_metadata,
        structures::{
            context::Context,
            environment::{Path, PathError, Simulator},
        },
        text_utils::{Color, print_debugger_welcome},
    },
    serialization::PrintCode,
};

use std::{collections::HashSet, num::NonZeroU32, path::PathBuf, rc::Rc};

use itertools::Itertools;
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

/// An opaque wrapper type for internal debugging information. This can only be
/// obtained by calling [Debugger::main_loop] and receiving a [DebuggerReturnStatus::Restart] return
/// value.
pub struct DebuggerInfo {
    ctx: DebuggingContext,
    input_stream: Input,
}
/// An enum indicating the non-error return status of the debugger
pub enum DebuggerReturnStatus {
    /// Debugger exited with a restart command and should be reinitialized with
    /// the returned information. Comes from [Command::Restart].
    Restart(Box<DebuggerInfo>),
    /// Debugger exited normally with an exit command. Comes from [Command::Exit].
    Exit,
}

pub enum StoppedReason {
    Done,
    Breakpoint(Vec<(String, String)>), //adapter then looks up line
    PauseReq,
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

/// A type alias for the debugger using an Rc of the context. Use this in cases
/// where the use of lifetimes would be a hindrance.
pub type OwnedDebugger = Debugger<Rc<Context>>;

impl OwnedDebugger {
    /// construct a debugger instance from the target calyx file
    /// todo: add support for data files
    pub fn from_file(
        file: &FilePath,
        lib_path: &FilePath,
    ) -> CiderResult<(Self, NewSourceMap)> {
        let (ctx, map) = setup_simulation_with_metadata(
            &Some(PathBuf::from(file)),
            lib_path,
            false,
        )?;

        let debugger: Debugger<Rc<Context>> =
            Self::new(Rc::new(ctx), &None, &None, RuntimeConfig::default())?;

        Ok((debugger, map))
    }
}

impl<C: AsRef<Context> + Clone> Debugger<C> {
    /// Construct a new debugger instance from the target calyx file
    pub fn new(
        program_context: C,
        data_file: &Option<std::path::PathBuf>,
        wave_file: &Option<std::path::PathBuf>,
        runtime_config: RuntimeConfig,
    ) -> CiderResult<Self> {
        let mut interpreter = Simulator::build_simulator(
            program_context.clone(),
            data_file,
            wave_file,
            runtime_config,
        )?;
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
                .map(|x| self.grp_idx_to_name(x))
                .collect(),
            done: self.interpreter.is_done(),
        }
    }

    fn grp_idx_to_name(&self, x: GroupIdx) -> (String, String) {
        let group_name = self.program_context.as_ref().lookup_name(x).clone();
        let parent_comp =
            self.program_context.as_ref().get_component_from_group(x);
        let parent_name = self
            .program_context
            .as_ref()
            .lookup_name(parent_comp)
            .clone();
        (parent_name, group_name)
    }

    pub fn get_all_cells(
        &self,
    ) -> impl Iterator<Item = (String, Vec<(String, PortValue)>)> {
        self.interpreter.env().iter_cells()
    }
    /// Get cell names and port values for the component specified by cmp_idx
    pub fn get_comp_cells(
        &self,
        cmp_idx: GlobalCellIdx,
    ) -> impl Iterator<Item = (String, Vec<(String, PortValue)>)> {
        // component idx -> global cell idx
        self.interpreter.env().iter_cmpt_cells(cmp_idx)
    }
    /// Get all components in the environment
    pub fn get_components(
        &self,
    ) -> impl Iterator<Item = (GlobalCellIdx, &String)> {
        //this gets the names AND idx, now how to get the lines T.T
        self.interpreter.env().iter_compts()
    }

    // Go to next step
    pub fn step(&mut self, n: u32) -> CiderResult<ProgramStatus> {
        self.do_step(n)?;

        Ok(self.status())
    }

    pub fn set_breakpoints(&mut self, breakpoints: Vec<ParsedControlName>) {
        self.create_breakpoints(breakpoints)
    }

    pub fn delete_breakpoints(&mut self, breakpoints: Vec<ParsedControlName>) {
        let parsed_bp_ids: Vec<ParsedBreakPointID> = breakpoints
            .into_iter()
            .map(ParsedBreakPointID::from)
            .collect_vec();
        self.manipulate_breakpoint(Command::Delete(parsed_bp_ids));
    }

    pub fn cont(&mut self) -> Result<StoppedReason, BoxedCiderError> {
        self.do_continue()?; //need to error handle
        let bps = self
            .debugging_context
            .hit_breakpoints()
            .map(|x| self.grp_idx_to_name(x))
            .collect_vec();
        if self.interpreter.is_done() {
            Ok(StoppedReason::Done)
        } else if !bps.is_empty() {
            Ok(StoppedReason::Breakpoint(bps))
        } else {
            unreachable!()
        }
    }

    #[inline]
    fn do_step(&mut self, n: u32) -> CiderResult<()> {
        for _ in 0..n {
            self.interpreter.step()?;
        }
        self.interpreter.converge()?;
        Ok(())
    }

    fn do_continue(&mut self) -> CiderResult<()> {
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
                        println!("{}", e.stylize_error());
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
                        .stylize_breakpoint()
                );
            }
            self.interpreter.converge()?;
        };
        Ok(())
    }

    // so on and so forth

    /// The main loop of the debugger. This function is the entry point for the
    /// debugger. It takes an optional [DebuggerInfo] struct which contains the
    /// input stream and the debugging context which allows the debugger to
    /// retain command history and other state after a restart. If not provided,
    /// a fresh context and input stream will be used instead.
    pub fn main_loop(
        mut self,
        info: Option<DebuggerInfo>,
    ) -> CiderResult<DebuggerReturnStatus> {
        let (input_stream, dbg_ctx) = info
            .map(|x| (Some(x.input_stream), Some(x.ctx)))
            .unwrap_or_else(|| (None, None));

        if let Some(dbg_ctx) = dbg_ctx {
            self.debugging_context = dbg_ctx;
        }

        let mut input_stream =
            input_stream.map(Ok).unwrap_or_else(Input::new)?;

        print_debugger_welcome();

        let mut err_count = 0_u8;

        while !self.interpreter.is_done() {
            let comm = input_stream.next_command();
            let comm = match comm {
                Ok(c) => {
                    err_count = 0;
                    c
                }
                Err(e) => match *e {
                    CiderError::InvalidCommand(_)
                    | CiderError::UnknownCommand(_)
                    | CiderError::ParseError(_) => {
                        println!("Error: {}", e.stylize_error());
                        err_count += 1;
                        if err_count == 3 {
                            println!(
                                "Type {} for a list of commands or {} for usage examples.",
                                "help".stylize_command(),
                                "explain".stylize_command()
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
                Command::StepOver(target, bound) => {
                    self.do_step_over(target, bound)?;
                }
                Command::Continue => self.do_continue()?,
                Command::Empty => {}
                Command::Display => {
                    for cell in self.interpreter.iter_active_cells() {
                        println!(
                            "{}",
                            self.interpreter.format_cell_ports(
                                cell,
                                PrintCode::Binary,
                                None
                            )
                        )
                    }
                }
                Command::Print(print_lists, code, print_mode) => {
                    for target in print_lists {
                        if let Err(e) = self.do_print(&target, code, print_mode)
                        {
                            println!("{}", e.stylize_error());
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

                Command::PrintPC(print_mode) => match print_mode {
                    PrintCommand::Normal => {
                        if let Some(source_info) = &self
                            .program_context
                            .as_ref()
                            .secondary
                            .source_info_table
                        {
                            let mut printed_position = false;
                            for position in
                                self.interpreter.env().iter_positions()
                            {
                                if let Some(location) =
                                    source_info.get_position(position)
                                {
                                    println!(
                                        "{}:{}",
                                        source_info
                                            .lookup_file_path(location.file)
                                            .display(),
                                        location.line
                                    );
                                    printed_position = true;
                                }
                            }

                            if !printed_position {
                                println!(
                                    "Source info unavailable, falling back to Calyx"
                                );
                                self.interpreter.print_pc();
                            }
                        } else {
                            self.interpreter.print_pc();
                        }
                    }
                    PrintCommand::PrintCalyx => {
                        self.interpreter.print_pc();
                    }
                    PrintCommand::PrintNodes => {
                        self.interpreter.print_pc_string();
                    }
                },
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

        println!(
            "Main component has finished executing. Debugger is now in inspection mode."
        );

        loop {
            let comm = input_stream.next_command();
            let comm = match comm {
                Ok(c) => c,
                Err(e) => match *e {
                    CiderError::InvalidCommand(_)
                    | CiderError::UnknownCommand(_)
                    | CiderError::ParseError(_) => {
                        println!("Error: {}", e.stylize_error());
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
                            println!("{}", e.stylize_error());
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
        group: super::commands::ParsedControlName,
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
                    println!("{}", e.stylize_error());
                    continue;
                }
            }
        }

        if error_occurred {
            println!("{}", "No watchpoints have been added.".stylize_error());
            return;
        }

        let watch_target =
            match group.lookup_group_watch(self.program_context.as_ref()) {
                Ok(v) => v,
                Err(e) => {
                    println!("Error: {}", e.stylize_error());
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
        target: super::commands::ParsedControlName,
        bound: Option<NonZeroU32>,
    ) -> Result<(), crate::errors::BoxedCiderError> {
        let target = match target.lookup_group(self.program_context.as_ref()) {
            Ok(v) => v,
            Err(e) => {
                println!("Error: {}", e.stylize_error());
                return Ok(());
            }
        };

        let mut bound: Option<u32> = bound.map(|x| x.into());

        if !self.interpreter.is_group_running(target) {
            println!("Group is not currently running")
        } else {
            while self.interpreter.is_group_running(target) {
                if let Some(current_count) = bound.as_mut() {
                    if *current_count == 0 {
                        println!("Bound reached, group is still running.");
                        break;
                    } else {
                        *current_count -= 1;
                    }
                }

                self.interpreter.step()?;
            }
            self.interpreter.converge()?;
        };
        Ok(())
    }

    fn create_breakpoints(
        &mut self,
        targets: Vec<super::commands::ParsedControlName>,
    ) {
        // TODO: THIS DOES IN TERMS OF GROUPS, MUST EXPAND FUNCTIONALITY FOR SPECIFIC ENABLES
        for target in targets {
            let target = target.lookup_group(self.program_context.as_ref());
            unwrap_error_message!(target);

            if self.interpreter.is_group_running(target) {
                println!(
                    "Warning: the group {} is already running. This breakpoint will not trigger until the next time the group runs.",
                    self.program_context
                        .as_ref()
                        .lookup_name(target)
                        .stylize_warning()
                )
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
        path: &Path,
        code: &Option<PrintCode>,
        mode: PrintMode,
    ) -> Result<(), PathError> {
        let code = code.unwrap_or(PrintCode::Binary);

        let name_override = match path {
            Path::Cell(_) | Path::Port(_) => None,
            Path::AbstractCell(_) | Path::AbstractPort { .. } => {
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
                        println!("{}","Target cell has no internal state, printing port information instead".stylize_warning());
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

    /// Returns the controlidx of the last node in the given path and component idx
    pub fn path_idx(
        &self,
        component: ComponentIdx,
        path: ParsePath,
    ) -> ControlIdx {
        let path_nodes = path.get_path();
        let env = self.interpreter.env();
        let ctx = env.ctx();

        let component_map = &ctx.primary.components;
        let control_map = &ctx.primary.control;

        // Get nodes
        let component_node = component_map.get(component).unwrap();

        let mut control_id = component_node.control().unwrap();

        let mut control_node = &control_map.get(control_id).unwrap().control;
        for parse_node in path_nodes {
            match parse_node {
                ParseNodes::Body => match control_node {
                    Control::While(while_struct) => {
                        control_id = while_struct.body();
                    }
                    Control::Repeat(repeat_struct) => {
                        control_id = repeat_struct.body;
                    }
                    _ => {
                        // TODO: Dont want to crash if invalid path, return result type w/ error malformed
                        panic!();
                    }
                },
                ParseNodes::If(branch) => match control_node {
                    Control::If(if_struct) => {
                        control_id = if branch {
                            if_struct.tbranch()
                        } else {
                            if_struct.fbranch()
                        };
                    }
                    _ => {
                        panic!();
                    }
                },
                ParseNodes::Offset(child) => match control_node {
                    Control::Par(par_struct) => {
                        let children = par_struct.stms();
                        control_id = children[child as usize];
                    }
                    Control::Seq(seq_struct) => {
                        let children = seq_struct.stms();
                        control_id = children[child as usize]
                    }
                    _ => {
                        panic!();
                    }
                },
            }
            control_node = control_map.get(control_id).unwrap();
        }
        control_id
    }
}
