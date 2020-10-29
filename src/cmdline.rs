use calyx::backend::traits::Backend;
use calyx::backend::verilog::VerilogBackend;
use calyx::{errors::FutilResult, ir, utils::OutputFile};
use itertools::Itertools;
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
    /// Input futil program
    #[structopt(parse(from_os_str))]
    pub file: Option<PathBuf>,

    /// Output file
    #[structopt(long = "output", short = "o", default_value)]
    pub output: OutputFile,

    #[structopt(long = "force-color")]
    pub color: bool,

    /// Path to the primitives library
    #[structopt(long, short, default_value = ".")]
    pub lib_path: PathBuf,

    /// Enable debug mode output
    #[structopt(long = "debug")]
    pub enable_debug: bool,

    /// Enable Verilator mode.
    #[structopt(long = "verilator")]
    pub enable_verilator: bool,

    /// Select a backend.
    #[structopt(short = "b", long = "backend", default_value)]
    pub backend: BackendOpt,

    /// Toplevel component
    #[structopt(short = "t", long = "toplevel", default_value = "main")]
    pub toplevel: String,

    /// Run this pass during execution
    #[structopt(short = "p", long = "pass", default_value = "all")]
    pub pass: Vec<String>,

    /// Disable pass during execution
    #[structopt(short = "d", long = "disable-pass")]
    pub disable_pass: Vec<String>,

    /// list all avaliable pass options
    #[structopt(long = "list-passes")]
    pub list_passes: bool,
}

// ================== Backend Variant and Parsing ===================== //

/// Enumeration of valid backends
#[derive(Debug, Copy, Clone)]
pub enum BackendOpt {
    Verilog,
    Futil,
    // Dot,
    None,
}

fn backends() -> Vec<(&'static str, BackendOpt)> {
    vec![
        (VerilogBackend::name(), BackendOpt::Verilog),
        ("futil", BackendOpt::Futil),
        // ("dot", BackendOpt::Dot),
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
            // Self::Dot => "dot",
            Self::None => "none",
        }
        .to_string()
    }
}

impl Opts {
    /// Given a context, calls the backend corresponding to the `BackendOpt` variant
    pub fn run_backend(self, context: &ir::Context) -> FutilResult<()> {
        match self.backend {
            BackendOpt::Verilog => VerilogBackend::run(&context, self.output),
            BackendOpt::Futil => {
                for import_path in &context.import_statements {
                    writeln!(
                        &mut self.output.get_write(),
                        "import \"{}\";",
                        import_path
                    )?
                }
                for comp in &context.components {
                    ir::IRPrinter::write_component(
                        comp,
                        &mut self.output.get_write(),
                    )?;
                }
                Ok(())
            }
            // BackendOpt::Dot => {
            //     let write_result = write!(
            //         self.output.get_write(),
            //         "{}",
            //         context
            //             .get_component(&self.toplevel.into())?
            //             .structure
            //             .visualize()
            //     );
            //     write_result.map_err(|err| {
            //         Error::InvalidFile(format!(
            //             "Failed to write: {}",
            //             err.to_string()
            //         ))
            //     })?;
            //     Ok(())
            // }
            BackendOpt::None => Ok(()),
        }
    }
}
