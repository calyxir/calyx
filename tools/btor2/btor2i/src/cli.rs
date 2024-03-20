use clap::Parser;

#[derive(Parser)]
#[command(about, version, author)] // keeps the cli synced with Cargo.toml
#[command(allow_hyphen_values(true))]
pub struct CLI {
    /// The BTOR2 file to run. stdin is assumed if file is not provided
    #[arg(short, long, action)]
    pub file: Option<String>,

    /// Profile mode
    #[arg(short, long, default_value = "false")]
    pub profile: bool,

    /// The number of times to repeat the simulation (used for profiling)
    #[arg(short, long, default_value = "1")]
    pub num_repeat: usize,

    /// Inputs for the main function
    #[arg(action)]
    pub inputs: Vec<String>,
}
