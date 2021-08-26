use crate::environment::InterpreterState;
use argh::FromArgs;
use calyx::{frontend, ir, pass_manager::PassManager, utils::OutputFile};
use interp::debugger::Debugger;
use interp::environment;
use interp::errors::{InterpreterError, InterpreterResult};
use interp::interpreter::interpret_component;
use std::path::PathBuf;
use std::{cell::RefCell, path::Path};

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

//first half of this is tests
/// Interpret a group from a Calyx program
fn main() -> InterpreterResult<()> {
    let opts: Opts = argh::from_env();

    // Construct IR
    let namespace = frontend::NamespaceDef::new(&opts.file, &opts.lib_path)?;
    let ir = ir::from_ast::ast_to_ir(namespace, false, false)?;
    let ctx = ir::RRC::new(RefCell::new(ir));
    let pm = PassManager::default_passes()?;

    pm.execute_plan(&mut ctx.borrow_mut(), &["validate".to_string()], &[])?;

    let ctx_ref: &ir::Context = &ctx.borrow();
    let main_component = ctx_ref
        .components
        .iter()
        .find(|&cm| cm.name == "main")
        .ok_or(InterpreterError::MissingMainComponent)?;

    let mems = interp::MemoryMap::inflate_map(&opts.data_file)?;
    let env = environment::InterpreterState::init(&ctx, main_component, &mems);
    let res = match opts.comm.unwrap_or(Command::Interpret(CommandInterpret {}))
    {
        Command::Interpret(_) => interpret_component(main_component, env),
        Command::Debug(CommandDebug { pass_through }) => {
            let mut cidb = Debugger::new(ctx_ref, main_component);
            cidb.main_loop(env, pass_through)
        }
    };

    print_res(res)
}
