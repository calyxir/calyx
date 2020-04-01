use crate::backend::traits::Backend;
use crate::backend::verilog::gen::VerilogBackend;
use crate::errors;
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
    #[structopt(required = true, parse(from_os_str))]
    pub file: PathBuf,

    /// Path to the primitives library.
    #[structopt(long, short, required = true)]
    pub libraries: Vec<PathBuf>,

    /// Enable debug mode output.
    #[structopt(short = "d", long = "debug")]
    pub enable_debug: bool,

    /// Select a backend.
    #[structopt(short = "b", long = "backend", default_value)]
    pub backend: BackendOpt,
}

// ================== Backend Variant and Parsing ===================== //

/// Enumeration of valid backends
#[derive(Debug, Copy, Clone)]
pub enum BackendOpt {
    Verilog,
    Futil,
    None,
}

fn backends() -> Vec<(&'static str, BackendOpt)> {
    vec![
        (VerilogBackend::name(), BackendOpt::Verilog),
        ("futil", BackendOpt::Futil),
        ("none", BackendOpt::None),
    ]
}

impl Default for BackendOpt {
    fn default() -> Self {
        BackendOpt::Futil
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
            BackendOpt::Futil => {
                context.pretty_print();
                Ok(())
            }
            BackendOpt::None => Ok(()),
        }
    }
}
