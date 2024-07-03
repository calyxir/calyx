use std::{env, path::PathBuf};
use tb::{
    cli::CLI,
    driver::Driver,
    error::{LocalError, LocalResult},
};

const CONFIG_FILE_NAME: &str = "calyx-tb.toml";

fn setup_logging() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "warn");
    }
    if env::var("NO_COLOR").is_err() {
        env::set_var("RUST_LOG_STYLE", "always");
    }

    env_logger::builder().format_target(false).init();
}

fn main() -> LocalResult<()> {
    setup_logging();

    let args = CLI::from_env();

    if args.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let config_path = match args.config {
        Some(config_path) => config_path,
        None => {
            log::info!(
                "No config file specified, using default: {}",
                CONFIG_FILE_NAME
            );
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
