use std::{env, path::PathBuf, str::FromStr};

use argh::FromArgs;

use crate::error::{LocalError, LocalResult};

pub struct ConfigSet {
    pub key: String,
    pub value: String,
}

impl FromStr for ConfigSet {
    type Err = LocalError;

    fn from_str(s: &str) -> LocalResult<ConfigSet> {
        s.find('=')
            .map(|index| {
                let (key, value) = s.split_at(index);
                let value = value.chars().skip(1).collect();
                ConfigSet {
                    key: key.to_string(),
                    value,
                }
            })
            .ok_or(LocalError::other("expected syntax 'key=value'"))
    }
}

#[derive(FromArgs, Default)]
/// Test verilog files under various harnesses.
pub struct CLI {
    #[argh(positional)]
    /// verilog or calyx file
    pub input: String,

    #[argh(option, short = 't', long = "test")]
    /// test harness
    pub tests: Vec<String>,

    #[argh(option, short = 's')]
    /// set a config option
    pub set: Vec<ConfigSet>,

    #[argh(option, short = 'u')]
    /// the testbench to invoke, e.g., verilator, cocotb, calyx
    pub using: String,

    /// path to the config file
    #[argh(option, short = 'c')]
    pub config: Option<PathBuf>,

    #[argh(switch)]
    /// displays version information
    pub version: bool,
}

impl CLI {
    pub fn from_env() -> Self {
        let args: Vec<_> = env::args().collect();
        if args.len() == 2 && matches!(args[1].as_str(), "-v" | "--version") {
            Self {
                version: true,
                ..Default::default()
            }
        } else {
            argh::from_env()
        }
    }
}
