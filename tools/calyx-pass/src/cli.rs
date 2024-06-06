use argh::FromArgs;

#[derive(FromArgs)]
#[argh(
    description = "A tool for visualizing pass transformations.\n\nAuthor: Ethan Uppal"
)]
pub struct ParseArgs {
    #[argh(
        option,
        description = "location of calyx executable",
        default = "String::from(\"\")",
        short = 'e',
        long = "exec"
    )]
    pub calyx_exec: String,

    #[argh(
        option,
        description = "first pass to not auto-accept",
        short = 'b',
        long = "break"
    )]
    pub breakpoint: Option<String>,

    #[argh(
        option,
        description = "pass alias to debug",
        default = "String::from(\"all\")",
        short = 'p',
        long = "pass"
    )]
    pub pass_alias: String,

    #[argh(option, description = "focus a component", short = 'c')]
    pub component: Option<String>,

    #[argh(positional, description = "calyx file")]
    pub input_file: String,
}
