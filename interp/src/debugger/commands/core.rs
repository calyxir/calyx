use itertools::{self, Itertools};
use lazy_static::lazy_static;
use owo_colors::OwoColorize;
use std::{
    fmt::{Display, Write},
    marker::PhantomData,
};

use crate::{
    flatten::{
        flat_ir::prelude::GroupIdx,
        structures::{
            context::Context,
            environment::{Environment, Path},
            index_trait::impl_index,
        },
    },
    serialization::PrintCode,
};

/// Identifier for breakpoints
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct BreakpointIdx(u32);

impl Display for BreakpointIdx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl_index!(BreakpointIdx);

/// Identifier for watchpoints
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash)]
pub struct WatchpointIdx(u32);

impl_index!(WatchpointIdx);
impl Display for WatchpointIdx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub struct ParsedGroupName {
    component: Option<String>,
    group: String,
}

impl ParsedGroupName {
    pub fn from_group_name(group: String) -> Self {
        Self {
            component: None,
            group,
        }
    }

    pub fn from_comp_and_group(component: String, group: String) -> Self {
        Self {
            component: Some(component),
            group,
        }
    }

    pub fn is_concrete(&self) -> bool {
        self.component.is_some()
    }

    pub fn concretize(&self, component: String) -> GroupName {
        GroupName {
            component: self.component.as_ref().cloned().unwrap_or(component),
            group: self.group.clone(),
        }
    }

    pub fn get_concrete(&self) -> Option<GroupName> {
        if self.is_concrete() {
            Some(GroupName {
                component: self.component.as_ref().cloned().unwrap(),
                group: self.group.clone(),
            })
        } else {
            None
        }
    }

    pub fn lookup_group(&self, context: &Context) -> Result<GroupIdx, String> {
        let comp = if let Some(c) = &self.component {
            context
                .lookup_comp_by_name(c.as_ref())
                .ok_or(format!("No component named {c}"))?
        } else {
            context.entry_point
        };

        context
            .lookup_group_by_name(self.group.as_ref(), comp)
            .ok_or(format!("No group named {} in component", self.group))
    }
}

#[derive(Debug, Clone)]
pub struct GroupName {
    pub component: String,
    pub group: String,
}

pub enum ParsedBreakPointID {
    Name(ParsedGroupName),
    Number(u32),
}

impl ParsedBreakPointID {
    pub fn parse_to_break_ids(
        &self,
        context: &Context,
    ) -> Result<BreakpointID, String> {
        match self {
            ParsedBreakPointID::Name(name) => {
                let group = name.lookup_group(context)?;
                Ok(BreakpointID::Name(group))
            }
            ParsedBreakPointID::Number(v) => {
                Ok(BreakpointID::Number(BreakpointIdx::from(*v)))
            }
        }
    }

    pub fn parse_to_watch_ids(
        &self,
        context: &Context,
    ) -> Result<WatchID, String> {
        match self {
            ParsedBreakPointID::Name(v) => {
                let group = v.lookup_group(context)?;
                Ok(WatchID::Name(group))
            }
            ParsedBreakPointID::Number(v) => {
                Ok(WatchID::Number(WatchpointIdx::from(*v)))
            }
        }
    }
}

impl From<u32> for ParsedBreakPointID {
    fn from(v: u32) -> Self {
        Self::Number(v)
    }
}

impl From<ParsedGroupName> for ParsedBreakPointID {
    fn from(v: ParsedGroupName) -> Self {
        Self::Name(v)
    }
}

pub enum BreakpointID {
    Name(GroupIdx),
    Number(BreakpointIdx),
}

impl BreakpointID {
    #[must_use]
    pub fn as_number(&self) -> Option<&BreakpointIdx> {
        if let Self::Number(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_name(&self) -> Option<&GroupIdx> {
        if let Self::Name(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

pub enum WatchID {
    Name(GroupIdx),
    Number(WatchpointIdx),
}

impl WatchID {
    #[must_use]
    pub fn as_name(&self) -> Option<&GroupIdx> {
        if let Self::Name(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_number(&self) -> Option<&WatchpointIdx> {
        if let Self::Number(v) = self {
            Some(v)
        } else {
            None
        }
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

#[derive(Debug, Clone, Copy)]
pub enum PrintMode {
    State,
    Port,
}
#[derive(Debug, Clone)]
pub struct PrintTuple(Vec<Path>, Option<PrintCode>, PrintMode);

impl PrintTuple {
    pub fn target(&self) -> &Vec<Path> {
        &self.0
    }

    pub fn print_code(&self) -> &Option<PrintCode> {
        &self.1
    }

    pub fn print_mode(&self) -> &PrintMode {
        &self.2
    }

    pub fn format<C: AsRef<Context> + Clone>(
        &self,
        env: &Environment<C>,
    ) -> String {
        let mut string = String::new();

        write!(
            string,
            "{}",
            match self.2 {
                PrintMode::State => "print-state".green(),
                PrintMode::Port => "print".green(),
            }
        )
        .unwrap();
        write!(
            string,
            " {}",
            match &self.1 {
                Some(s) => format!("{}", s),
                None => "".red().to_string(),
            }
        )
        .unwrap();
        write!(
            string,
            " {}",
            &self.0.iter().map(|x| x.as_string(env)).join(" "),
        )
        .unwrap();

        string
    }
}

impl From<(Vec<Path>, Option<PrintCode>, PrintMode)> for PrintTuple {
    fn from(val: (Vec<Path>, Option<PrintCode>, PrintMode)) -> Self {
        PrintTuple(val.0, val.1, val.2)
    }
}

pub enum Command {
    Step(u32),                                             // Step execution
    Continue, // Execute until breakpoint
    Empty,    // Empty command, does nothing
    Display,  // Display full environment contents
    Print(Vec<Vec<String>>, Option<PrintCode>, PrintMode), // Print something
    Break(Vec<ParsedGroupName>), // Create a breakpoint
    Help,     // Help message
    Exit,     // Exit the debugger
    InfoBreak, // List breakpoints
    InfoWatch,
    Disable(Vec<ParsedBreakPointID>),
    Enable(Vec<ParsedBreakPointID>),
    Delete(Vec<ParsedBreakPointID>),
    EnableWatch(Vec<ParsedBreakPointID>),
    DisableWatch(Vec<ParsedBreakPointID>),
    DeleteWatch(Vec<ParsedBreakPointID>),
    StepOver(ParsedGroupName),
    Watch(
        ParsedGroupName,
        WatchPosition,
        Vec<Vec<String>>,
        Option<PrintCode>,
        PrintMode,
    ),
    PrintPC(bool),
    Explain,
    Restart,
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
                .usage("> print-state \\s mem").build(),
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
            CIBuilder::new().invocation("restart")
                .description("Restart the debugger from the beginning of the execution. Command history, breakpoints, watchpoints, etc. are preserved").build(),
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
