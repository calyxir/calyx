use crate::utils::{add_suffix, ignore};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS")
)]
pub struct Opts {
    // name of path where futil program lives
    #[structopt(required = true, parse(from_os_str))]
    pub file: PathBuf,

    // name of the component in <file>
    #[structopt(short, long, default_value = "main")]
    pub component: String,

    // optional argument that optionally takes a value
    #[structopt(short = "s", long = "show-struct")]
    pub visualize_structure: Option<Option<PathBuf>>,

    // optional argument that optionally takes a path to output png
    #[structopt(long = "show-fsm")]
    pub visualize_fsm: Option<Option<PathBuf>>,

    // library paths
    #[structopt(long, short)]
    pub libraries: Option<Vec<PathBuf>>,

    #[structopt(long = "fout")]
    pub futil_output: Option<Option<PathBuf>>,

    // where to output verilog
    #[structopt(short)]
    pub output: Option<PathBuf>,
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
            Some(f) => write!(f, "{}", msg).unwrap(),
        }
        Ok(())
    }
}

/// Function that helps deal with optional paths that can be provided
/// on the command line. Takes in a optional path, an optional `ext` to
/// replace the path extension with, and a function that does the writing
pub fn path_write<F>(
    path: &Option<PathBuf>,
    suffix: Option<&str>,
    ext: Option<&str>,
    write_fn: &mut F,
) where
    F: FnMut(&mut Writer) -> Result<(), std::fmt::Error>,
{
    match path {
        None => write_fn(&mut Writer { file: None }).unwrap(),
        Some(p) => {
            let mut path = p.clone();
            suffix.map_or((), |x| add_suffix(&mut path, x));
            ext.map_or((), |ext| ignore(path.set_extension(ext)));
            write_fn(&mut Writer {
                file: Some(File::create(path.as_path()).unwrap()),
            })
        }
        .unwrap(),
    }
}
