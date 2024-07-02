use std::{env, path::PathBuf};

use argh::FromArgs;

#[derive(FromArgs)]
/// Test verilog files under various harnesses.
pub struct CLI {
    #[argh(positional)]
    /// verilog file
    pub input: String,

    #[argh(option, short = 't', long = "test")]
    /// test harness
    pub tests: Vec<String>,

    #[argh(option, short = 'u')]
    /// the testbench to invoke
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
                input: String::new(),
                tests: Vec::new(),
                using: String::new(),
                config: None,
                version: true,
            }
        } else {
            argh::from_env()
        }
    }
}
