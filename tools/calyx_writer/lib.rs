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
    rc::Rc,
};

/// Shorthand for the `Rc<RefCell<>>` pattern.
pub type RRC<T> = Rc<RefCell<T>>;

/// Convenience constructor for [`RRC`].
fn rrc<T>(value: T) -> RRC<T> {
    Rc::new(RefCell::new(value))
}

// TODO(ethan): too lazy to find a crate that handles this
struct Indent {
    current: String,
    add: usize,
}

impl Indent {
    fn push(&mut self) {
        for _ in 0..self.add {
            self.current.push(' ');
        }
    }

    fn pop(&mut self) {
        for _ in 0..self.add {
            self.current.pop();
        }
    }

    fn current(&self) -> &str {
        &self.current
    }
}

impl Default for Indent {
    fn default() -> Self {
        Indent {
            current: String::new(),
            add: 2,
        }
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
    pub fn general<S: ToString>(element: S) -> Marker {
        Marker {
            element: element.to_string(),
            id: None,
        }
    }

    /// A marker for a unique `element` identified by `id`.
    pub fn unique<S: ToString, T: ToString>(element: S, id: T) -> Marker {
        Marker {
            element: element.to_string(),
            id: Some(id.to_string()),
        }
    }

    /// Constructs a comment string for the marker at a given `location`. For
    /// example, `marker.to_string("end")`.
    pub fn to_string<S: ToString>(&self, location: S) -> String {
        format!(
            "// {} {}{}\n",
            self.element.to_ascii_uppercase(),
            location.to_string().to_ascii_uppercase(),
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

    /// Writes this element to `f`. It may be assumed that `indent.current()`
    /// spaces have already been written, but on any further line, these number
    /// of spaces must be effected. See also [`CalyxWriter::writeln`].
    fn write(
        &self,
        f: &mut fmt::Formatter<'_>,
        indent: &mut Indent,
    ) -> fmt::Result;

    /// Writes this element followed by a newline and wrapped with markers if
    /// the element provides them. See more information at
    /// [`CalyxWriter::write`].
    ///
    /// Do NOT override this function. See details at [`CalyxWriter`].
    fn writeln(
        &self,
        f: &mut fmt::Formatter<'_>,
        indent: &mut Indent,
    ) -> fmt::Result {
        if let Some(marker) = self.marker() {
            write!(f, "{}", marker.to_string("start"))?;
            self.write(f, indent)?;
            f.write_char('\n')?;
            write!(f, "{}", marker.to_string("end"))?;
        } else {
            self.write(f, indent)?;
            f.write_char('\n')?;
        }
        Ok(())
    }
}

/// A calyx import statement.
struct Import {
    path: String,
}

impl CalyxWriter for Import {
    fn write(
        &self,
        f: &mut fmt::Formatter<'_>,
        _indent: &mut Indent,
    ) -> fmt::Result {
        write!(f, "import \"{}\";", self.path)
    }
}

/// A clyx attribute.
#[derive(PartialEq, Eq, Clone)]
pub struct Attribute {
    name: String,
    value: Option<u64>,
}

impl Attribute {
    /// Constructs a `calyx_frontend::BoolAttr`, such as `@external`, named
    /// `name`.
    pub fn bool<S: ToString>(name: S) -> Self {
        Self {
            name: name.to_string(),
            value: None,
        }
    }

    /// Constructs a `calyx_frontend::NumAttr`, such as `@go`, named `name`.    
    pub fn num<S: ToString>(name: S, value: u64) -> Self {
        Self {
            name: name.to_string(),
            value: Some(value),
        }
    }
}

impl CalyxWriter for Attribute {
    fn write(
        &self,
        f: &mut fmt::Formatter<'_>,
        _indent: &mut Indent,
    ) -> fmt::Result {
        write!(f, "@{}", self.name)?;
        if let Some(value) = self.value {
            write!(f, "({})", value)?;
        }
        Ok(())
    }
}

/// A list of attributes.
type Attributes = Vec<Attribute>;

impl CalyxWriter for Attributes {
    fn write(
        &self,
        f: &mut fmt::Formatter<'_>,
        indent: &mut Indent,
    ) -> fmt::Result {
        for attr in self {
            attr.write(f, indent)?;
            f.write_char(' ')?;
        }
        Ok(())
    }
}

/// A calyx port on a cell or a component.
#[derive(PartialEq, Eq, Clone)]
pub struct Port {
    attributes: Vec<Attribute>,
    /// The cell that this port belongs to, or `None` if it is in a component
    /// signature.
    parent: Option<String>,
    name: String,
    /// A width of `-1` means the actual width is unknown/irrelevant/inferred.
    width: isize,
}

impl Port {
    /// Constructs a new port with the given `parent` and `name`.
    ///
    /// Requires: `width > 0`.
    pub fn new<S: ToString, T: ToString>(
        parent: Option<S>,
        name: T,
        width: usize,
    ) -> Self {
        assert!(width > 0);
        Self {
            attributes: vec![],
            parent: parent.map(|s| s.to_string()),
            name: name.to_string(),
            width: width as isize,
        }
    }

    /// Constructs a new port with the given `parent` and `name` and with
    /// inferred width.
    pub fn inferred<S: ToString, T: ToString>(
        parent: Option<S>,
        name: T,
    ) -> Self {
        Self {
            attributes: vec![],
            parent: parent.map(|s| s.to_string()),
            name: name.to_string(),
            width: -1,
        }
    }

    pub fn add_attribute(&mut self, attr: Attribute) {
        self.attributes.push(attr);
    }

    pub fn has_inferred_width(&self) -> bool {
        self.width == -1
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
    fn write(
        &self,
        f: &mut fmt::Formatter<'_>,
        _indent: &mut Indent,
    ) -> fmt::Result {
        self.fmt(f)
    }
}

/// Abstracts port functionality over components and cells.
pub trait PortProvider {
    /// Retrieves the port on this provider named `port`.
    ///
    /// Requires: `port` exists on this provider.
    fn get(&self, port: String) -> Port;
}

/// A calyx cell.
pub struct Cell {
    is_ref: bool,
    attributes: Vec<Attribute>,
    name: String,
    /// The name of the component or primitive instantiated, e.g., "std_reg"
    /// for the register primitive.
    inst: String,
    args: Vec<u64>,
}

impl PortProvider for Cell {
    fn get(&self, port: String) -> Port {
        Port::inferred(Some(self.name.clone()), port)
    }
}

impl CalyxWriter for Cell {
    fn write(
        &self,
        f: &mut fmt::Formatter<'_>,
        indent: &mut Indent,
    ) -> fmt::Result {
        self.attributes.write(f, indent)?;
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

/// A guard for a calyx [`Assignment`].
#[derive(PartialEq, Eq)]
pub enum Guard {
    None,
    Constant(u64),
    Port(Port),
    And(Box<Guard>, Box<Guard>),
}

impl CalyxWriter for Guard {
    fn write(
        &self,
        _f: &mut fmt::Formatter<'_>,
        _indent: &mut Indent,
    ) -> fmt::Result {
        // TODO
        Ok(())
    }
}

/// A calyx assignment.
pub struct Assignment {
    lhs: Port,
    rhs: Port,
    guard: Guard,
}

impl CalyxWriter for Assignment {
    fn write(
        &self,
        f: &mut fmt::Formatter<'_>,
        indent: &mut Indent,
    ) -> fmt::Result {
        write!(f, "{} = ", self.lhs)?;
        if self.guard != Guard::None {
            self.guard.write(f, indent)?;
            write!(f, " ? ")?;
        }
        write!(f, "{};", self.rhs)
    }
}

/// A calyx group.
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
    fn new<S: ToString>(name: S, is_comb: bool) -> Self {
        Self {
            name: name.to_string(),
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
        self.assign_guarded(lhs, rhs, Guard::None);
    }

    /// Adds an assignment guarded by `guard` between `lhs` and `rhs`.
    pub fn assign_guarded(&mut self, lhs: Port, rhs: Port, guard: Guard) {
        self.assignments.push(Assignment { lhs, rhs, guard });
    }
}

impl PortProvider for Group {
    fn get(&self, port: String) -> Port {
        Port::inferred(Some(self.name.clone()), port)
    }
}

impl CalyxWriter for Group {
    fn write(
        &self,
        f: &mut fmt::Formatter<'_>,
        indent: &mut Indent,
    ) -> fmt::Result {
        if let Some(description) = &self.description {
            for line in description.lines() {
                write!(f, "// {}\n{}", line, indent.current())?;
            }
        }
        if let Some(latency) = self.latency {
            write!(f, "static<{}> ", latency)?;
        }
        if self.is_comb {
            write!(f, "comb ")?;
        }
        writeln!(f, "group {} {{", self.name)?;
        indent.push();
        for assignment in &self.assignments {
            write!(f, "{}", indent.current())?;
            assignment.writeln(f, indent)?;
        }
        indent.pop();
        write!(f, "{}}}", indent.current())
    }
}

/// Helper variant with [`Control`].
enum ControlValue {
    Empty,
    Enable(RRC<Group>),
    Seq(Vec<Control>),
    Par(Vec<Control>),
    While(Port, Option<RRC<Group>>, Vec<Control>),
    If(Port, Option<RRC<Group>>, Vec<Control>, Vec<Control>),
}

/// Structured calyx control with attributes.
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

    pub fn add_attribute(&mut self, attribute: Attribute) {
        self.attributes.push(attribute);
    }
}

impl CalyxWriter for Control {
    fn write(
        &self,
        f: &mut fmt::Formatter<'_>,
        indent: &mut Indent,
    ) -> fmt::Result {
        self.attributes.write(f, indent)?;
        match &self.value {
            ControlValue::Empty => {}
            ControlValue::Enable(group) => {
                write!(f, "{};", group.borrow().name)?;
            }
            ControlValue::Seq(body) => {
                writeln!(f, "seq {{")?;
                indent.push();
                for node in body {
                    write!(f, "{}", indent.current())?;
                    node.writeln(f, indent)?;
                }
                indent.pop();
                write!(f, "{}}}", indent.current())?;
            }
            ControlValue::Par(body) => {
                writeln!(f, "par {{")?;
                indent.push();
                for node in body {
                    write!(f, "{}", indent.current())?;
                    node.writeln(f, indent)?;
                }
                indent.pop();
                write!(f, "{}}}", indent.current())?;
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
                indent.push();
                for node in body {
                    write!(f, "{}", indent.current())?;
                    node.writeln(f, indent)?;
                }
                indent.pop();
                write!(f, "{}}}", indent.current())?;
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
                indent.push();
                for node in body_true {
                    write!(f, "{}", indent.current())?;
                    node.writeln(f, indent)?;
                }
                indent.pop();
                writeln!(f, "{}}} else {{", indent.current())?;
                indent.push();
                for node in body_false {
                    write!(f, "{}", indent.current())?;
                    node.writeln(f, indent)?;
                }
                indent.pop();
                write!(f, "{}}}", indent.current())?;
            }
        }
        Ok(())
    }
}

/// A calyx component.
pub struct Component {
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
    /// Sets whether the component is combinational or not.
    pub fn set_comb(&mut self, is_comb: bool) {
        self.is_comb = is_comb;
    }

    /// Adds an input port `name` to this component.
    pub fn add_input<S: ToString>(&mut self, name: S, width: usize) {
        self.add_port(Port::new::<String, S>(None, name, width), true);
    }

    /// Adds an output port `name` to this component.
    pub fn add_output<S: ToString>(&mut self, name: S, width: usize) {
        self.add_port(Port::new::<String, S>(None, name, width), false);
    }

    /// [`Component::add_input`] or [`Component::add_output`] may be more
    /// useful.
    pub fn add_port(&mut self, port: Port, is_input: bool) {
        if is_input {
            self.inputs.push(port);
        } else {
            self.outputs.push(port);
        }
    }

    /// Constructs a new cell named `name` that instatiates `inst` with
    /// arguments `args`.
    pub fn cell<S: ToString, T: ToString>(
        &mut self,
        name: S,
        inst: T,
        args: Vec<u64>,
    ) -> RRC<Cell> {
        let cell = rrc(Cell {
            is_ref: false,
            attributes: vec![],
            name: name.to_string(),
            inst: inst.to_string(),
            args,
        });
        self.cells.push(cell.clone());
        cell
    }

    /// Establishes a continuous assignment guarded by `guard` between `lhs` and
    /// `rhs`.
    pub fn assign(&mut self, lhs: Port, rhs: Port, guard: Guard) {
        self.continuous_assignments
            .push(Assignment { lhs, rhs, guard });
    }

    /// Use [`declare_group!`] and [`build_group!`] instead.
    pub fn group<S: ToString>(&mut self, name: S) -> RRC<Group> {
        let group = rrc(Group::new(name, false));
        self.groups.push(group.clone());
        group
    }

    /// Use [`declare_group!`] and [`build_group!`] instead.
    pub fn comb_group<S: ToString>(&mut self, name: S) -> RRC<Group> {
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
    fn brace_section<S: ToString, F>(
        &self,
        f: &mut fmt::Formatter,
        indent: &mut Indent,
        name: S,
        body: F,
    ) -> fmt::Result
    where
        F: FnOnce(&mut fmt::Formatter, &mut Indent) -> fmt::Result,
    {
        indent.push();
        writeln!(f, "{}{} {{", indent.current(), name.to_string(),)?;
        indent.push();
        let result = body(f, indent);
        indent.pop();
        writeln!(f, "{}}}", indent.current())?;
        indent.pop();
        result
    }

    /// Formats a list of ports as an input or output signature. For example,
    /// the syntax for ports in the declaration of a component with name-width
    /// pairs separated by colons is such a signature.
    ///
    /// Requires: all ports in `ports` must have definite widths.
    fn write_ports_sig(
        &self,
        f: &mut fmt::Formatter,
        ports: &Vec<Port>,
    ) -> fmt::Result {
        for (i, port) in ports.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            assert!(!port.has_inferred_width());
            write!(f, "{}: {}", port.name, port.width)?;
        }
        Ok(())
    }
}

impl PortProvider for Component {
    fn get(&self, port: String) -> Port {
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

impl CalyxWriter for Component {
    fn marker(&self) -> Option<Marker> {
        Some(Marker::unique("component", self.name.clone()))
    }

    fn write(
        &self,
        f: &mut fmt::Formatter<'_>,
        indent: &mut Indent,
    ) -> fmt::Result {
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
        self.brace_section(f, indent, "cells", |f, indent| {
            for cell in &self.cells {
                write!(f, "{}", indent.current())?;
                cell.borrow().writeln(f, indent)?;
            }
            Ok(())
        })?;
        self.brace_section(f, indent, "wires", |f, indent| {
            for group in &self.groups {
                write!(f, "{}", indent.current())?;
                group.borrow().writeln(f, indent)?;
            }
            for assignment in &self.continuous_assignments {
                write!(f, "{}", indent.current())?;
                assignment.writeln(f, indent)?;
            }
            Ok(())
        })?;
        self.brace_section(f, indent, "control", |f, indent| {
            write!(f, "{}", indent.current())?;
            self.control.writeln(f, indent)
        })?;
        write!(f, "}}")
    }
}

/// A complete calyx program containing imports and components.
#[derive(Default)]
pub struct Program {
    imports: Vec<Import>,
    /// inv: no element is removed from this array
    comps: Vec<Rc<RefCell<Component>>>,
}

impl Program {
    /// Constructs an empty program.
    pub fn new() -> Self {
        Self {
            imports: vec![],
            comps: vec![],
        }
    }

    /// Imports the calyx standard library file `path`.
    pub fn import<S: ToString>(&mut self, path: S) {
        self.imports.push(Import {
            path: path.to_string(),
        });
    }

    /// Constructs an empty component named `name`.
    pub fn comp<S: ToString>(&mut self, name: S) -> RRC<Component> {
        let comp = rrc(Component {
            is_comb: false,
            name: name.to_string(),
            inputs: vec![],
            outputs: vec![],
            cells: vec![],
            groups: vec![],
            control: Control::empty(),
            continuous_assignments: vec![],
        });
        self.comps.push(comp.clone());
        comp
    }
}

impl CalyxWriter for Program {
    fn write(
        &self,
        f: &mut fmt::Formatter<'_>,
        indent: &mut Indent,
    ) -> fmt::Result {
        for import in &self.imports {
            import.writeln(f, indent)?;
        }
        for comp in &self.comps {
            comp.borrow().writeln(f, indent)?;
        }
        Ok(())
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write(f, &mut Indent::default())
    }
}

/// Similar to the `structure!` macro in `calyx_ir`.
#[macro_export]
macro_rules! build_cells {
    ($comp:ident; $($name:ident = $cell:ident($($args:expr),*);)*) => {
        $(
            let $name = $comp.borrow_mut().cell(stringify!($name), stringify!($cell), vec![$($args as u64),*]);
        )*
    };
}

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

#[macro_export]
macro_rules! build_group {
    ($group:ident; $($lhs:ident.$lhs_port:ident = $rhs:ident.$rhs_port:ident;)*) => {
        $(
            $group.borrow_mut().assign(
                $lhs.borrow().get(stringify!($lhs_port).to_string()),
                $rhs.borrow().get(stringify!($rhs_port).to_string()),
            );
    )*
    };
}

#[macro_export]
macro_rules! build_control {
    ([$x:ident]) => {
        Control::enable($x.clone())
    };
    ([seq { $($x:tt),+ }]) => {
        Control::seq(vec![$(build_control!($x)),*])
    };
    ([par { $($x:tt),+ }]) => {
        Control::par(vec![$(build_control!($x)),*])
    };
    ([while $cond:ident.$port:ident { $($x:tt),* }]) => {
        Control::while_($cond.borrow().get(stringify!($port).into()), None, vec![$(build_control!($x)),*])
    };
    ([while $cond:ident.$port:ident with $comb_group:ident { $($x:tt),* }]) => {
        Control::while_($cond.borrow().get(stringify!($port).into()), Some($comb_group), vec![$(build_control!($x)),*])
    };
    ([if $cond:ident.$port:ident { $($x_true:tt),* }]) => {
        Control::if_($cond.borrow().get(stringify!($port).into()), None, vec![$(build_control!($x_true)),*], vec![])
    };
    ([if $cond:ident.$port:ident { $($x_true:tt),* } else { $($x_false:tt),* }]) => {
        Control::if_($cond.borrow().get(stringify!($port).into()), None, vec![$(build_control!($x_true)),*], vec![$(build_control!($x_false)),*])
    };
    ([if $cond:ident.$port:ident with $comb_group:ident { $($x_true:tt),* }=]) => {
        Control::if_($cond.borrow().get(stringify!($port).into()), Some($comb_group.clone()), vec![$(build_control!($x_true)),*], vec![])
    };
    ([if $cond:ident.$port:ident with $comb_group:ident { $($x_true:tt),* } else { $($x_false:tt),* }]) => {
        Control::if_($cond.borrow().get(stringify!($port).into()), Some($comb_group.clone()), vec![$(build_control!($x_true)),*], vec![$(build_control!($x_false)),*])
    };
}
