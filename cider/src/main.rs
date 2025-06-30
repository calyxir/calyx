//! Cider: The Calyx Interpreter and Debugger.

use argh::FromArgs;

use calyx_utils::OutputFile;
use cider::{
    configuration::{self, ColorConfig},
    debugger::{Debugger, DebuggerInfo, DebuggerReturnStatus},
    errors::CiderResult,
    flatten::structures::{context::Context, environment::Simulator},
};

use std::{
    io::stdout,
    path::{Path, PathBuf},
};

#[derive(FromArgs)]
#[argh(help_triggers("-h", "--help"))]
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

    #[argh(switch, long = "error-on-overflow")]
    /// upgrades [over | under]flow warnings to errors
    error_on_overflow: bool,
    /// silence warnings
    #[argh(switch, short = 'q', long = "quiet")]
    quiet: bool,

    /// dump registers as single entry memories
    #[argh(switch, long = "dump-registers")]
    dump_registers: bool,
    /// dumps all memories rather than just external ones
    #[argh(switch, long = "all-memories")]
    dump_all_memories: bool,

    /// enables debug logging
    #[argh(switch, long = "debug-logging")]
    debug_logging: bool,

    /// enable undefined guard check
    #[argh(switch, long = "undef-guard-check")]
    undef_guard_check: bool,

    /// optional wave file output path
    #[argh(option, long = "wave-file")]
    pub wave_file: Option<PathBuf>,

    /// perform data-race analysis
    #[argh(switch, long = "check-data-race")]
    check_data_race: bool,

    /// configure color output (on | off | auto). default = on
    #[argh(option, long = "force-color", default = "ColorConfig::On")]
    color_conf: ColorConfig,

    /// entangle memories with the given name. This option should only be used
    /// if you know what you are doing and as a result is hidden from the help output.
    #[argh(option, hidden_help, long = "entangle")]
    entangle: Vec<String>,

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
fn main() -> CiderResult<()> {
    let opts: Opts = argh::from_env();

    let config = configuration::Config::builder()
        .dump_registers(opts.dump_registers)
        .dump_all_memories(opts.dump_all_memories)
        .build();

    let runtime_config = configuration::RuntimeConfig::builder()
        .check_data_race(opts.check_data_race)
        .debug_logging(opts.debug_logging)
        .quiet(opts.quiet)
        .allow_invalid_memory_access(opts.allow_invalid_memory_access)
        .error_on_overflow(opts.error_on_overflow)
        .undef_guard_check(opts.undef_guard_check)
        .color_config(opts.color_conf)
        .build();

    let command = opts.mode.unwrap_or(Command::Interpret(CommandInterpret {}));
    let i_ctx = cider::flatten::setup_simulation(
        &opts.file,
        &opts.lib_path,
        opts.skip_verification,
        &opts.entangle,
    )?;

    match &command {
        Command::Interpret(_) => {
            let mut sim = Simulator::build_simulator(
                &i_ctx,
                &opts.data_file,
                &opts.wave_file,
                runtime_config,
            )?;

            sim.run_program()?;

            let output = sim
                .dump_memories(config.dump_registers, config.dump_all_memories);

            output.serialize(&mut stdout())?;
            Ok(())
        }
        Command::Debug(_) => {
            let mut info: Option<DebuggerInfo<&Context>> = None;
            loop {
                let debugger = Debugger::new(
                    &i_ctx,
                    &opts.data_file,
                    &opts.wave_file,
                    runtime_config,
                )?;

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
