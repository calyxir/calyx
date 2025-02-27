//! This module contains the core data structures and commands used by the debugger

use itertools::{self, Itertools};
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
        },
    },
    serialization::PrintCode,
};

use cider_idx::impl_index;

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

/// The name of a group taken from user input. The component may be elided in
/// which case it is generally assumed to be the entry point.
#[derive(Debug)]
pub struct ParsedGroupName {
    component: Option<String>,
    group: String,
}

impl ParsedGroupName {
    /// Create a new [ParsedGroupName] from just a group name.
    pub fn from_group_name(group: String) -> Self {
        Self {
            component: None,
            group,
        }
    }

    /// Create a new [ParsedGroupName] from a component and group name.
    pub fn from_comp_and_group(component: String, group: String) -> Self {
        Self {
            component: Some(component),
            group,
        }
    }

    /// Attempts to look up the group of the given name in the context. If the
    /// group lacks a component, it is assumed to be the entry point.
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

/// An enum representing a breakpoint/watchpoint from user input. This may or
/// may not be valid.
pub enum ParsedBreakPointID {
    /// A breakpoint given by the group name.
    Name(ParsedGroupName),
    /// A breakpoint given by the identifying number.
    Number(u32),
}

impl ParsedBreakPointID {
    /// Attempts to parse the breakpoint from user input into a concrete [BreakpointID].
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

    /// Attempts to parse the watchpoint from user input into a concrete [WatchID].
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

/// A concrete breakpoint
pub enum BreakpointID {
    /// A breakpoint on the given group. This does not guarantee that there is
    /// such a breakpoint, but it does guarantee that the group exists.
    Name(GroupIdx),
    /// A breakpoint on the given ID. This does not guarantee that there is a
    /// breakpoint by the given ID. In such cases, operations on the breakpoint
    /// will produce an error.
    Number(BreakpointIdx),
}

impl BreakpointID {
    /// Attempts to get the breakpoint ID as a number.
    #[must_use]
    pub fn as_number(&self) -> Option<&BreakpointIdx> {
        if let Self::Number(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Attempts to get the breakpoint ID as a group.
    #[must_use]
    pub fn as_name(&self) -> Option<&GroupIdx> {
        if let Self::Name(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

/// A concrete watchpoint
pub enum WatchID {
    /// A watchpoint on the given group. This does not guarantee that there is
    /// such a watchpoint, but it does guarantee that the group exists. Since
    /// multiple watchpoints may exist for a single group, any operation applied
    /// to this watchpoint will affect all of them.
    Name(GroupIdx),
    /// A watchpoint on the given ID. This does not guarantee that there is a
    /// watchpoint by the given ID. In such cases, operations on the watchpoint
    /// will produce an error.
    Number(WatchpointIdx),
}

impl WatchID {
    /// Attempts to get the watchpoint ID as a name.
    #[must_use]
    pub fn as_name(&self) -> Option<&GroupIdx> {
        if let Self::Name(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Attempts to get the watchpoint ID as a number.
    #[must_use]
    pub fn as_number(&self) -> Option<&WatchpointIdx> {
        if let Self::Number(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

/// The position of a watchpoint relative to a group's execution.
#[derive(Clone, Copy, Debug)]
pub enum WatchPosition {
    /// The watchpoint is placed at the beginning of the group execution.
    Before,
    /// The watchpoint is placed at the end of the group execution.
    After,
}

impl Default for WatchPosition {
    fn default() -> Self {
        Self::Before
    }
}

/// An enum representing what information the print command targets.
#[derive(Debug, Clone, Copy)]
pub enum PrintMode {
    /// The print command targets the state of the cell. This only works for
    /// cells which contain internal state such as registers or memories.
    State,
    /// The print command targets the port information. This may be applied to a
    /// single port, or the cell in which case all ports are printed.
    Port,
}

/// A tuple representing a print command.
///
/// The tuple consists of a list of paths to the targets to print, an optional
/// print code used to format the information, and the print mode.
#[derive(Debug, Clone)]
pub struct PrintTuple(Vec<Path>, Option<PrintCode>, PrintMode);

impl PrintTuple {
    /// Returns a reference to the list of targets to print.
    pub fn target(&self) -> &Vec<Path> {
        &self.0
    }

    /// Returns a reference to the print code.
    pub fn print_code(&self) -> &Option<PrintCode> {
        &self.1
    }

    /// Returns a reference to the print mode.
    pub fn print_mode(&self) -> &PrintMode {
        &self.2
    }

    /// Return a formatted string representing the print tuple. Used to display
    /// stored watchpoints to the user.
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
/// ParseNodes enum is used to represent what child to traverse with respect to
/// the current ControlIdx.
/// Body defines that we should go into the body of a while or repeat.
/// Offset defines which child to go to.
/// If defines whether we should go to the true or false branch next
#[derive(Debug, PartialEq, Clone)]
pub enum ParseNodes {
    Body,
    Offset(u32),
    If(bool),
}
pub struct ParsePath {
    nodes: Vec<ParseNodes>,
    component_name: String,
}

impl ParsePath {
    pub fn new(nodes: Vec<ParseNodes>, name: String) -> ParsePath {
        ParsePath {
            nodes,
            component_name: name,
        }
    }

    pub fn get_path(&self) -> Vec<ParseNodes> {
        self.nodes.clone()
    }

    pub fn get_name(&self) -> &str {
        &self.component_name
    }

    pub fn from_iter<I>(iter: I, component_name: String) -> ParsePath
    where
        I: IntoIterator<Item = ParseNodes>,
    {
        ParsePath::new(iter.into_iter().collect(), component_name)
    }
}

// Different types of printing commands
pub enum PrintCommand {
    Normal,
    PrintCalyx,
    PrintNodes,
}

/// A command that can be sent to the debugger.
pub enum Command {
    /// Advance the execution by a given number of steps (cycles).
    Step(u32),
    /// Execute until the next breakpoint. Or until the program finishes
    Continue,
    /// Empty command, does nothing.
    Empty,
    /// Display the full environment contents. Currently this command is defunct
    Display,
    /// Print out the value of the given target. Can be configured with
    /// different modes and print formats.
    Print(Vec<Vec<String>>, Option<PrintCode>, PrintMode),
    /// Create a breakpoint on the given groups.
    Break(Vec<ParsedGroupName>),
    /// Display the help message.
    Help,
    /// Exit the debugger.
    Exit,
    /// List all breakpoints.
    InfoBreak,
    /// List all watchpoints.
    InfoWatch,
    /// Disable the given breakpoints.
    Disable(Vec<ParsedBreakPointID>),
    /// Enable the given breakpoints.
    Enable(Vec<ParsedBreakPointID>),
    /// Delete the given breakpoints.
    Delete(Vec<ParsedBreakPointID>),
    /// Enable the given watchpoints.
    EnableWatch(Vec<ParsedBreakPointID>),
    /// Disable the given watchpoints.
    DisableWatch(Vec<ParsedBreakPointID>),
    /// Delete the given watchpoints.
    DeleteWatch(Vec<ParsedBreakPointID>),
    /// Advance the execution until the given group is no longer running.
    StepOver(ParsedGroupName),
    /// Create a watchpoint
    Watch(
        ParsedGroupName,
        WatchPosition,
        Vec<Vec<String>>,
        Option<PrintCode>,
        PrintMode,
    ),
    /// Print the current program counter
    PrintPC(PrintCommand),
    /// Show command examples
    Explain,
    /// Restart the debugger from the beginning of the execution. Command history, breakpoints, watchpoints, etc. are preserved.
    Restart,
}

type Description = &'static str;
type UsageExample = &'static str;
type CommandName = &'static str;

impl Command {
    /// Returns the help message for the debugger.
    pub fn get_help_string() -> String {
        let mut out = String::new();

        for CommandInfo {
            invocation: names,
            description: message,
            ..
        } in get_command_info().iter()
        {
            writeln!(out, "    {: <30}{}", names.join(", "), message.green())
                .unwrap();
        }

        out
    }

    /// Returns the usage examples for the debugger.
    pub fn get_explain_string() -> String {
        let mut out = String::new();
        for CommandInfo {
            invocation,
            description,
            usage_example,
        } in get_command_info()
            .iter()
            .filter(|x| !x.usage_example.is_empty())
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

use std::sync::LazyLock;
/// A (lazy) static list of [CommandInfo] objects used for the help and
/// explain messages. Access via [get_command_info]
static COMMAND_INFO: LazyLock<Box<[CommandInfo]>> = LazyLock::new(|| {
    [
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
            CIBuilder::new().invocation("enable").invocation("en")
                .description("Enable target breakpoint")
                .usage("> enable 1").usage("> enable do_add").build(),
            // disable
            CIBuilder::new().invocation("disable").invocation("dis")
                .description("Disable target breakpoint")
                .usage("> disable 4").usage("> disable do_mult").build(),

            // del watch
            CIBuilder::new().invocation("delete-watch").invocation("delw")
                .description("Delete target watchpoint")
                .usage("> delete-watch 1")
                .usage("> delete-watch do_add").build(),

            CIBuilder::new().invocation("enable-watch").invocation("enw")
                .description("Enable target watchpoint")
                .usage("> enable-watch 1")
                .usage("> enable-watch do_add").build(),

            CIBuilder::new().invocation("disable-watch").invocation("disw")
                .description("Disable target watchpoint")
                .usage("> disable-watch 4")
                .usage("> disable-watch do_mult").build(),
            // explain
            CIBuilder::new().invocation("explain")
                .description("Show examples of commands which take arguments").build(),
            CIBuilder::new().invocation("restart")
                .description("Restart the debugger from the beginning of the execution. Command history, breakpoints, watchpoints, etc. are preserved")
                .build(),
            CIBuilder::new().invocation("exit")
                .invocation("quit")
                .description("Exit the debugger").build(),
        ].into()
});

/// Returns the list of [CommandInfo] objects used for the help and explain
/// messages
#[inline]
fn get_command_info() -> &'static [CommandInfo] {
    &COMMAND_INFO
}

#[derive(Clone, Debug)]
struct CommandInfo {
    invocation: Box<[CommandName]>,
    description: Description,
    usage_example: Box<[UsageExample]>,
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
            invocation: self.invocation.into(),
            description: self.description.unwrap(),
            usage_example: self.usage_example.into(),
        }
    }
}
