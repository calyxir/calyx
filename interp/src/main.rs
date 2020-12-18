use calyx::{
    errors::{Error, FutilResult},
    frontend::{library, parser},
    ir,
    utils::OutputFile,
};
use std::path::PathBuf;
pub use structopt::StructOpt;

mod interpreter;
pub mod interpretgroup;

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
    let interpreter: interpretgroup::GroupInterpreter =
        interpretgroup::GroupInterpreter {
            component: opts.component.clone(),
            group: opts.group.clone(),
        };

    // Get input file
    let namespace = match &opts.file {
        Some(file) => parser::FutilParser::parse_file(&file),
        None => Err(Error::InvalidFile(
            "Must provide a FuTIL file as input (for now)!".to_string(),
        )),
    }?;

    // Get libraries used in file
    // The only library test programs should have for now is primitives/std.lib
    let library: Vec<_> = namespace
        .libraries
        .iter()
        .map(|path| {
            library::parser::LibraryParser::parse_file(
                &opts.lib_path.join(path),
            )
        })
        .collect::<FutilResult<Vec<_>>>()?;

    // Construct IR
    let ir = ir::from_ast::ast_to_ir(
        namespace.components,
        &library,
        namespace.libraries,
        false,
        false,
    )?;

    // Run the interpreter (in this case, group interpreter)
    interpreter.interpret(ir)?;

    Ok(())
}
