use crate::utils::{add_suffix, ignore};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use structopt::StructOpt;

/// Definition of the command line interface. Uses the `structopt` derive macro
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

    /// Name of the top-level component. Used by visulization to find the top-level module.
    #[structopt(short, long, default_value = "main")]
    pub component: String,

    /// Generate a PNG or DOT file to visualize the structure.
    #[structopt(short = "s", long = "show-struct")]
    pub visualize_structure: Option<Option<PathBuf>>,

    /// Generate a PNG or DOT representation of the FSM.
    #[structopt(long = "show-fsm")]
    pub visualize_fsm: Option<Option<PathBuf>>,

    /// Path to the primitives library.
    #[structopt(long, short, required = true)]
    pub libraries: Vec<PathBuf>,

    #[structopt(long = "futil-output")]
    pub futil_output: Option<Option<PathBuf>>,

    /// Output location for generated Verilog.
    #[structopt(short)]
    pub output: Option<PathBuf>,

    /// Enable debug mode output.
    #[structopt(short = "d", long = "debug")]
    pub enable_debug: bool,
}

// ================== Helper Functions ======================= //

/// struct that implements `std::fmt::Write` to support rewriting
/// to either stdout or a file
pub struct Writer {
    file: Option<File>,
}

impl std::fmt::Write for Writer {
    fn write_str(&mut self, msg: &str) -> Result<(), std::fmt::Error> {
        match &mut self.file {
            None => print!("{}", msg),
            Some(f) => write!(f, "{}", msg).expect("Writing failed"),
        }
        Ok(())
    }
}

/// Function that helps deal with optional paths that can be provided
/// on the command line. Takes in a optional path, an optional `ext` to
/// replace the path extension with, and a function that does the writing
#[allow(unused)]
pub fn path_write<F>(
    path: &Option<PathBuf>,
    suffix: Option<&str>,
    ext: Option<&str>,
    write_fn: &mut F,
) where
    F: FnMut(&mut Writer) -> Result<(), std::fmt::Error>,
{
    let r = match path {
        None => write_fn(&mut Writer { file: None }),
        Some(p) => {
            let mut path = p.clone();
            suffix.map_or((), |x| add_suffix(&mut path, x));
            ext.map_or((), |ext| ignore(path.set_extension(ext)));
            write_fn(&mut Writer {
                file: Some(
                    File::create(path.as_path()).expect("File creation failed"),
                ),
            })
        }
    };
    // panic if writing fails
    r.expect("path_write failed")
}
