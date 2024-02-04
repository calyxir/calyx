use super::{OpRef, StateRef};
use camino::Utf8PathBuf;

/// A request to the Driver directing it what to build.
#[derive(Debug)]
pub struct Request {
    /// The input format.
    pub start_state: StateRef,

    /// The output format to produce.
    pub end_state: StateRef,

    /// The filename to read the input from, or None to read from stdin.
    pub start_file: Option<Utf8PathBuf>,

    /// The filename to write the output to, or None to print to stdout.
    pub end_file: Option<Utf8PathBuf>,

    /// A sequence of operators to route the conversion through.
    pub through: Vec<OpRef>,

    /// The working directory for the build.
    pub workdir: Utf8PathBuf,
}
