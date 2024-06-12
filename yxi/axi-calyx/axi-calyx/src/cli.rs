use argh::FromArgs;
use std::path::{Path, PathBuf};

/// AXI generator.
#[derive(FromArgs)]
pub struct ParseArgs {
    #[argh(switch, description = "disable logging", short = 'q')]
    pub quiet: bool,

    #[argh(
        option,
        description = "library path",
        short = 'l',
        default = "Path::new(\".\").into()"
    )]
    pub lib_path: PathBuf,

    #[argh(
        positional,
        description = "YXI file",
        default = "String::from(\"input.yxi\")"
    )]
    pub input_file: String,
}
