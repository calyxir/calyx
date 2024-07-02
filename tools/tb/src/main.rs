use std::{env, path::PathBuf};
use tb::{
    cli::CLI,
    driver::Driver,
    error::{LocalError, LocalResult},
};

const CONFIG_FILE_NAME: &str = "calyx-tb.toml";

fn main() -> LocalResult<()> {
    let args = CLI::from_env();

    if args.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
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

    let default_loc = {
        let mut default_loc = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        default_loc.push("plugins");
        default_loc
    };
    let driver = Driver::load(&[default_loc])?;
    driver.run(args.using, config_path, args.input, &args.tests)
}
