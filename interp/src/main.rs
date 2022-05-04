use calyx::{frontend, ir, pass_manager::PassManager, utils::OutputFile};
use interp::{
    configuration,
    debugger::{source::SourceMap, Debugger},
    environment::InterpreterState,
    errors::{InterpreterError, InterpreterResult},
    interpreter::ComponentInterpreter,
    interpreter_ir as iir,
};

use argh::FromArgs;
use slog::warn;
use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(FromArgs)]
/// The Calyx Interpreter
pub struct Opts {
    /// input file
    #[argh(positional, from_str_fn(read_path))]
    pub file: Option<PathBuf>,

    /// output file, default is stdout
    #[argh(
        option,
        short = 'o',
        long = "output",
        default = "OutputFile::default()"
    )]
    pub output: OutputFile,

    /// path to the primitives library
    #[argh(option, short = 'l', default = "Path::new(\"..\").into()")]
    pub lib_path: PathBuf,

    /// path to optional datafile used to initialze memories. If it is not
    /// provided memories will be initialzed with zeros
    #[argh(option, long = "data", short = 'd', from_str_fn(read_path))]
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

    /// serializes values using base64 encoding
    #[argh(switch, long = "raw")]
    raw: bool,

    #[argh(subcommand)]
    comm: Option<Command>,
}

fn read_path(path: &str) -> Result<PathBuf, String> {
    Ok(Path::new(path).into())
}
#[derive(FromArgs)]
#[argh(subcommand)]
enum Command {
    Interpret(CommandInterpret),
    Debug(CommandDebug),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "interpret")]
/// Interpret the given program directly
struct CommandInterpret {}

#[derive(FromArgs)]
#[argh(subcommand, name = "debug")]
/// Interpret the given program with the interactive debugger
struct CommandDebug {}

#[inline]
fn print_res(
    res: InterpreterResult<InterpreterState>,
    raw: bool,
) -> InterpreterResult<()> {
    match res {
        Ok(env) => {
            if raw {
                env.print_env_raw()
            } else {
                env.print_env()
            };
            Ok(())
        }
        Err(InterpreterError::Exit) => Ok(()), // The exit command doesn't cause an error code
        Err(e) => Err(e),
    }
}

/// Interpret a group from a Calyx program
fn main() -> InterpreterResult<()> {
    let opts: Opts = argh::from_env();

    let builder = configuration::ConfigBuilder::new();

    let config = builder
        .quiet(opts.quiet)
        .allow_invalid_memory_access(opts.allow_invalid_memory_access)
        .error_on_overflow(opts.error_on_overflow)
        .allow_par_conflicts(opts.allow_par_conflicts)
        .build();

    interp::logging::initialze_logger(config.quiet);

    let log = interp::logging::root();

    if config.allow_par_conflicts {
        warn!(log, "You have enabled Par conflicts. This is not recommended and is usually a bad idea")
    }

    // Construct IR
    let ws = frontend::Workspace::construct(&opts.file, &opts.lib_path)?;
    let mut ctx = ir::from_ast::ast_to_ir(ws)?;
    let pm = PassManager::default_passes()?;

    if !opts.skip_verification {
        pm.execute_plan(&mut ctx, &["validate".to_string()], &[])?;
    }

    let entry_point = ctx.entrypoint;

    let metadata = ctx.metadata;

    let components: iir::ComponentCtx = Rc::new(
        ctx.components
            .into_iter()
            .map(|x| Rc::new(x.into()))
            .collect(),
    );

    let main_component = components
        .iter()
        .find(|&cm| cm.name == entry_point)
        .ok_or(InterpreterError::MissingMainComponent)?;

    let mems = interp::MemoryMap::inflate_map(&opts.data_file)?;

    let env = InterpreterState::init_top_level(
        &components,
        main_component,
        &mems,
        &config,
    )?;
    let res = match opts.comm.unwrap_or(Command::Interpret(CommandInterpret {}))
    {
        Command::Interpret(_) => {
            ComponentInterpreter::interpret_program(env, main_component)
        }
        Command::Debug(CommandDebug {}) => {
            let map = metadata.map(SourceMap::from_string);
            let map = if let Some(map_res) = map {
                Some(map_res?)
            } else {
                None
            };
            let mut cidb = Debugger::new(&components, main_component, map);
            cidb.main_loop(env)
        }
    };

    print_res(res, opts.raw)
}
