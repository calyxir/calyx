use tempdir::TempDir;

use crate::{
    config::Config,
    error::{LocalError, LocalResult},
    testbench::{cocotb, verilator, TestbenchRef},
};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Default)]
pub struct Driver {
    tbs: HashMap<String, TestbenchRef>,
}

impl Driver {
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

    pub fn run<S: AsRef<str>, P: AsRef<Path>>(
        &self,
        name: S,
        path: P,
        input: String,
        tests: &[String],
    ) -> LocalResult<()> {
        if let Some(tb) = self.tbs.get(name.as_ref()) {
            let work_dir =
                TempDir::new(".calyx-tb").map_err(LocalError::from)?;
            let mut config = Config::from(path, name)?;
            let input =
                copy_into(input, &work_dir).map_err(LocalError::from)?;
            let mut test_basenames = vec![];
            for test in tests {
                test_basenames.push(
                    copy_into(test, &work_dir).map_err(LocalError::from)?,
                );
            }
            tb.setup(&mut config)?;
            config.doctor()?;
            tb.run(input, &test_basenames, work_dir, &config)
        } else {
            Err(LocalError::Other(format!(
                "Unknown testbench '{}'",
                name.as_ref()
            )))
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
