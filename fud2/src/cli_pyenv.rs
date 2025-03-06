use std::{fs, path::Path, process::Command};

use argh::{CommandInfo, FromArgs};
use fud_core::{
    cli::{CliExt, FromArgFn, RedactArgFn},
    config,
};

/// manage a fud2 python environment
#[derive(FromArgs)]
#[argh(subcommand, name = "env")]
pub struct PyenvCommand {
    #[argh(subcommand)]
    sub: PyenvAction,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum PyenvAction {
    Init(InitCommand),
    Activate(ActivateCommand),
}

/// initialize python venv and install necessary packages
#[derive(FromArgs)]
#[argh(subcommand, name = "init")]
pub struct InitCommand {}

/// activate the fud2 python venv for manual management
#[derive(FromArgs)]
#[argh(subcommand, name = "activate")]
pub struct ActivateCommand {}

impl PyenvCommand {
    fn init(&self, driver: &fud_core::Driver) -> anyhow::Result<()> {
        let data_dir = config::data_dir(&driver.name);

        fs::create_dir_all(&data_dir)?;

        let pyenv = data_dir.join("venv");

        // create new venv
        Command::new("python3")
            .args(["-m", "venv"])
            .arg(&pyenv)
            .stdout(std::io::stdout())
            .output()?;

        // install vcdvcd (for Profiler)
        Command::new(pyenv.join("bin").join("pip"))
            .arg("install")
            .arg("vcdvcd")
            .stdout(std::io::stdout())
            .output()?;

        // install flit
        Command::new(pyenv.join("bin").join("pip"))
            .arg("install")
            .arg("flit")
            .stdout(std::io::stdout())
            .output()?;

        // grab the location of the calyx base install
        let config = config::load_config(&driver.name);
        let calyx_base: String = config.extract_inner("calyx.base")?;

        // install fud python library
        Command::new(pyenv.join("bin").join("python"))
            .args(["-m", "flit", "install"])
            .current_dir(Path::new(&calyx_base).join("fud"))
            .stdout(std::io::stdout())
            .output()?;

        // install calyx-py library
        Command::new(pyenv.join("bin").join("python"))
            .args(["-m", "flit", "install"])
            .current_dir(Path::new(&calyx_base).join("calyx-py"))
            .stdout(std::io::stdout())
            .output()?;

        // add python location to fud2.toml
        let config_path = config::config_path(&driver.name);
        let contents = fs::read_to_string(&config_path)?;
        let mut toml_doc: toml_edit::DocumentMut = contents.parse()?;

        toml_doc["python"] = toml_edit::value(
            pyenv
                .join("bin")
                .join("python")
                .to_string_lossy()
                .to_string(),
        );

        fs::write(&config_path, toml_doc.to_string())?;

        Ok(())
    }

    fn activate(&self, driver: &fud_core::Driver) -> anyhow::Result<()> {
        let data_dir = config::data_dir(&driver.name);
        let pyenv = data_dir.join("venv");

        if !pyenv.exists() {
            anyhow::bail!(
                "You need to run `fud2 env init` before you can activate the venv"
            )
        }

        println!("{}", pyenv.join("bin").join("activate").to_str().unwrap());

        Ok(())
    }

    fn run(&self, driver: &fud_core::Driver) -> anyhow::Result<()> {
        match self.sub {
            PyenvAction::Init(_) => self.init(driver),
            PyenvAction::Activate(_) => self.activate(driver),
        }
    }
}

pub enum Fud2CliExt {
    Pyenv(PyenvCommand),
}

impl CliExt for Fud2CliExt {
    fn inner_command_info() -> Vec<CommandInfo> {
        vec![CommandInfo {
            name: "env",
            description: "manage the fud2 python environment",
        }]
    }

    fn inner_redact_arg_values() -> Vec<(&'static str, RedactArgFn)> {
        vec![("env", PyenvCommand::redact_arg_values)]
    }

    fn inner_from_args() -> Vec<(&'static str, FromArgFn<Self>)> {
        vec![("env", |cmd_name, args| {
            PyenvCommand::from_args(cmd_name, args).map(Self::Pyenv)
        })]
    }

    fn run(&self, driver: &fud_core::Driver) -> anyhow::Result<()> {
        match &self {
            Fud2CliExt::Pyenv(cmd) => cmd.run(driver),
        }
    }
}
