use argh::FromArgs;

/// AXI generator.
#[derive(FromArgs)]
pub struct ParseArgs {
    #[argh(
        positional,
        description = "YXI file",
        default = "String::from(\"input.yxi\")"
    )]
    pub input_file: String,
}
