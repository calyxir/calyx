use calyx::{
    errors::{CalyxResult, Error},
    frontend, ir,
    pass_manager::PassManager,
    utils::OutputFile,
};

use argh::FromArgs;
use interp::environment;
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
}

fn read_path(path: &str) -> Result<PathBuf, String> {
    Ok(Path::new(path).into())
}

//first half of this is tests
/// Interpret a group from a Calyx program
fn main() -> CalyxResult<()> {
    let opts: Opts = argh::from_env();

    // Construct IR
    let namespace = frontend::NamespaceDef::new(&opts.file, &opts.lib_path)?;
    let ir = ir::from_ast::ast_to_ir(namespace, false, false)?;

    let ctx = ir::RRC::new(RefCell::new(ir));

    let pm = PassManager::default_passes()?;

    pm.execute_plan(&mut ctx.borrow_mut(), &["validate".to_string()], &[])?;

    let mems = interp::MemoryMap::inflate_map(&opts.data_file)?;

    let env = environment::InterpreterState::init(&ctx, &mems);

    // Get main component; assuming that opts.component is main
    // TODO: handle when component, group are not default values

    let ctx_ref: &ir::Context = &ctx.borrow();
    let main_component = ctx_ref
        .components
        .iter()
        .find(|&cm| cm.name == "main")
        .ok_or_else(|| {
            Error::Impossible("Cannot find main component".to_string())
        })?;

    match interpret_component(main_component, env) {
        Ok(e) => {
            e.print_env();
            Ok(())
        }
        Err(err) => CalyxResult::Err(err),
    }
}
