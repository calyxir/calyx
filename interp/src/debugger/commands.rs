use calyx_ir::Id;
use itertools::{self, Itertools};
use lazy_static::lazy_static;
use owo_colors::OwoColorize;
use std::{
    fmt::{Display, Write},
    marker::PhantomData,
};

use crate::structures::names::CompGroupName;

#[derive(Debug)]
pub struct ParsedGroupName {
    component: Option<calyx_ir::Id>,
    group: calyx_ir::Id,
}

impl ParsedGroupName {
    pub fn from_group_name(group: calyx_ir::Id) -> Self {
        Self {
            component: None,
            group,
        }
    }

    pub fn from_comp_and_group(
        component: calyx_ir::Id,
        group: calyx_ir::Id,
    ) -> Self {
        Self {
            component: Some(component),
            group,
        }
    }

    pub fn is_concrete(&self) -> bool {
        self.component.is_some()
    }

    pub fn unwrap_concrete(self) -> CompGroupName {
        CompGroupName::new(self.group, self.component.unwrap())
    }

    pub fn concretize(
        mut self,
        main_comp_name: &calyx_ir::Id,
    ) -> CompGroupName {
        if !self.is_concrete() {
            self.component = Some(*main_comp_name);
        }

        self.unwrap_concrete()
    }
}

pub enum BreakPointId {
    Name(ParsedGroupName),
    Number(u64),
}

impl From<ParsedGroupName> for BreakPointId {
    fn from(grp: ParsedGroupName) -> Self {
        Self::Name(grp)
    }
}
impl From<u64> for BreakPointId {
    fn from(n: u64) -> Self {
        Self::Number(n)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PrintCode {
    Binary,
    Unsigned,
    Signed,
    UFixed(usize),
    SFixed(usize),
}

impl Default for PrintCode {
    fn default() -> Self {
        Self::Unsigned
    }
}

impl Display for PrintCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PrintCode::Binary => "\\b".cyan().to_string(),
                PrintCode::Unsigned => "\\u".blue().to_string(),
                PrintCode::Signed => "\\s".yellow().to_string(),
                PrintCode::UFixed(n) => format!("\\u.{}", n),
                PrintCode::SFixed(n) => format!("\\s.{}", n),
            }
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub enum WatchPosition {
    Before,
    After,
}

impl Default for WatchPosition {
    fn default() -> Self {
        Self::Before
    }
}

#[derive(Debug)]
pub enum PrintMode {
    State,
    Port,
}
#[derive(Debug)]
pub struct PrintTuple(Vec<Vec<Id>>, Option<PrintCode>, PrintMode);

impl PrintTuple {
    pub fn target(&self) -> &Vec<Vec<Id>> {
        &self.0
    }

    pub fn print_code(&self) -> &Option<PrintCode> {
        &self.1
    }

    pub fn print_mode(&self) -> &PrintMode {
        &self.2
    }
}

impl From<(Vec<Vec<Id>>, Option<PrintCode>, PrintMode)> for PrintTuple {
    fn from(val: (Vec<Vec<Id>>, Option<PrintCode>, PrintMode)) -> Self {
        PrintTuple(val.0, val.1, val.2)
    }
}

impl Display for PrintTuple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self.2 {
                PrintMode::State => "print-state".green(),
                PrintMode::Port => "print".green(),
            }
        )?;
        write!(
            f,
            " {}",
            match &self.1 {
                Some(s) => format!("{}", s),
                None => "".red().to_string(),
            }
        )?;
        write!(
            f,
            " {}",
            &self.0.iter().map(|x| x.iter().join(".")).join(" "),
        )
    }
}

pub enum Command {
    Step(u64), // Step execution
    Continue,  // Execute until breakpoint
    Empty,     // Empty command, does nothing
    Display,   // Display full environment contents
    Print(Vec<Vec<calyx_ir::Id>>, Option<PrintCode>, PrintMode), // Print something
    Break(Vec<ParsedGroupName>), // Create a breakpoint
    Help,                        // Help message
    Exit,                        // Exit the debugger
    InfoBreak,                   // List breakpoints
    InfoWatch,
    Disable(Vec<BreakPointId>),
    Enable(Vec<BreakPointId>),
    Delete(Vec<BreakPointId>),
    DeleteWatch(Vec<BreakPointId>),
    StepOver(ParsedGroupName),
    Watch(
        ParsedGroupName,
        WatchPosition,
        Vec<Vec<calyx_ir::Id>>,
        Option<PrintCode>,
        PrintMode,
    ),
    PrintPC(bool),
    Explain,
}

type Description = &'static str;
type UsageExample = &'static str;
type CommandName = &'static str;

impl Command {
    pub fn get_help_string() -> String {
        let mut out = String::new();
        for CommandInfo {
            invocation: names,
            description: message,
            ..
        } in COMMAND_INFO.iter()
        {
            writeln!(out, "    {: <20}{}", names.join(", "), message.green())
                .unwrap();
        }

        out
    }

    pub fn get_explain_string() -> String {
        let mut out = String::new();
        for CommandInfo {
            invocation,
            description,
            usage_example,
        } in COMMAND_INFO.iter().filter(|x| !x.usage_example.is_empty())
        {
            writeln!(out).unwrap();
            writeln!(out, "{}", invocation.join(", ")).unwrap();
            writeln!(out, "   {}", description).unwrap();
            writeln!(
                out,
                "     {}",
                usage_example.join("\n     ").blue().italic()
            )
            .unwrap();
        }
        writeln!(out).unwrap();
        out
    }
}

// I wouldn't recommend looking at this

lazy_static! {
    /// A (lazy) static list of [CommandInfo] objects used for the help and
    /// explain messages
    static ref COMMAND_INFO: Vec<CommandInfo> = {
        vec![
            // step
            CIBuilder::new().invocation("step")
                .invocation("s")
                .description("Advance the execution by a step. If provided a number, it will advance by that many steps (skips breakpoints).")
                .usage("> s").usage("> s 5").build(),
            // step-over
            CIBuilder::new().invocation("step-over")
                .description("Advance the execution over a given group.")
                .usage("> step-over this_group").build(),
            // continue
            CIBuilder::new().invocation("continue")
                .invocation("c")
                .description("Continue until the program finishes executing or hits a breakpoint").build(),
            // display
            CIBuilder::new().invocation("display")
                .invocation("d")
                .description("Display the full state of the main component").build(),
            // print
            CIBuilder::new().invocation("print")
                .invocation("p")
                .description("Print target value. Takes an optional print code before the target. Valid print codes are \\u (unsigned), \\s (signed), \\u.X (unsigned fixed-point, X frac bits), \\s.X (signed fixed-point)")
                .usage("> p reg.write_en").usage("> p \\u mult1").build(),
            // print-state
            CIBuilder::new().invocation("print-state")
                .description("Print the internal state of the target cell. Takes an optional print code before the target")
                .usage("> watch after GROUP with print-state \\s mem").build(),
            // watch
            CIBuilder::new().invocation("watch")
                .description("Watch a given group with a print statement. Takes an optional position (before/after)")
                .usage("> watch GROUP with p \\u reg.in").usage("> watch after GROUP with print-state \\s mem").build(),
            // where
            CIBuilder::new().invocation("where")
                .invocation("pc")
                .description("Displays the current program location using source metadata if applicable otherwise showing the calyx tree").build(),
            // where calyx
            CIBuilder::new().invocation("where calyx")
                .description("Enhance 'where' command adding an optional flag that enables  printing calyx group tree, even if source information is not available").build(),
            // help
            CIBuilder::new().invocation("help")
                .invocation("h")
                .description("Print this message").build(),
            // break
            CIBuilder::new().invocation("break")
                .invocation("br")
                .description("Create a breakpoint")
                .usage("> br do_add",).usage("> br subcomp::let0").build(),
            // info break
            CIBuilder::new().invocation("info break")
                .invocation("ib")
                .description("List all breakpoints").build(),
            // delete
            CIBuilder::new().invocation("delete")
                .invocation("del")
                .description("Delete target breakpoint")
                .usage("> del 1").usage("> del do_add").build(),
            // enable
            CIBuilder::new().invocation("enable")
                .description("Enable target breakpoint")
                .usage("> enable 1").usage("> enable do_add").build(),
            // disable
            CIBuilder::new().invocation("disable")
                .description("Disable target breakpoint")
                .usage("> disable 4").usage("> disable do_mult").build(),
            // explain
            CIBuilder::new().invocation("explain")
                .description("Show examples of commands which take arguments").build(),
            // exit/quit
            CIBuilder::new().invocation("exit")
                .invocation("quit")
                .description("Exit the debugger").build(),
        ]
    };
}

#[derive(Clone, Debug)]
pub struct CommandInfo {
    invocation: Vec<CommandName>,
    description: Description,
    usage_example: Vec<UsageExample>,
}

// type shenanigans

trait BuildState {}
struct Missing;
impl BuildState for Missing {}
struct Present;
impl BuildState for Present {}

#[derive(Default, Clone, Debug)]
struct CommandInfoBuilder<I, D>
where
    I: BuildState,
    D: BuildState,
{
    invocation: Vec<CommandName>,
    description: Option<Description>,
    usage_example: Vec<UsageExample>,
    phantom_i: PhantomData<I>,
    phantom_d: PhantomData<D>,
}

type CIBuilder = CommandInfoBuilder<Missing, Missing>;

impl CommandInfoBuilder<Missing, Missing> {
    fn new() -> Self {
        Self {
            invocation: Default::default(),
            description: Default::default(),
            usage_example: Default::default(),
            phantom_i: PhantomData,
            phantom_d: PhantomData,
        }
    }
}

impl<I, D> CommandInfoBuilder<I, D>
where
    I: BuildState,
    D: BuildState,
{
    fn invocation(
        mut self,
        val: CommandName,
    ) -> CommandInfoBuilder<Present, D> {
        self.invocation.push(val);

        CommandInfoBuilder {
            invocation: self.invocation,
            description: self.description,
            usage_example: self.usage_example,
            phantom_i: PhantomData::<Present>,
            phantom_d: self.phantom_d,
        }
    }

    fn description(
        mut self,
        desc: Description,
    ) -> CommandInfoBuilder<I, Present> {
        self.description = Some(desc);
        CommandInfoBuilder {
            invocation: self.invocation,
            description: self.description,
            usage_example: self.usage_example,
            phantom_i: self.phantom_i,
            phantom_d: PhantomData::<Present>,
        }
    }

    fn usage(mut self, usage: UsageExample) -> CommandInfoBuilder<I, D> {
        self.usage_example.push(usage);
        CommandInfoBuilder {
            invocation: self.invocation,
            description: self.description,
            usage_example: self.usage_example,
            phantom_i: self.phantom_i,
            phantom_d: self.phantom_d,
        }
    }
}

impl CommandInfoBuilder<Present, Present> {
    fn build(self) -> CommandInfo {
        CommandInfo {
            invocation: self.invocation,
            description: self.description.unwrap(),
            usage_example: self.usage_example,
        }
    }
}
