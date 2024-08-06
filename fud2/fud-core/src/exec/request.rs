use super::{plan::FindPlan, OpRef, StateRef};
use camino::Utf8PathBuf;

/// A request to the Driver directing it what to build.
#[derive(Debug)]
pub struct Request {
    /// The input format.
    /// Invariant: start_states.len() >= start_files.len()
    pub start_states: Vec<StateRef>,

    /// The output format to produce.
    /// Invariant: end_states.len() >= end_files.len()
    pub end_states: Vec<StateRef>,

    /// The filename to read the input from, or None to read from stdin.
    pub start_files: Vec<Utf8PathBuf>,

    /// The filename to write the output to, or None to print to stdout.
    pub end_files: Vec<Utf8PathBuf>,

    /// A sequence of operators to route the conversion through.
    pub through: Vec<OpRef>,

    /// The working directory for the build.
    pub workdir: Utf8PathBuf,

    /// The algorithm used to find a plan to turn start states into end states
    pub planner: Box<dyn FindPlan>,
}
