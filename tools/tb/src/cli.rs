use argh::FromArgs;

#[derive(FromArgs)]
/// Test verilog files under various harnesses.
pub struct CLI {
    #[argh(positional)]
    /// verilog file
    pub input: String,

    #[argh(option, short = 't')]
    /// test harness
    pub tests: Vec<String>,

    #[argh(option, short = 'u')]
    /// the testbench to invoke
    pub using: String,

    #[argh(switch)]
    /// displays version information
    pub version: bool,
}
