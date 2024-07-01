use super::Testbench;

pub struct Verilator;

impl Testbench for Verilator {
    fn run(
        &self,
        _input: String,
        _tests: &[String],
        _work_dir: tempdir::TempDir,
    ) -> super::TestbenchResult {
        todo!()
    }
}
