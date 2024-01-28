use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::{env, path::Path};

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// The `ninja` command to execute in `run` mode.
    pub ninja: String,

    /// Never delete the temporary directory used to execute ninja in `run` mode.
    pub keep_build_dir: bool,

    /// Enable verbose output.
    pub verbose: bool,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            ninja: "ninja".to_string(),
            keep_build_dir: false,
            verbose: false,
        }
    }
}

/// Load configuration data from the standard config file location.
pub(crate) fn load_config(name: &str) -> Figment {
    // The configuration is usually at `~/.config/driver_name.toml`.
    let config_base = env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        let home = env::var("HOME").expect("$HOME not set");
        home + "/.config"
    });
    let config_path = Path::new(&config_base).join(name).with_extension("toml");

    // Use our defaults, overridden by the TOML config file.
    Figment::from(Serialized::defaults(GlobalConfig::default())).merge(Toml::file(config_path))
}
