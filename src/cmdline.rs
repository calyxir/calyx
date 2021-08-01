use argh::FromArgs;
use calyx::backend::traits::Backend;
use calyx::backend::{
    verilog::VerilogBackend, xilinx::XilinxInterfaceBackend,
    xilinx::XilinxXmlBackend,
};
use calyx::{errors::CalyxResult, ir, utils::OutputFile};
use itertools::Itertools;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(FromArgs, Debug)]
/// The Calyx compiler
pub struct Opts {
    /// input calyx program
    #[argh(positional, from_str_fn(read_path))]
    pub file: Option<PathBuf>,

    /// output file
    #[argh(option, short = 'o', default = "OutputFile::default()")]
    pub output: OutputFile,

    /// path to the primitives library
    #[argh(option, short = 'l', default = "Path::new(\".\").into()")]
    pub lib_path: PathBuf,

    /// enable debug mode output
    #[argh(switch, long = "debug")]
    pub enable_debug: bool,

    /// enable synthesis mode
    #[argh(switch, long = "synthesis")]
    pub enable_synthesis: bool,

    /// select a backend
    #[argh(option, short = 'b', default = "BackendOpt::default()")]
    pub backend: BackendOpt,

    /// toplevel component
    #[argh(option, short = 't', default = "\"main\".to_string()")]
    pub toplevel: String,

    /// run this pass during execution
    #[argh(option, short = 'p')]
    pub pass: Vec<String>,

    /// disable pass during execution
    #[argh(option, short = 'd', long = "disable-pass")]
    pub disable_pass: Vec<String>,

    /// list all avaliable pass options
    #[argh(switch, long = "list-passes")]
    pub list_passes: bool,
}

fn read_path(path: &str) -> Result<PathBuf, String> {
    Ok(Path::new(path).into())
}

// ================== Backend Variant and Parsing ===================== //

/// Enumeration of valid backends
#[derive(Debug, Copy, Clone)]
pub enum BackendOpt {
    Verilog,
    Xilinx,
    XilinxXml,
    Calyx,
    None,
}

fn backends() -> Vec<(&'static str, BackendOpt)> {
    vec![
        ("verilog", BackendOpt::Verilog),
        ("xilinx", BackendOpt::Xilinx),
        ("xilinx-xml", BackendOpt::XilinxXml),
        ("futil", BackendOpt::Calyx),
        ("calyx", BackendOpt::Calyx),
        ("none", BackendOpt::None),
    ]
}

impl Default for BackendOpt {
    fn default() -> Self {
        BackendOpt::Calyx
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
            Self::Xilinx => "xilinx",
            Self::XilinxXml => "xilinx-xml",
            Self::Calyx => "calyx",
            Self::None => "none",
        }
        .to_string()
    }
}

impl Opts {
    /// Given a context, calls the backend corresponding to the `BackendOpt` variant
    pub fn run_backend(self, context: &ir::Context) -> CalyxResult<()> {
        match self.backend {
            BackendOpt::Verilog => {
                let backend = VerilogBackend::default();
                backend.run(context, self.output)
            }
            BackendOpt::Xilinx => {
                let backend = XilinxInterfaceBackend::default();
                backend.run(context, self.output)
            }
            BackendOpt::XilinxXml => {
                let backend = XilinxXmlBackend::default();
                backend.run(context, self.output)
            }
            BackendOpt::Calyx => {
                for import_path in &context.imports {
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
                    writeln!(&mut self.output.get_write())?
                }
                Ok(())
            }
            BackendOpt::None => Ok(()),
        }
    }
}
