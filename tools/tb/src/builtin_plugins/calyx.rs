use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

use crate::{
    config::Config, error::LocalResult, plugin::Plugin, semver, tempdir,
};

#[derive(Default)]
pub struct CalyxTB;

mod config_keys {}

const DRIVER_CODE: &str = include_str!("resources/driver.rs");

impl Plugin for CalyxTB {
    fn name(&self) -> &'static str {
        "calyx"
    }

    fn version(&self) -> semver::Version {
        semver::Version::new(0, 0, 0)
    }

    fn setup(&self, _config: &mut Config) -> LocalResult<()> {
        Ok(())
    }

    fn run(
        &self,
        input: String,
        tests: &[String],
        work_dir: tempdir::TempDir,
        _config: &Config,
    ) -> LocalResult<()> {
        eprintln!(
            "recommendation: Run the #[calyx_ffi_tests] as Rust tests directly"
        );

        eprintln!("tb: --using {}: setting up dummy crate", self.name());

        let mut dut_path = PathBuf::from(work_dir.path());
        dut_path.push(&input);
        if let Some(parent) = dut_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&input, dut_path)?;

        let mut calyx_ffi_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        calyx_ffi_path.push("../calyx-ffi");

        let mut main_file = PathBuf::from(work_dir.path());
        main_file.push("main.rs");
        fs::write(main_file, DRIVER_CODE)?;

        let mut manifest_path = PathBuf::from(work_dir.path());
        manifest_path.push("Cargo.toml");

        let mut lib_path = PathBuf::from(work_dir.path());
        lib_path.push("lib.rs");

        let mut manifest = toml::Table::new();
        manifest.insert(
            "package".into(),
            toml::Value::Table(toml::map::Map::from_iter([
                ("name".to_string(), "test_crate".into()),
                ("edition".to_string(), "2021".into()),
            ])),
        );
        manifest.insert(
            "lib".into(),
            toml::Value::Table(toml::map::Map::from_iter([(
                "path".to_string(),
                "lib.rs".into(),
            )])),
        );
        manifest.insert(
            "bin".into(),
            vec![toml::Value::Table(toml::map::Map::from_iter([
                ("name".to_string(), "test".into()),
                ("path".to_string(), "main.rs".into()),
            ]))]
            .into(),
        );
        manifest.insert(
            "dependencies".into(),
            toml::Value::Table(toml::map::Map::from_iter([(
                "calyx-ffi".to_string(),
                toml::Value::Table(toml::map::Map::from_iter([(
                    "path".to_string(),
                    calyx_ffi_path.to_string_lossy().to_string().into(),
                )])),
            )])),
        );

        for test in tests {
            fs::write(&manifest_path, manifest.to_string())?;
            fs::copy(test, &lib_path)?;

            eprintln!(
                "tb: --using {}: building and testing `{}` with `{}`",
                self.name(),
                input,
                test
            );
            eprintln!("  (may take a while because `rustc` is slow)");

            let output = Command::new("cargo")
                .arg("expand")
                .arg("--lib")
                .current_dir(work_dir.path())
                .output()?;

            println!("{}", unsafe {
                String::from_utf8_unchecked(output.stdout)
            });
            let output = Command::new("cargo")
                .arg("run")
                .arg("--quiet")
                .current_dir(work_dir.path())
                .output()?;
            io::stderr().write_all(&output.stderr)?;
            io::stdout().write_all(&output.stdout)?;
        }

        Ok(())
    }
}
