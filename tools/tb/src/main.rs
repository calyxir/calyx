use std::{env, path::PathBuf};
use tb::{
    cli::CLI,
    driver::Driver,
    error::{LocalError, LocalResult},
};

const CONFIG_FILE_NAME: &str = "calyx-tb.toml";

fn main() -> LocalResult<()> {
    let args: CLI = argh::from_env();

    if args.version {
        println!(
            "{} v{}",
            std::env::current_exe()
                .expect("how did you call this without argv[0]??")
                .to_str()
                .expect("argv[0] not valid unicode"),
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }

    let config_path = match args.config {
        Some(config_path) => config_path,
        None => {
            let mut config_path =
                PathBuf::from(env::var("HOME").expect("user has no $HOME :("));
            config_path.push(".config");
            config_path.push(CONFIG_FILE_NAME);
            config_path
        }
    };

    if !config_path.exists() {
        return Err(LocalError::other(format!(
            "missing config file {}",
            config_path.to_string_lossy()
        )));
    }

    let driver = Driver::new();
    driver.run(args.using, config_path, args.input, &args.tests)
}
