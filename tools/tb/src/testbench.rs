use crate::{config::Config, error::LocalResult};
use tempdir::TempDir;

pub mod cocotb;
pub mod verilator;

pub trait Testbench {
    fn setup(&self, config: &mut Config) -> LocalResult<()>;

    /// - `input` is a relative path to the input file in `work_dir`.
    /// - `tests` are a relative paths to the testing harnesses in `work_dir`.
    fn run(
        &self,
        input: String,
        tests: &[String],
        work_dir: TempDir,
        config: &Config,
    ) -> LocalResult<()>;
}

pub type TestbenchRef = Box<dyn Testbench>;
