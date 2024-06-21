// Author: Ethan Uppal

//! This library serves as a rust version of calyx-py. The main builder API is
//! via [`Program`], from where you can create [`Component`]s. Most `struct`s
//! here are [`CalyxWriter`]s, so you can easily obtain their calyx
//! representations (although, of course, that of [`Program`] is arguably the
//! most useful). Macros have been developed for creating cells, groups, and
//! control; they are the *intended* way of constructing these elements.

use std::{
    cell::RefCell,
    fmt::{self, Display, Write},
    num::NonZeroU64,
    path::PathBuf,
    rc::Rc,
    str::FromStr,
};

/// Shorthand for the `Rc<RefCell<>>` pattern.
pub type RRC<T> = Rc<RefCell<T>>;

/// Convenience constructor for [`RRC`].
fn rrc<T>(value: T) -> RRC<T> {
    Rc::new(RefCell::new(value))
}

pub struct IndentFormatter<'a, 'b: 'a> {
    current_indent: String,
    add_indent: usize,
    last_was_newline: bool,
    formatter: &'a mut fmt::Formatter<'b>,
}

impl<'a, 'b: 'a> IndentFormatter<'a, 'b> {
    /// Constructs a new formatter managing indents of `indent` spaces in the
    /// wrapped formatter `formatter`.
    pub fn new(formatter: &'a mut fmt::Formatter<'b>, indent: usize) -> Self {
        Self {
            current_indent: String::new(),
            add_indent: indent,
            last_was_newline: false,
            formatter,
        }
    }

    /// Adds a level of indentation.
    pub fn increase_indent(&mut self) {
        for _ in 0..self.add_indent {
            self.current_indent.push(' ');
        }
    }

    /// Removes a level of indentation.
    pub fn decrease_indent(&mut self) {
        for _ in 0..self.add_indent {
            self.current_indent.pop();
        }
    }
}

impl fmt::Write for IndentFormatter<'_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            if self.last_was_newline {
                self.formatter.write_str(&self.current_indent)?;
            }
            self.formatter.write_char(c)?;
            self.last_was_newline = c == '\n';
        }
        Ok(())
    }
}

/// A location in the source code for source element `element` and optionally a
/// unique identifier `id` for that element. For example,
/// ```
/// // COMPONENT END: main
/// ```
/// would be a marker for the "component" element and with a unique identifier
/// of "main".
struct Marker {
    element: String,
    id: Option<String>,
}

impl Marker {
    /// A marker for an `element`.
    #[allow(dead_code)]
    pub fn general<S: AsRef<str>>(element: S) -> Marker {
        Marker {
            element: element.as_ref().to_string(),
            id: None,
        }
    }

    /// A marker for a unique `element` identified by `id`.
    pub fn unique<S: AsRef<str>, T: AsRef<str>>(element: S, id: T) -> Marker {
        Marker {
            element: element.as_ref().to_string(),
            id: Some(id.as_ref().to_string()),
        }
    }

    /// Constructs a comment string for the marker at a given `location`. For
    /// example, `marker.to_string("end")`.
    pub fn to_string<S: AsRef<str>>(&self, location: S) -> String {
        format!(
            "// {} {}{}\n",
            self.element.to_ascii_uppercase(),
            location.as_ref().to_string().to_ascii_uppercase(),
            self.id
                .as_ref()
                .map(|id| format!(": {}", id))
                .unwrap_or_default()
        )
    }
}

/// An element of calyx source code that MUST implement [`CalyxWriter::write`],
/// can OPTIONALLY implement [`CalyxWriter::marker`], and must NEVER override
/// [`CalyxWriter::writeln`].
trait CalyxWriter {
    fn marker(&self) -> Option<Marker> {
        None
    }

    /// Writes this element to `f`. See also [`CalyxWriter::writeln`].
    fn write(&self, f: &mut IndentFormatter<'_, '_>) -> fmt::Result;

    /// Writes this element followed by a newline and wrapped with markers if
    /// the element provides them. See also [`CalyxWriter::write`].
    ///
    /// Do NOT override this function. See details at [`CalyxWriter`].
    fn writeln(&self, f: &mut IndentFormatter<'_, '_>) -> fmt::Result {
        if let Some(marker) = self.marker() {
            write!(f, "{}", marker.to_string("start"))?;
            self.write(f)?;
            f.write_char('\n')?;
            write!(f, "{}", marker.to_string("end"))?;
        } else {
            self.write(f)?;
            f.write_char('\n')?;
        }
        Ok(())
    }
}

/// Imports a calyx file from the standard library.
struct Import {
    path: PathBuf,
}

impl CalyxWriter for Import {
    fn write(&self, f: &mut IndentFormatter<'_, '_>) -> fmt::Result {
        write!(
            f,
            "import \"{}\";",
            self.path.to_str().expect("invalid path")
        )
    }
}

/// See [`calyx_frontend::Attributes`](https://docs.calyxir.org/source/calyx_frontend/struct.Attributes.html).
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Attribute {
    name: String,
    value: u64,
}

impl Attribute {
    /// Constructs a `calyx_frontend::BoolAttr`, such as `@external`, named
    /// `name`.
    pub fn bool<S: AsRef<str>>(name: S) -> Self {
        Self {
            name: name.as_ref().to_string(),
            value: 1,
        }
    }

    /// Constructs a `calyx_frontend::NumAttr`, such as `@go`, named `name`.
    pub fn num<S: AsRef<str>>(name: S, value: u64) -> Self {
        Self {
            name: name.as_ref().to_string(),
            value,
        }
    }
}

impl CalyxWriter for Attribute {
    fn write(&self, f: &mut IndentFormatter<'_, '_>) -> fmt::Result {
        write!(f, "@{}({})", self.name, self.value)
    }
}

/// A list of attributes.
type Attributes = Vec<Attribute>;

/// Abstracts attribute functionality. This trait doesn't actually do anything,
/// but it ties together all the implementations via the type system to make
/// them more maintainable in response to API changes.
trait AttributeProvider {
    fn add_attribute(&mut self, attr: Attribute);

    fn with_attribute(mut self, attr: Attribute) -> Self
    where
        Self: Sized,
    {
        self.add_attribute(attr);
        self
    }
}

impl CalyxWriter for Attributes {
    fn write(&self, f: &mut IndentFormatter<'_, '_>) -> fmt::Result {
        for attr in self {
            attr.write(f)?;
            f.write_char(' ')?;
        }
        Ok(())
    }
}

/// A port on a cell or a component.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Port {
    attributes: Vec<Attribute>,
    /// The cell that this port belongs to, or `None` if it is in a component
    /// signature.
    parent: Option<String>,
    name: String,
    width: Option<NonZeroU64>,
}

impl Port {
    /// Constructs a new port with the given `parent` and `name`.
    ///
    /// Requires: `width` is nonzero.
    pub fn new<S: AsRef<str>, T: AsRef<str>>(
        parent: Option<S>,
        name: T,
        width: u64,
    ) -> Self {
        Self {
            attributes: vec![],
            parent: parent.map(|s| s.as_ref().to_string()),
            name: name.as_ref().to_string(),
            width: Some(NonZeroU64::new(width).expect("width cannot be zero")),
        }
    }

    /// Constructs a new port with the given `name` in a component.
    ///
    /// Requires: `width > 0`.
    pub fn new_in_comp<S: AsRef<str>>(name: S, width: u64) -> Self {
        Self::new::<String, S>(None, name, width)
    }

    /// Constructs a new port with the given `parent` and `name` and with
    /// inferred width.
    pub fn inferred<S: AsRef<str>, T: AsRef<str>>(
        parent: Option<S>,
        name: T,
    ) -> Self {
        Self {
            attributes: vec![],
            parent: parent.map(|s| s.as_ref().to_string()),
            name: name.as_ref().to_string(),
            width: None,
        }
    }

    pub fn has_inferred_width(&self) -> bool {
        self.width.is_none()
    }
}

impl AttributeProvider for Port {
    fn add_attribute(&mut self, attr: Attribute) {
        self.attributes.push(attr);
    }
}

impl Display for Port {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(parent) = &self.parent {
            write!(f, "{}.", parent)?;
        }
        write!(f, "{}", self.name)
    }
}

impl CalyxWriter for Port {
    /// Behaves identically to [`Port::fmt`]. Please read [`CalyxWriter::write`]
    /// for documentation on this function.
    fn write(&self, f: &mut IndentFormatter<'_, '_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

/// Abstracts port functionality over components and cells. This trait doesn't
/// actually do anything, but it ties together all the implementations via the
/// type system to make them more maintainable in response to API changes. For
/// example, Griffin suggested I change the name of the member function from
/// `get` to `get_port`, which I could achieve everywhere with a simple
/// refactor.
pub trait PortProvider {
    /// Retrieves the port on this provider named `port`.
    ///
    /// Requires: `port` exists on this provider.
    fn get_port(&self, port: String) -> Port;
}

/// See `calyx_ir::Cell`.
pub struct Cell {
    is_ref: bool,
    attributes: Vec<Attribute>,
    name: String,
    /// The name of the component or primitive instantiated, e.g., "std_reg"
    /// for the register primitive.
    inst: String,
    args: Vec<u64>,
}

impl Cell {
    /// Whether this cell is a ref cell.
    pub fn is_ref(&self) -> bool {
        self.is_ref
    }
}

impl PortProvider for Cell {
    fn get_port(&self, port: String) -> Port {
        Port::inferred(Some(self.name.clone()), port)
    }
}

impl AttributeProvider for Cell {
    fn add_attribute(&mut self, attr: Attribute) {
        self.attributes.push(attr);
    }
}

impl CalyxWriter for Cell {
    fn write(&self, f: &mut IndentFormatter<'_, '_>) -> fmt::Result {
        self.attributes.write(f)?;
        write!(
            f,
            "{}{} = {}({});",
            if self.is_ref { "ref " } else { "" },
            self.name,
            self.inst,
            self.args
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

// Griffin: To state the obvious, which I'm sure you'll add later. This is missing Or, Not, and BinOps. Also worth noting that Port's are only allowed as individual guard elements when they are 1bit values, otherwise they have to be part of some comparison operator

/// A guard for an [`Assignment`].
#[derive(PartialEq, Eq, Debug)]
pub enum Guard {
    True,
    Port(Port),
    Not(Box<Guard>),
    And(Box<Guard>, Box<Guard>),
    Or(Box<Guard>, Box<Guard>),
}

impl CalyxWriter for Guard {
    fn write(&self, f: &mut IndentFormatter<'_, '_>) -> fmt::Result {
        match self {
            Guard::True => {}
            Guard::Port(port) => port.write(f)?,
            Guard::And(lhs, rhs) => {
                write!(f, "(")?;
                lhs.write(f)?;
                write!(f, " & ")?;
                rhs.write(f)?;
                write!(f, ")")?;
            }
            _ => todo!("not all guards implemented"),
        }
        Ok(())
    }
}

/// See `calyx_ir::Assignment`.
#[derive(PartialEq, Eq, Debug)]
pub struct Assignment {
    lhs: Port,
    rhs: Port,
    guard: Guard,
}

impl Assignment {
    /// Constructs a new assignment guarded by `guard` between `lhs` and `rhs`.
    ///
    /// Requires: the assignment is well-formed.
    fn new(lhs: Port, rhs: Port, guard: Guard) -> Self {
        if !lhs.has_inferred_width() && !rhs.has_inferred_width() {
            assert!(lhs.width == rhs.width, "port width mismatch");
        }
        Self { lhs, rhs, guard }
    }
}

impl CalyxWriter for Assignment {
    fn write(&self, f: &mut IndentFormatter<'_, '_>) -> fmt::Result {
        write!(f, "{} = ", self.lhs)?;
        if self.guard != Guard::True {
            self.guard.write(f)?;
            write!(f, " ? ")?;
        }
        write!(f, "{};", self.rhs)
    }
}

/// See `calyx_ir::Group`. Contains optional documentation.
#[derive(PartialEq, Eq, Debug)]
pub struct Group {
    name: String,
    description: Option<String>,
    is_comb: bool,
    /// `None` if dynamic, `Some(latency)` if static and taking `latency cycles.`
    latency: Option<usize>,
    assignments: Vec<Assignment>,
}

impl Group {
    /// Constructs an empty group named `name`, combinational if and only if
    /// `is_comb`.
    fn new<S: AsRef<str>>(name: S, is_comb: bool) -> Self {
        Self {
            name: name.as_ref().to_string(),
            description: None,
            is_comb,
            latency: None,
            assignments: vec![],
        }
    }

    /// Sets a human-readable description of the group. This description may
    /// take multiple lines.
    pub fn describe(&mut self, description: String) {
        self.description = Some(description);
    }

    pub fn set_latency(&mut self, latency: usize) {
        self.latency = Some(latency);
    }

    pub fn is_comb(&self) -> bool {
        self.is_comb
    }

    /// Adds an unguarded assignment between `lhs` and `rhs`. Behaves like
    /// [`Group::assign_guarded`] with [`Guard::None`] passed for the guard.
    pub fn assign(&mut self, lhs: Port, rhs: Port) {
        self.assign_guarded(lhs, rhs, Guard::True);
    }

    /// Adds an assignment guarded by `guard` between `lhs` and `rhs`.
    pub fn assign_guarded(&mut self, lhs: Port, rhs: Port, guard: Guard) {
        self.assignments.push(Assignment::new(lhs, rhs, guard));
    }
}

impl PortProvider for Group {
    fn get_port(&self, port: String) -> Port {
        Port::inferred(Some(self.name.clone()), port)
    }
}

impl CalyxWriter for Group {
    fn write(&self, f: &mut IndentFormatter<'_, '_>) -> fmt::Result {
        if let Some(description) = &self.description {
            for line in description.lines() {
                writeln!(f, "// {}", line)?;
            }
        }
        if let Some(latency) = self.latency {
            write!(f, "static<{}> ", latency)?;
        }
        if self.is_comb {
            write!(f, "comb ")?;
        }
        writeln!(f, "group {} {{", self.name)?;
        f.increase_indent();
        for assignment in &self.assignments {
            assignment.writeln(f)?;
        }
        f.decrease_indent();
        write!(f, "}}")
    }
}

/// Helper variant with [`Control`].
#[derive(PartialEq, Eq, Debug)]
enum ControlValue {
    Empty,
    Enable(RRC<Group>),
    Seq(Vec<Control>),
    Par(Vec<Control>),
    While(Port, Option<RRC<Group>>, Vec<Control>),
    If(Port, Option<RRC<Group>>, Vec<Control>, Vec<Control>),
}

/// Structured calyx control with attributes.
#[derive(PartialEq, Eq, Debug)]
pub struct Control {
    attributes: Attributes,
    value: ControlValue,
}

impl Control {
    /// Constructs an empty control node.
    pub fn empty() -> Self {
        Control {
            attributes: Attributes::new(),
            value: ControlValue::Empty,
        }
    }

    /// Constructs a control node that enables `group` in a context-dependent
    /// way. For example, in a sequence control, this control will enable the
    /// group after all previous nodes in sequence.
    pub fn enable(group: RRC<Group>) -> Self {
        Control {
            attributes: Attributes::new(),
            value: ControlValue::Enable(group),
        }
    }

    /// Constructs a sequential control node.
    pub fn seq(nodes: Vec<Control>) -> Self {
        Control {
            attributes: Attributes::new(),
            value: ControlValue::Seq(nodes),
        }
    }

    /// Constructs a parallel control node.
    pub fn par(nodes: Vec<Control>) -> Self {
        Control {
            attributes: Attributes::new(),
            value: ControlValue::Par(nodes),
        }
    }

    /// Constructs a while loop control node. A combinational group `group` may
    /// optionally be activated for the condition `cond`.
    pub fn while_(
        cond: Port,
        group: Option<RRC<Group>>,
        controls: Vec<Control>,
    ) -> Self {
        Control {
            attributes: Attributes::new(),
            value: ControlValue::While(cond, group, controls),
        }
    }

    /// Constructs a conditional branching control node. A combinational group
    /// `group` may optionally be activated for the condition `cond`.
    pub fn if_(
        port: Port,
        group: Option<RRC<Group>>,
        then_controls: Vec<Control>,
        else_controls: Vec<Control>,
    ) -> Self {
        Control {
            attributes: Attributes::new(),
            value: ControlValue::If(port, group, then_controls, else_controls),
        }
    }
}

impl AttributeProvider for Control {
    fn add_attribute(&mut self, attribute: Attribute) {
        self.attributes.push(attribute);
    }
}

impl CalyxWriter for Control {
    fn write(&self, f: &mut IndentFormatter<'_, '_>) -> fmt::Result {
        self.attributes.write(f)?;
        match &self.value {
            ControlValue::Empty => {}
            ControlValue::Enable(group) => {
                write!(f, "{};", group.borrow().name)?;
            }
            ControlValue::Seq(body) => {
                writeln!(f, "seq {{")?;
                f.increase_indent();
                for node in body {
                    node.writeln(f)?;
                }
                f.decrease_indent();
                write!(f, "}}")?;
            }
            ControlValue::Par(body) => {
                writeln!(f, "par {{")?;
                f.increase_indent();
                for node in body {
                    node.writeln(f)?;
                }
                f.decrease_indent();
                write!(f, "}}")?;
            }
            ControlValue::While(cond, group, body) => {
                writeln!(
                    f,
                    "while {}{} {{",
                    cond,
                    if let Some(group) = group {
                        format!(" with {}", group.borrow().name)
                    } else {
                        "".into()
                    }
                )?;
                f.increase_indent();
                for node in body {
                    node.writeln(f)?;
                }
                f.decrease_indent();
                write!(f, "}}")?;
            }
            ControlValue::If(cond, group, body_true, body_false) => {
                writeln!(
                    f,
                    "if {}{} {{",
                    cond,
                    if let Some(group) = group {
                        format!(" with {}", group.borrow().name)
                    } else {
                        "".into()
                    }
                )?;
                f.increase_indent();
                for node in body_true {
                    node.writeln(f)?;
                }
                f.decrease_indent();
                writeln!(f, "}} else {{")?;
                f.increase_indent();
                for node in body_false {
                    node.writeln(f)?;
                }
                f.decrease_indent();
                write!(f, "}}")?;
            }
        }
        Ok(())
    }
}

/// See `calyx_ir::Component`.
pub struct Component {
    attributes: Attributes,
    is_comb: bool,
    name: String,
    inputs: Vec<Port>,
    outputs: Vec<Port>,
    cells: Vec<RRC<Cell>>,
    groups: Vec<RRC<Group>>,
    control: Control,
    continuous_assignments: Vec<Assignment>,
}

impl Component {
    #[allow(clippy::single_element_loop)]
    fn new<S: AsRef<str>>(name: S, is_comb: bool) -> Self {
        let mut new_self = Self {
            attributes: vec![],
            is_comb,
            name: name.as_ref().to_string(),
            inputs: vec![],
            outputs: vec![],
            cells: vec![],
            groups: vec![],
            control: Control::empty(),
            continuous_assignments: vec![],
        };
        if is_comb {
            for input in [
                Port::new_in_comp("go", 1)
                    .with_attribute(Attribute::num("go", 1)),
                Port::new_in_comp("clk", 1)
                    .with_attribute(Attribute::bool("clk")),
                Port::new_in_comp("reset", 1)
                    .with_attribute(Attribute::bool("reset")),
            ] {
                new_self.inputs.push(input);
            }
            for output in [Port::new_in_comp("done", 1)
                .with_attribute(Attribute::num("done", 1))]
            {
                new_self.outputs.push(output);
            }
        }
        new_self
    }

    /// Adds an input port `name` to this component.
    pub fn add_input<S: AsRef<str>>(&mut self, name: S, width: u64) {
        self.inputs.push(Port::new_in_comp(name, width));
    }

    /// Adds an output port `name` to this component.
    pub fn add_output<S: AsRef<str>>(&mut self, name: S, width: u64) {
        self.outputs.push(Port::new_in_comp(name, width));
    }

    /// Constructs a new cell named `name` that instantiates `inst` with
    /// arguments `args`.
    pub fn cell<S: AsRef<str>, T: AsRef<str>>(
        &mut self,
        is_ref: bool,
        name: S,
        inst: T,
        args: Vec<u64>,
    ) -> RRC<Cell> {
        let cell = rrc(Cell {
            is_ref,
            attributes: vec![],
            name: name.as_ref().to_string(),
            inst: inst.as_ref().to_string(),
            args,
        });
        self.cells.push(cell.clone());
        cell
    }

    /// Establishes a continuous assignment guarded by `guard` between `lhs` and
    /// `rhs`.
    pub fn assign(&mut self, lhs: Port, rhs: Port, guard: Guard) {
        self.continuous_assignments
            .push(Assignment::new(lhs, rhs, guard));
    }

    /// Use [`declare_group!`] and [`build_group!`] instead.
    pub fn group<S: AsRef<str>>(&mut self, name: S) -> RRC<Group> {
        let group = rrc(Group::new(name, false));
        self.groups.push(group.clone());
        group
    }

    /// Use [`declare_group!`] and [`build_group!`] instead.
    pub fn comb_group<S: AsRef<str>>(&mut self, name: S) -> RRC<Group> {
        let group = rrc(Group::new(name, true));
        self.groups.push(group.clone());
        group
    }

    /// Sets the root control node for this component. Use [`build_control!`] to
    /// construct this node.
    pub fn set_control(&mut self, control: Control) {
        self.control = control;
    }

    /// Opens an indented section within a component and runs a callback after
    /// applying another indent. For instance, when called with a `name` of
    /// "cells", this function allows formatting of:
    /// ```
    /// ...
    ///   cells {
    ///     ...
    ///   }
    /// ...
    /// ```
    fn brace_section<S: AsRef<str>, F>(
        &self,
        f: &mut IndentFormatter<'_, '_>,
        name: S,
        body: F,
    ) -> fmt::Result
    where
        F: FnOnce(&mut IndentFormatter<'_, '_>) -> fmt::Result,
    {
        f.increase_indent();
        writeln!(f, "{} {{", name.as_ref(),)?;
        f.increase_indent();
        body(f)?;
        f.decrease_indent();
        writeln!(f, "}}")?;
        f.decrease_indent();
        Ok(())
    }

    /// Formats a list of ports as an input or output signature. For example,
    /// the syntax for ports in the declaration of a component with name-width
    /// pairs separated by colons is such a signature.
    ///
    /// Requires: all ports in `ports` must have definite widths.
    fn write_ports_sig(
        &self,
        f: &mut IndentFormatter<'_, '_>,
        ports: &[Port],
    ) -> fmt::Result {
        for (i, port) in ports.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            assert!(!port.has_inferred_width());
            port.attributes.write(f)?;
            write!(f, "{}: {}", port.name, port.width.unwrap())?;
        }
        Ok(())
    }
}

impl PortProvider for Component {
    fn get_port(&self, port: String) -> Port {
        self.inputs
            .iter()
            .chain(self.outputs.iter())
            .find(|p| p.name == port)
            .unwrap_or_else(|| {
                panic!("{}", "port does not exist, violating precondition")
            })
            .clone()
    }
}

impl AttributeProvider for Component {
    fn add_attribute(&mut self, attribute: Attribute) {
        self.attributes.push(attribute);
    }
}

impl CalyxWriter for Component {
    fn marker(&self) -> Option<Marker> {
        Some(Marker::unique("component", self.name.clone()))
    }

    fn write(&self, f: &mut IndentFormatter<'_, '_>) -> fmt::Result {
        self.attributes.write(f)?;
        write!(
            f,
            "{}component {}(",
            if self.is_comb { "comb " } else { "" },
            self.name
        )?;
        self.write_ports_sig(f, &self.inputs)?;
        write!(f, ") -> (")?;
        self.write_ports_sig(f, &self.outputs)?;
        writeln!(f, ") {{")?;
        self.brace_section(f, "cells", |f| {
            for cell in &self.cells {
                cell.borrow().writeln(f)?;
            }
            Ok(())
        })?;
        self.brace_section(f, "wires", |f| {
            for group in &self.groups {
                group.borrow().writeln(f)?;
            }
            for assignment in &self.continuous_assignments {
                assignment.writeln(f)?;
            }
            Ok(())
        })?;
        self.brace_section(f, "control", |f| self.control.writeln(f))?;
        write!(f, "}}")
    }
}

/// A complete calyx program containing imports and components. To obtain the
/// generated calyx as text, use the [`Program::to_string`] function.
#[derive(Default)]
pub struct Program {
    imports: Vec<Import>,
    /// inv: no element is removed from this array
    comps: Vec<RRC<Component>>,
}

impl Program {
    /// Constructs an empty program.
    pub fn new() -> Self {
        Self::default()
    }

    /// Imports the calyx standard library file `path`.
    ///
    /// Requires: `path` is a well-formed path.
    pub fn import<S: AsRef<str>>(&mut self, path: S) {
        self.imports.push(Import {
            path: PathBuf::from_str(path.as_ref())
                .expect("malformed input path"),
        });
    }

    /// Constructs an empty component named `name`.
    pub fn comp<S: AsRef<str>>(&mut self, name: S) -> RRC<Component> {
        let comp = rrc(Component::new(name, false));
        self.comps.push(comp.clone());
        comp
    }

    /// Constructs an empty combinational component named `name`.
    pub fn comb_comp<S: AsRef<str>>(&mut self, name: S) -> RRC<Component> {
        let comp = rrc(Component::new(name, true));
        self.comps.push(comp.clone());
        comp
    }
}

impl CalyxWriter for Program {
    fn write(&self, f: &mut IndentFormatter<'_, '_>) -> fmt::Result {
        for import in &self.imports {
            import.writeln(f)?;
        }
        for comp in &self.comps {
            comp.borrow().writeln(f)?;
        }
        Ok(())
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = IndentFormatter::new(f, 2);
        self.write(&mut f)
    }
}

/// Static assertion.
#[macro_export]
macro_rules! const_assert {
    ($($tt:tt)*) => {
        const _: () = assert!($($tt)*);
    }
}

/// Constructs static and dynamic assertions for the given primitive, if a
/// verification case exists. Due to limiations (https://github.com/rust-lang/rust/issues/85077)
/// we have to add a dummy value to the beginning of the input array, so the
/// logic must be modified accordingly (and annoyingly).
#[macro_export]
macro_rules! _validate_primitive {
    (std_reg($arg_arr:expr)) => {
        const_assert!(
            $arg_arr.len() - 1 == 1,
            "Invalid std_reg instantiation: std_reg takes 1 argument"
        );
    };
    (std_add($arg_arr:expr)) => {
        const_assert!(
            $arg_arr.len() - 1 == 1,
            "Invalid std_add instantiation: std_add takes 1 argument"
        );
    };
    (comb_mem_d1($arg_arr:expr)) => {
        const_assert!(
            $arg_arr.len() - 1 == 3,
            "Invalid comb_mem_d1 instantiation: comb_mem_d1 takes 3 arguments"
        );
    };
    (seq_mem_d1($arg_arr:expr)) => {
        const_assert!(
            $arg_arr.len() - 1 == 3,
            "Invalid seq_mem_d1 instantiation: seq_mem_d1 takes 3 arguments"
        );
    };
    (std_bit_slice($arg_arr:expr)) => {
        const_assert!(
            $arg_arr.len() - 1 == 4,
            "Invalid std_bit_slice instantiation: std_bit_slice takes 4 arguments"
        );
        assert!($arg_arr[4] >= $arg_arr[1], "Invalid std_bit_slice instantiation: out_width <= in_width");
        assert!($arg_arr[4] == $arg_arr[3] - $arg_arr[2], "Invalid std_bit_slice instantiation: out_width must be end_idx - start_idx")
    };
    ($idc:ident($idc2:expr)) => {};
}

/// Similar to the `structure!` macro in `calyx_ir`. For example,
/// ```
/// build_cells!(comp;
///     a = std_reg(32);
///     ref b = std_add(32);
/// )
/// ```
/// Remember to import (via [`Program::import`]) the necessary primitives.
#[macro_export]
macro_rules! build_cells {
    ($comp:ident; ref $name:ident = $cell:ident($($args:expr),*); $($rest:tt)*) => {
        _validate_primitive!($cell([0 as u64, $($args as u64),*]));
        let $name = $comp.borrow_mut().cell(true, stringify!($name), stringify!($cell), vec![$($args as u64),*]);
        build_cells!($comp; $($rest)*);
    };
    ($comp:ident; $name:ident = $cell:ident($($args:expr),*); $($rest:tt)*) => {
        _validate_primitive!($cell([0 as u64, $($args as u64),*]));
        let $name = $comp.borrow_mut().cell(false, stringify!($name), stringify!($cell), vec![$($args as u64),*]);
        build_cells!($comp; $($rest)*);
    };
    ($comp:ident;) => {};
}

/// `declare_group!(comp; group name)` declares `name` as a group and binds it
/// as a variable. `declare_group!(comp; comb group name)` behaves identically
/// but constructs a combinational group. In both cases, an optional `: "..."`
/// can be places after the group name (`name`) to provide it a description.
#[macro_export]
macro_rules! declare_group {
    ($comp:expr; group $group:ident) => {
        let $group = $comp.borrow_mut().group(stringify!($group));
    };
    ($comp:expr; group $group:ident: $desc:expr) => {
        let $group = $comp.borrow_mut().group(stringify!($group));
        $group.borrow_mut().describe($desc.to_string());
    };
    ($comp:expr; comb group $group:ident) => {
        let $group = $comp.borrow_mut().comb_group(stringify!($group));
    };
    ($comp:expr; comb group $group:ident: $desc:expr) => {
        let $group = $comp.borrow_mut().comb_group(stringify!($group));
        $group.borrow_mut().describe($desc.to_string());
    };
}

/// Adds assignments to a group. For example,
/// ```
/// build_group!(group;
///     a.addr0 = b.out;
///     b.foo = a.bar;
///     comp.in_port = x.foo;
/// )
/// ```
/// Currently, only unguarded assignments are supported. (TODO(ethan))
#[macro_export]
macro_rules! build_group {
    ($group:expr; $($lhs:ident.$lhs_port:ident = $rhs:ident.$rhs_port:ident;)*) => {
        $(
            $group.borrow_mut().assign(
                $lhs.borrow().get(stringify!($lhs_port).to_string()),
                $rhs.borrow().get(stringify!($rhs_port).to_string()),
            );
    )*
    };
}

/// Recursively constructs control nodes. You can use calyx syntax or embed
/// arbitrary expressions that are typed `Control`. For example,
/// ```
/// let control = build_control!(
///     [par {
///         [if comp.read_en {
///             [if tag_matches.out with check_tag_matches {
///                 [read_cached]
///             } else {
///                 [read_uncached]
///             }]
///         }],
///         [if comp.write_en {
///             [(Control::empty())]
///         }]
///     }]
/// );
/// ```
#[macro_export]
macro_rules! build_control {
    ([$x:ident]) => {
        $crate::Control::enable($x.clone())
    };
    (($c:expr)) => {
        $c
    };
    ([seq { $($x:tt),+ }]) => {
        $crate::Control::seq(vec![$(build_control!($x)),*])
    };
    ([par { $($x:tt),+ }]) => {
        $crate::Control::par(vec![$(build_control!($x)),*])
    };
    ([while $cond:ident.$port:ident { $($x:tt),* }]) => {
        $crate::Control::while_($cond.borrow().get(stringify!($port).into()), None, vec![$(build_control!($x)),*])
    };
    ([while $cond:ident.$port:ident with $comb_group:ident { $($x:tt),* }]) => {
        $crate::Control::while_($cond.borrow().get(stringify!($port).into()), Some($comb_group), vec![$(build_control!($x)),*])
    };
    ([if $cond:ident.$port:ident { $($x_true:tt),* }]) => {
        $crate::Control::if_($cond.borrow().get(stringify!($port).into()), None, vec![$(build_control!($x_true)),*], vec![])
    };
    ([if $cond:ident.$port:ident { $($x_true:tt),* } else { $($x_false:tt),* }]) => {
        $crate::Control::if_($cond.borrow().get(stringify!($port).into()), None, vec![$(build_control!($x_true)),*], vec![$(build_control!($x_false)),*])
    };
    ([if $cond:ident.$port:ident with $comb_group:ident { $($x_true:tt),* }=]) => {
        $crate::Control::if_($cond.borrow().get(stringify!($port).into()), Some($comb_group.clone()), vec![$(build_control!($x_true)),*], vec![])
    };
    ([if $cond:ident.$port:ident with $comb_group:ident { $($x_true:tt),* } else { $($x_false:tt),* }]) => {
        $crate::Control::if_($cond.borrow().get(stringify!($port).into()), Some($comb_group.clone()), vec![$(build_control!($x_true)),*], vec![$(build_control!($x_false)),*])
    };
}
