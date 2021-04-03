mod environment;
mod interpret_component;
mod interpret_control;
mod interpret_group;
mod interpreter;
mod primitives;
mod update;

use calyx::{
    errors::{Error, FutilResult},
    frontend, ir,
    utils::OutputFile,
};
use std::path::PathBuf;
use structopt::StructOpt;

/// CLI Options
#[derive(Debug, StructOpt)]
#[structopt(name = "interpreter", about = "interpreter CLI")]
pub struct Opts {
    /// Input file
    #[structopt(parse(from_os_str))]
    pub file: Option<PathBuf>,

    /// Output file, default is stdout
    #[structopt(short = "o", long = "output", default_value)]
    pub output: OutputFile,

    /// Path to the primitives library
    #[structopt(long, short, default_value = "..")]
    pub lib_path: PathBuf,

    /// Component to interpret
    #[structopt(short = "c", long = "component", default_value = "main")]
    pub component: String,

    /// Group to interpret
    /// XX(karen): The user can specify a particular group to interpret,
    /// assuming the group is in `main` if not specified otherwise.
    #[structopt(short = "g", long = "group", default_value = "main")]
    pub group: String,
}

/// Interpret a group from a FuTIL program
fn main() -> FutilResult<()> {
    let opts = Opts::from_args();

    // Construct IR
    let namespace = frontend::NamespaceDef::new(&opts.file, &opts.lib_path)?;
    let ir = ir::from_ast::ast_to_ir(namespace, false, false)?;

    // Get main component; assuming that opts.component is main
    // TODO: handle when component, group are not default values
    let mn = ir
        .components
        .into_iter()
        .find(|cm| cm.name == "main".to_string())
        .ok_or(Error::Impossible("Cannot find main component".to_string()))?;

    // TODO: context is moved to environment
    let env = environment::Environment::init(ir);

    let interpreter: interpret_component::ComponentInterpreter =
        interpret_component::ComponentInterpreter {
            environment: env,
            component: mn,
        };
    interpreter.interpret();

    Ok(())
}
