use super::Testbench;

pub struct Verilator;

impl Testbench for Verilator {
    fn run(
        &self,
        input: String,
        tests: &[String],
        work_dir: tempdir::TempDir,
    ) -> super::TestbenchResult {
        todo!()
    }
}
