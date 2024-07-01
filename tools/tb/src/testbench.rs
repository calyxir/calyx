use std::{collections::HashMap, fs, path::PathBuf};
use tempdir::TempDir;

pub mod cocotb;
pub mod verilator;

pub type TestbenchResult = std::io::Result<()>;

fn testbench_error_unknown_tb<S: AsRef<str>>(tb: S) -> TestbenchResult {
    TestbenchResult::Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        format!("Unknown testbench '{}'", tb.as_ref()),
    ))
}

pub trait Testbench {
    // TODO: add a config system here
    /// - `input` is a relative path to the input file in `work_dir`.
    /// - `tests` are a relative paths to the testing harnesses in `work_dir`.
    fn run(
        &self,
        input: String,
        tests: &[String],
        work_dir: TempDir,
    ) -> TestbenchResult;
}

pub type TestbenchRef = Box<dyn Testbench>;

#[derive(Default)]
pub struct TestbenchManager {
    tbs: HashMap<String, TestbenchRef>,
}

impl TestbenchManager {
    pub fn new() -> Self {
        let mut new_self = Self::default();
        new_self.register("cocotb", Box::new(cocotb::CocoTB));
        new_self.register("verilator", Box::new(verilator::Verilator));
        new_self
    }

    pub fn register<S: AsRef<str>>(&mut self, name: S, tb: TestbenchRef) {
        assert!(
            self.tbs.insert(name.as_ref().to_string(), tb).is_none(),
            "cannot re-register the same testbench name for a different testbench"
        );
    }

    pub fn run<S: AsRef<str>>(
        &self,
        name: S,
        input: String,
        tests: &[String],
    ) -> TestbenchResult {
        if let Some(tb) = self.tbs.get(name.as_ref()) {
            let work_dir = TempDir::new(".tb")?;
            let input = copy_into(input, &work_dir)?;
            let mut test_basenames = vec![];
            for test in tests {
                test_basenames.push(copy_into(test, &work_dir)?);
            }
            tb.run(input, &test_basenames, work_dir)
        } else {
            testbench_error_unknown_tb(name)
        }
    }
}

fn copy_into<S: AsRef<str>>(
    file: S,
    work_dir: &TempDir,
) -> std::io::Result<String> {
    let from_path = PathBuf::from(file.as_ref());
    let basename = from_path
        .file_name()
        .expect("path ended with ..")
        .to_str()
        .expect("invalid unicode")
        .to_string();
    let mut to_path = work_dir.path().to_path_buf();
    to_path.push(&basename);
    fs::copy(from_path, to_path)?;
    Ok(basename)
}
