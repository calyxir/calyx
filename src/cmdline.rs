use calyx::backend::traits::Backend;
use calyx::backend::verilog::gen::VerilogBackend;
use calyx::{
    errors::Result, frontend::pretty_print::PrettyPrint, lang::context,
};
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
    #[structopt(parse(from_os_str))]
    pub file: Option<PathBuf>,

    /// Path to the primitives library.
    #[structopt(long, short, default_value = ".")]
    pub lib_path: PathBuf,

    /// Enable debug mode output.
    #[structopt(short = "d", long = "debug")]
    pub enable_debug: bool,

    /// Select a backend.
    #[structopt(short = "b", long = "backend", default_value)]
    pub backend: BackendOpt,

    /// Toplevel component
    #[structopt(short = "t", long = "toplevel", default_value = "main")]
    pub toplevel: String,

    /// choose a single pass
    #[structopt(short = "p", long = "pass", default_value = "all")]
    pub pass: Vec<String>,

    ///list all avaliable pass options
    #[structopt(long = "list-passes")]
    pub list_passes: bool,
}

// ================== Backend Variant and Parsing ===================== //

/// Enumeration of valid backends
#[derive(Debug, Copy, Clone)]
pub enum BackendOpt {
    Verilog,
    Futil,
    Dot,
    None,
}

fn backends() -> Vec<(&'static str, BackendOpt)> {
    vec![
        (VerilogBackend::name(), BackendOpt::Verilog),
        ("futil", BackendOpt::Futil),
        ("dot", BackendOpt::Dot),
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
    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
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
            Self::Dot => "dot",
            Self::None => "none",
        }
        .to_string()
    }
}

impl Opts {
    /// Given a context, calls the backend corresponding to the `BackendOpt` variant
    pub fn run_backend<W: Write>(
        self,
        context: &context::Context,
        file: &mut W,
    ) -> Result<()> {
        match self.backend {
            BackendOpt::Verilog => VerilogBackend::run(&context, file),
            BackendOpt::Futil => {
                context.pretty_print();
                Ok(())
            }
            BackendOpt::Dot => {
                write!(
                    file,
                    "{}",
                    context
                        .get_component(&self.toplevel.into())?
                        .structure
                        .visualize()
                )?;
                Ok(())
            }
            BackendOpt::None => Ok(()),
        }
    }
}
