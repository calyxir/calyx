//! Command line parsing for the Calyx compiler.
use argh::FromArgs;
#[cfg(feature = "serialize")]
use calyx_backend::SexpBackend;
#[cfg(feature = "yxi")]
use calyx_backend::YxiBackend;
use calyx_backend::{
    xilinx::{XilinxInterfaceBackend, XilinxXmlBackend},
    Backend, BackendOpt, FirrtlBackend, MlirBackend, PrimitiveUsesBackend,
    ResourcesBackend, VerilogBackend,
};
use calyx_ir as ir;
use calyx_utils::{CalyxResult, Error, OutputFile};
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

/// help information about passes
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "pass-help")]
pub struct Help {
    /// alias or pass name to get help for
    #[argh(positional)]
    pub name: Option<String>,
}

/// supported subcommands
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum Subcommand {
    /// Help mode
    Help(Help),
}

#[derive(FromArgs)]
/// Options passed to the Calyx compiler.
pub struct Opts {
    #[argh(subcommand)]
    pub sub: Option<Subcommand>,

    /// input calyx program
    #[argh(positional, from_str_fn(read_path))]
    pub file: Option<PathBuf>,

    /// output file
    #[argh(option, short = 'o', default = "OutputFile::Stdout")]
    pub output: OutputFile,

    /// path to the primitives library
    #[argh(
        option,
        short = 'l',
        default = "Path::new(option_env!(\"CALYX_PRIMITIVES_DIR\").unwrap_or(\".\")).into()"
    )]
    pub lib_path: PathBuf,

    /// compilation mode
    #[argh(option, short = 'm', default = "CompileMode::default()")]
    pub compile_mode: CompileMode,

    /// enable synthesis mode
    #[argh(switch, long = "synthesis")]
    pub enable_synthesis: bool,

    /// disable verification checks emitted by backends
    #[argh(switch)]
    pub disable_verify: bool,

    /// emit nested assignments (only relevant to the Verilog backend)
    #[argh(switch, long = "nested")]
    pub nested_assign: bool,

    /// emit extmodules to use with SystemVerilog implementations
    /// of primitives (only relevant to the FIRRTL backend)
    #[argh(switch, long = "emit-primitive-extmodules")]
    pub emit_primitive_extmodules: bool,

    /// select a backend
    #[argh(option, short = 'b', default = "BackendOpt::default()")]
    pub backend: BackendOpt,

    /// run this pass during execution
    #[argh(option, short = 'p')]
    pub pass: Vec<String>,

    /// disable pass during execution
    #[argh(option, short = 'd', long = "disable-pass")]
    pub disable_pass: Vec<String>,

    /// extra options passed to the context
    #[argh(option, short = 'x', long = "extra-opt")]
    pub extra_opts: Vec<String>,

    /// enable verbose printing
    #[argh(option, long = "log", default = "log::LevelFilter::Warn")]
    pub log_level: log::LevelFilter,

    #[argh(switch, long = "dump-ir")]
    /// print out the IR after every pass
    pub dump_ir: bool,

    #[argh(switch, long = "version")]
    /// print out the version information
    pub version: bool,
}

fn read_path(path: &str) -> Result<PathBuf, String> {
    Ok(Path::new(path).into())
}

// Compilation modes
#[derive(Default, PartialEq, Eq)]
pub enum CompileMode {
    /// Compile the input file and ignore the dependencies.
    File,
    #[default]
    /// Transitively compile all dependencies `import`ed by the input file.
    Project,
}

impl FromStr for CompileMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "file" => Ok(CompileMode::File),
            "project" => Ok(CompileMode::Project),
            s => Err(format!("Unknown compilation mode: {}. Valid options are `file` or `project`", s))
        }
    }
}

// ================== Backend Variant and Parsing ===================== //

impl Opts {
    /// Given a context, calls the backend corresponding to the `BackendOpt` variant
    pub fn run_backend(self, context: ir::Context) -> CalyxResult<()> {
        match self.backend {
            BackendOpt::Mlir => {
                let backend = MlirBackend;
                backend.run(context, self.output)
            }
            BackendOpt::Resources => {
                let backend = ResourcesBackend;
                backend.run(context, self.output)
            }
            BackendOpt::Sexp => {
                #[cfg(feature = "serialize")]
                {
                    let backend = SexpBackend;
                    backend.run(context, self.output)
                }
                #[cfg(not(feature = "serialize"))]
                {
                    Err(Error::misc(
                        "Sexp backend requires the `serialize` feature to be enabled",
                    ))
                }
            }
            BackendOpt::Verilog => {
                let backend = VerilogBackend;
                backend.run(context, self.output)
            }
            BackendOpt::Xilinx => {
                let backend = XilinxInterfaceBackend;
                backend.run(context, self.output)
            }
            BackendOpt::XilinxXml => {
                let backend = XilinxXmlBackend;
                backend.run(context, self.output)
            }
            #[cfg(feature = "yxi")]
            BackendOpt::Yxi => {
                let backend = YxiBackend;
                backend.run(context, self.output)
            }
            BackendOpt::Firrtl => {
                let backend = FirrtlBackend;
                backend.run(context, self.output)
            }
            BackendOpt::PrimitiveUses => {
                let backend = PrimitiveUsesBackend;
                backend.run(context, self.output)
            }
            BackendOpt::Calyx => {
                ir::Printer::write_context(
                    &context,
                    false,
                    &mut self.output.get_write(),
                )?;
                Ok(())
            }
            BackendOpt::None => Ok(()),
        }
    }

    /// Get the current set of options from the command line invocation.
    pub fn get_opts() -> CalyxResult<Opts> {
        let mut opts: Opts = argh::from_env();

        if opts.compile_mode == CompileMode::File
            && !matches!(opts.backend, BackendOpt::Calyx | BackendOpt::None)
        {
            return Err(Error::misc(format!(
                "--compile-mode=file is only valid with -b calyx. `-b {}` requires --compile-mode=project",
                opts.backend.to_string()
            )));
        }

        // argh doesn't allow us to specify a default for this so we fill it
        // in manually.
        if opts.pass.is_empty() {
            opts.pass = vec!["all".into()];
        }

        Ok(opts)
    }
}
