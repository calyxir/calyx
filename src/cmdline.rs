use crate::backend::{traits::Backend, verilog::gen};
use crate::errors;
use crate::lang::context;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;

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
    #[structopt(short = "b", long = "backend", default_value = "verilog")]
    pub backend: BackendOpt,
}

// ================== Backend Variant and Parsing ===================== //

#[derive(Debug, StructOpt)]
pub enum BackendOpt {
    Verilog,
    None,
}

/// Command line parsing for the Backend enum
impl FromStr for BackendOpt {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "verilog" => Ok(Self::Verilog),
            "none" => Ok(Self::None),
            x => Err(format!("{} is not a valid backend.", x)),
        }
    }
}

impl ToString for BackendOpt {
    fn to_string(&self) -> String {
        match self {
            Self::Verilog => "verilog",
            Self::None => "none",
        }
        .to_string()
    }
}

impl BackendOpt {
    pub fn run(&self, context: &context::Context) -> Result<(), errors::Error> {
        match self {
            BackendOpt::Verilog => gen::VerilogBackend::run(&context),
            BackendOpt::None => Ok(()),
        }
    }
}
