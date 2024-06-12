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
        short = 'e'
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
        long = "pass-alias"
    )]
    pub pass_alias: String,

    #[argh(
        option,
        description = "disable pass before -b breakpoint",
        short = 'd',
        long = "disable-pass"
    )]
    pub disable: Vec<String>,

    #[argh(option, description = "focus a component", short = 'c')]
    pub component: Option<String>,

    #[argh(switch, description = "displays version information")]
    pub version: bool,

    #[argh(positional, description = "calyx file")]
    pub input_file: Option<String>,
}
