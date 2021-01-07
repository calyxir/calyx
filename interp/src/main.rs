use calyx::{errors::FutilResult, frontend, ir, utils::OutputFile};
use interp::interpret_group::GroupInterpreter;
use std::path::PathBuf;
use structopt::StructOpt;

/// CLI Options
#[derive(Debug, StructOpt)]
#[structopt(name = "interpreter", about = "group interpreter CLI")]
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
    #[structopt(short = "g", long = "group")]
    pub group: String,
}

/// Interpret a group from a FuTIL program
fn main() -> FutilResult<()> {
    let opts = Opts::from_args();

    // Construct interpreter
    let interpreter: GroupInterpreter = GroupInterpreter {
        component: opts.component.clone(),
        group: opts.group.clone(),
    };

    // Construct IR
    let namespace = frontend::NamespaceDef::new(&opts.file, &opts.lib_path)?;
    let ir = ir::from_ast::ast_to_ir(namespace, false, false)?;

    // Run the interpreter (in this case, group interpreter)
    interpreter.interpret(ir)?;

    Ok(())
}
