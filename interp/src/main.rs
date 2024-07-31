use argh::FromArgs;

use calyx_utils::OutputFile;
use interp::{
    configuration,
    debugger::{Debugger, DebuggerInfo, DebuggerReturnStatus},
    errors::InterpreterResult,
    flatten::structures::environment::Simulator,
};

use slog::warn;
use std::{
    io::stdout,
    path::{Path, PathBuf},
};

#[derive(FromArgs)]
/// The Calyx Interpreter
pub struct Opts {
    /// input file
    #[argh(positional)]
    pub file: Option<PathBuf>,

    /// output file, default is stdout
    #[argh(
        option,
        short = 'o',
        long = "output",
        default = "OutputFile::Stdout"
    )]
    pub output: OutputFile,

    /// path to the primitives library
    #[argh(option, short = 'l', default = "Path::new(\"..\").into()")]
    pub lib_path: PathBuf,

    /// path to optional datafile used to initialize memories. If it is not
    /// provided memories will be initialized with zeros
    #[argh(option, long = "data", short = 'd')]
    pub data_file: Option<PathBuf>,

    #[argh(switch, long = "no-verify")]
    /// flag to bypass verification checks before running the program
    /// note: the interpreter will not behave correctly on malformed input
    skip_verification: bool,

    #[argh(switch, long = "allow-invalid-memory-access")]
    /// enables "sloppy" memory access which returns zero when passed an invalid index
    /// rather than erroring
    allow_invalid_memory_access: bool,

    #[argh(switch, long = "allow-par-conflicts")]
    /// enables "sloppy" par simulation which allows parallel overlap when values agree
    allow_par_conflicts: bool,
    #[argh(switch, long = "error-on-overflow")]
    /// upgrades [over | under]flow warnings to errors
    error_on_overflow: bool,
    /// silence warnings
    #[argh(switch, short = 'q', long = "--quiet")]
    quiet: bool,

    /// dump registers as single entry memories
    #[argh(switch, long = "dump-registers")]
    dump_registers: bool,
    /// dumps all memories rather than just external ones
    #[argh(switch, long = "all-memories")]
    dump_all_memories: bool,

    #[argh(subcommand)]
    mode: Option<Command>,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Command {
    Interpret(CommandInterpret),
    Debug(CommandDebug),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "interpret")]
/// [default] Run the given program directly and output the resulting memory dump
struct CommandInterpret {}

#[derive(FromArgs)]
#[argh(subcommand, name = "debug")]
/// Open the program in the interactive debugger
struct CommandDebug {}

/// Interpret a group from a Calyx program
fn main() -> InterpreterResult<()> {
    let opts: Opts = argh::from_env();

    let config = configuration::ConfigBuilder::new()
        .quiet(opts.quiet)
        .allow_invalid_memory_access(opts.allow_invalid_memory_access)
        .error_on_overflow(opts.error_on_overflow)
        .allow_par_conflicts(opts.allow_par_conflicts)
        .dump_registers(opts.dump_registers)
        .dump_all_memories(opts.dump_all_memories)
        .build();

    interp::logging::initialize_logger(config.quiet);

    let log = interp::logging::root();

    if config.allow_par_conflicts {
        warn!(log, "You have enabled Par conflicts. This is not recommended and is usually a bad idea")
    }

    let command = opts.mode.unwrap_or(Command::Interpret(CommandInterpret {}));
    let i_ctx = interp::flatten::setup_simulation(
        &opts.file,
        &opts.lib_path,
        opts.skip_verification,
    )?;

    match &command {
        Command::Interpret(_) => {
            let mut sim = Simulator::build_simulator(&i_ctx, &opts.data_file)?;

            sim.run_program()?;

            let output = sim
                .dump_memories(config.dump_registers, config.dump_all_memories);

            output.serialize(&mut stdout())?;
            Ok(())
        }
        Command::Debug(_) => {
            let mut info: Option<DebuggerInfo> = None;
            loop {
                let debugger = Debugger::new(&i_ctx, &opts.data_file)?;

                let result = debugger.main_loop(info)?;
                match result {
                    DebuggerReturnStatus::Exit => break,
                    DebuggerReturnStatus::Restart(new_info) => {
                        info = Some(*new_info);
                    }
                }
            }
            Ok(())
        }
    }
}
