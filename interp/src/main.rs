use calyx::{frontend, ir, pass_manager::PassManager, utils::OutputFile};
use interp::{
    debugger::Debugger,
    environment::InterpreterState,
    errors::{InterpreterError, InterpreterResult},
    interpreter::interpret_component,
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
struct CommandDebug {
    #[argh(switch, short = 'p', long = "pass-through")]
    /// flag which runs the program to completion through the debugger
    pass_through: bool,
}

#[inline]
fn print_res(
    res: InterpreterResult<InterpreterState>,
) -> InterpreterResult<()> {
    match res {
        Ok(env) => {
            env.print_env();
            Ok(())
        }
        Err(InterpreterError::Exit) => Ok(()), // The exit command doesn't cause an error code
        Err(e) => Err(e),
    }
}

/// Interpret a group from a Calyx program
fn main() -> InterpreterResult<()> {
    let opts: Opts = argh::from_env();

    {
        // get read access to the settings
        let mut write_lock = interp::SETTINGS.write().unwrap();
        if opts.quiet {
            write_lock.quiet = true;
        }
        if opts.allow_invalid_memory_access {
            write_lock.allow_invalid_memory_access = true;
        }
        if opts.error_on_overflow {
            write_lock.error_on_overflow = true;
        }
        if opts.allow_par_conflicts {
            write_lock.allow_par_conflicts = true;
        }
        // release lock
    }

    let log = &interp::logging::ROOT_LOGGER;

    if interp::SETTINGS.read().unwrap().allow_par_conflicts {
        warn!(log, "You have enabled Par conflicts. This is not recommended and is usually a bad idea")
    }

    // Construct IR
    let ws = frontend::Workspace::construct(&opts.file, &opts.lib_path)?;
    let mut ctx = ir::from_ast::ast_to_ir(ws, ir::BackendConf::default())?;
    let pm = PassManager::default_passes()?;

    if !opts.skip_verification {
        pm.execute_plan(&mut ctx, &["validate".to_string()], &[])?;
    }

    let entry_point = ctx.entrypoint;

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

    let env =
        InterpreterState::init_top_level(&components, main_component, &mems);
    let res = match opts.comm.unwrap_or(Command::Interpret(CommandInterpret {}))
    {
        Command::Interpret(_) => interpret_component(main_component, env?),
        Command::Debug(CommandDebug { pass_through }) => {
            let mut cidb = Debugger::new(&components, main_component);
            cidb.main_loop(env?, pass_through)
        }
    };

    print_res(res)
}
