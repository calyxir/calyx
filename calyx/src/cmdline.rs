use crate::backend::interpreter::eval::Interpreter;
use crate::backend::traits::Backend;
use crate::backend::verilog::gen::VerilogBackend;
use crate::errors;
use crate::lang::ast;
use crate::lang::context;
use crate::lang::pretty_print::PrettyPrint;
use itertools::Itertools;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
pub use structopt::StructOpt;

// Definition of the command line interface. Uses the `structopt` derive macro
#[derive(StructOpt, Debug)]
#[structopt(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS")
)]
#[allow(clippy::option_option)]
pub struct Opts {
    /// Input futil program.
    #[structopt(required_unless = "list-passes", parse(from_os_str))]
    pub file: Option<PathBuf>,

    /// Path to the primitives library.
    #[structopt(long, short, required_unless = "list-passes")]
    pub libraries: Vec<PathBuf>,

    /// Enable debug mode output.
    #[structopt(short = "d", long = "debug")]
    pub enable_debug: bool,

    /// Select a backend.
    #[structopt(short = "b", long = "backend", default_value)]
    pub backend: BackendOpt,

    /// Choose a single pass
    #[structopt(short = "p", long = "pass", default_value = "all")]
    pub pass: Vec<String>,

    ///list all avaliable pass options
    #[structopt(long = "list-passes")]
    pub list_passes: bool,

    /// Specifies an inputs file for simulation
    #[structopt(
        short = "i",
        long = "component inputs",
        required_if("backend", "interpreter")
    )]
    pub inputs: Option<PathBuf>,
}

// ================== Backend Variant and Parsing ===================== //

/// Enumeration of valid backends
#[derive(Debug, Copy, Clone)]
pub enum BackendOpt {
    Verilog,
    Futil,
    Interpreter,
    None,
}

fn backends() -> Vec<(&'static str, BackendOpt)> {
    vec![
        (VerilogBackend::name(), BackendOpt::Verilog),
        (Interpreter::name(), BackendOpt::Interpreter),
        ("futil", BackendOpt::Futil),
        ("none", BackendOpt::None),
    ]
}

impl Default for BackendOpt {
    fn default() -> Self {
        BackendOpt::Futil
    }
}

#[derive(Debug, Clone)]
pub struct InterpreterArgs {
    pub component: ast::Id,
    pub input_file: PathBuf,
}

/// Command line parsing for interpreter arguments
impl FromStr for InterpreterArgs {
    type Err = String;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut iter = input.trim().split_whitespace();
        if let Some(comp_name) = iter.next() {
            if let Some(input_file) = iter.next() {
                return Ok(InterpreterArgs {
                    component: ast::Id::from(comp_name),
                    input_file: PathBuf::from(input_file),
                });
            }
        }
        Err(format!(
                "Incorrect usage: --interpreter flag expects the following arguments: <component_name> <input_file>",
            ))
    }
}

/// Command line parsing for the Backend enum
impl FromStr for BackendOpt {
    type Err = String;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        // allocate a vector for the list of backends
        let backends = backends();
        // see if there is a backend for the string that we receive
        let found_backend = backends
            .iter()
            .find(|(backend_name, _)| &input == backend_name);
        if let Some((_, opt)) = found_backend {
            // return the BackendOpt if we found one
            Ok(*opt)
        } else {
            // build list of backends for error message
            let backend_str = backends
                .iter()
                .map(|(name, _)| (*name).to_string())
                .join(", ");
            Err(format!(
                "`{}` is not a valid backend.\nValid backends: {}",
                input, backend_str
            ))
        }
    }
}

/// Convert `BackendOpt` to a string
impl ToString for BackendOpt {
    fn to_string(&self) -> String {
        match self {
            Self::Verilog => "verilog",
            Self::Interpreter => "interpreter",
            Self::Futil => "futil",
            Self::None => "none",
        }
        .to_string()
    }
}

impl BackendOpt {
    /// Given a context, calls the backend corresponding to the `BackendOpt` variant
    pub fn run<W: Write>(
        self,
        context: &context::Context,
        file: W,
    ) -> Result<(), errors::Error> {
        match self {
            BackendOpt::Verilog => VerilogBackend::run(&context, file),
            BackendOpt::Interpreter => Interpreter::emit(&context, file),
            BackendOpt::Futil => {
                context.pretty_print();
                Ok(())
            }
            BackendOpt::None => Ok(()),
        }
    }
}
