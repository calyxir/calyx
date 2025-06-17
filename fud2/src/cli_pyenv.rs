use std::{fs, process::Command};

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
        // This is a bit of a hack to detect whether or not uv is installed
        if Command::new("uv").arg("--version").output().is_err() {
            anyhow::bail!(
                "fud2 env requires `uv` to be installed.\n       Installation instructions can be found at: https://docs.astral.sh/uv/getting-started/installation/"
            )
        }

        let data_dir = config::data_dir(&driver.name);
        fs::create_dir_all(&data_dir)?;

        let pyenv = data_dir.join("venv");

        // create new venv
        Command::new("uv")
            .args(["venv"])
            .arg(&pyenv)
            .stdout(std::io::stdout())
            .stderr(std::io::stderr())
            .output()?;

        // grab the location of the calyx base install
        let config = config::load_config(&driver.name);
        let calyx_base: String = config.extract_inner("calyx.base")?;

        Command::new("uv")
            .args([
                "sync",
                "--all-extras",
                // needed to use the proper venv since uv REALLY wants to use a
                // local `.venv` directory
                "--active",
            ])
            .stdout(std::io::stdout())
            .stderr(std::io::stderr())
            // ensure this executes in the appropriate venv
            .env("VIRTUAL_ENV", &pyenv)
            // ensure this executes from the calyx dir so that uv sync reads the
            // right pyproject.toml when installing dependencies
            .current_dir(calyx_base)
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
