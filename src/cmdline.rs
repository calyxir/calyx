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

    /// Path to the primitives library.
    #[structopt(long, short, required = true)]
    pub libraries: Vec<PathBuf>,

    /// Enable debug mode output.
    #[structopt(short = "d", long = "debug")]
    pub enable_debug: bool,
}
