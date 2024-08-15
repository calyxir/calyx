use std::{fs, path::Path, process::Command, sync::OnceLock};

use argh::{CommandInfo, DynamicSubCommand, EarlyExit};
use fud_core::{cli::FakeDynamic, config};

#[derive(PartialEq, Debug)]
pub struct PyenvCommand {}

impl DynamicSubCommand for PyenvCommand {
    fn commands() -> &'static [&'static CommandInfo] {
        static RET: OnceLock<Vec<&'static CommandInfo>> = OnceLock::new();
        RET.get_or_init(|| {
            let mut commands = Vec::new();

            let env_cmdinfo = CommandInfo {
                name: "env",
                description: "initialize fud2 python environment",
            };

            commands.push(&*Box::leak(Box::new(env_cmdinfo)));

            commands
        })
    }

    fn try_redact_arg_values(
        command_name: &[&str],
        args: &[&str],
    ) -> Option<Result<Vec<String>, EarlyExit>> {
        for command in Self::commands() {
            if command_name.last() == Some(&command.name) {
                // Process arguments and redact values here.
                if !args.is_empty() {
                    return Some(Err(
                        "Our example dynamic command never takes arguments!"
                            .to_string()
                            .into(),
                    ));
                }
                return Some(Ok(Vec::new()));
            }
        }
        None
    }

    fn try_from_args(
        command_name: &[&str],
        args: &[&str],
    ) -> Option<Result<Self, EarlyExit>> {
        for command in Self::commands() {
            if command_name.last() == Some(&command.name) {
                if !args.is_empty() {
                    return Some(Err(
                        "Our example dynamic command never takes arguments!"
                            .to_string()
                            .into(),
                    ));
                }
                return Some(Ok(PyenvCommand {}));
            }
        }
        None
    }
}

impl FakeDynamic for PyenvCommand {
    fn run(&self, driver: &fud_core::Driver) -> anyhow::Result<()> {
        let data_dir = config::data_dir(&driver.name);

        fs::create_dir_all(&data_dir)?;

        let pyenv = data_dir.join("venv");

        // create new venv
        Command::new("python3")
            .args(["-m", "venv"])
            .arg(&pyenv)
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
}
