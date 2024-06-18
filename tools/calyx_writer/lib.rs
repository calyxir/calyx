// TODO(ethan): document everything & make code good

use std::{
    cell::RefCell,
    fmt::{self, Display, Write},
    rc::Rc,
};

pub type RRC<T> = Rc<RefCell<T>>;

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

struct Marker {
    name: String,
    id: Option<String>,
}

impl Marker {
    pub fn general<S: ToString>(name: S) -> Marker {
        Marker {
            name: name.to_string(),
            id: None,
        }
    }

    pub fn unique<S: ToString, T: ToString>(name: S, id: T) -> Marker {
        Marker {
            name: name.to_string(),
            id: Some(id.to_string()),
        }
    }

    pub fn to_string<S: ToString>(&self, descriptor: S) -> String {
        format!(
            "// {} {}{}\n",
            self.name.to_ascii_uppercase(),
            descriptor.to_string(),
            self.id
                .as_ref()
                .map(|id| format!(": {}", id))
                .unwrap_or_default()
        )
    }
}

trait CalyxWriter {
    fn marker(&self) -> Option<Marker> {
        return None;
    }

    fn write(
        &self,
        f: &mut fmt::Formatter<'_>,
        indent: &mut Indent,
    ) -> fmt::Result;

    fn writeln(
        &self,
        f: &mut fmt::Formatter<'_>,
        indent: &mut Indent,
    ) -> fmt::Result {
        if let Some(marker) = self.marker() {
            write!(f, "{}", marker.to_string("START"))?;
            self.write(f, indent)?;
            f.write_char('\n')?;
            write!(f, "{}", marker.to_string("END"))?;
        } else {
            self.write(f, indent)?;
            f.write_char('\n')?;
        }
        Ok(())
    }
}

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

#[derive(PartialEq, Eq, Clone)]
pub struct Attribute {
    name: String,
    value: Option<u64>,
}

impl Attribute {
    pub fn bool<S: ToString>(name: S) -> Self {
        Self {
            name: name.to_string(),
            value: None,
        }
    }

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

#[derive(PartialEq, Eq, Clone)]
pub struct Port {
    attributes: Vec<Attribute>,
    parent: Option<String>,
    name: String,
    // -1 = unkown
    width: isize,
}

impl Port {
    pub fn new<S: ToString, T: ToString>(
        parent: Option<S>,
        name: T,
        width: usize,
    ) -> Self {
        Self {
            attributes: vec![],
            parent: parent.map(|s| s.to_string()),
            name: name.to_string(),
            width: width as isize,
        }
    }

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
}

type Ports = Vec<Port>;

pub trait PortProvider {
    fn get(&self, port: String) -> Port;
}

impl CalyxWriter for Ports {
    fn write(
        &self,
        f: &mut fmt::Formatter<'_>,
        indent: &mut Indent,
    ) -> fmt::Result {
        for (i, port) in self.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            port.write(f, indent)?;
        }
        Ok(())
    }
}

impl CalyxWriter for Port {
    fn write(
        &self,
        f: &mut fmt::Formatter<'_>,
        indent: &mut Indent,
    ) -> fmt::Result {
        self.attributes.write(f, indent)?;
        write!(f, "{}: {}", self.name, self.width)
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

pub struct Cell {
    is_ref: bool,
    attributes: Vec<Attribute>,
    name: String,
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

pub struct Group {
    name: String,
    description: Option<String>,
    is_comb: bool,
    latency: Option<usize>,
    assignments: Vec<Assignment>,
}

impl Group {
    fn new<S: ToString>(name: S, is_comb: bool) -> Self {
        Self {
            name: name.to_string(),
            description: None,
            is_comb,
            latency: None,
            assignments: vec![],
        }
    }

    pub fn describe(&mut self, description: String) {
        self.description = Some(description);
    }

    pub fn set_latency(&mut self, latency: usize) {
        self.latency = Some(latency);
    }

    pub fn is_comb(&self) -> bool {
        self.is_comb
    }

    pub fn assign(&mut self, lhs: Port, rhs: Port) {
        self.assign_guarded(lhs, rhs, Guard::None);
    }

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

pub enum ControlValue {
    Empty,
    Enable(RRC<Group>),
    Seq(Vec<Control>),
    Par(Vec<Control>),
    While(Port, Option<RRC<Group>>, Vec<Control>),
    If(Port, Option<RRC<Group>>, Vec<Control>, Vec<Control>),
}

pub struct Control {
    attributes: Attributes,
    value: ControlValue,
}

impl Control {
    pub fn enable(group: RRC<Group>) -> Self {
        Control {
            attributes: Attributes::new(),
            value: ControlValue::Enable(group),
        }
    }

    pub fn seq(controls: Vec<Control>) -> Self {
        Control {
            attributes: Attributes::new(),
            value: ControlValue::Seq(controls),
        }
    }

    pub fn par(controls: Vec<Control>) -> Self {
        Control {
            attributes: Attributes::new(),
            value: ControlValue::Par(controls),
        }
    }

    pub fn while_(
        port: Port,
        group: Option<RRC<Group>>,
        controls: Vec<Control>,
    ) -> Self {
        Control {
            attributes: Attributes::new(),
            value: ControlValue::While(port, group, controls),
        }
    }

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

pub struct Component {
    is_comb: bool,
    name: String,
    inputs: Ports,
    outputs: Ports,
    cells: Vec<RRC<Cell>>,
    groups: Vec<RRC<Group>>,
    control: Control,
    continuous_assignments: Vec<Assignment>,
}

impl Component {
    pub fn set_comb(&mut self, is_comb: bool) {
        self.is_comb = is_comb;
    }

    pub fn add_input<S: ToString>(&mut self, name: S, width: usize) {
        self.add_port(Port::new::<String, S>(None, name, width), true);
    }

    pub fn add_output<S: ToString>(&mut self, name: S, width: usize) {
        self.add_port(Port::new::<String, S>(None, name, width), false);
    }

    pub fn add_port(&mut self, port: Port, is_input: bool) {
        if is_input {
            self.inputs.push(port);
        } else {
            self.outputs.push(port);
        }
    }

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

    pub fn assign(&mut self, lhs: Port, rhs: Port, guard: Guard) {
        self.continuous_assignments
            .push(Assignment { lhs, rhs, guard });
    }

    pub fn group<S: ToString>(&mut self, name: S) -> RRC<Group> {
        let group = rrc(Group::new(name, false));
        self.groups.push(group.clone());
        group
    }

    pub fn comb_group<S: ToString>(&mut self, name: S) -> RRC<Group> {
        let group = rrc(Group::new(name, true));
        self.groups.push(group.clone());
        group
    }

    pub fn set_control(&mut self, control: Control) {
        self.control = control;
    }

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
}

impl PortProvider for Component {
    fn get(&self, port: String) -> Port {
        self.inputs
            .iter()
            .chain(self.outputs.iter())
            .find(|p| p.name == port)
            .expect("port does not exist, violating precondition".into())
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
        self.inputs.write(f, indent)?;
        write!(f, ") -> (")?;
        self.outputs.write(f, indent)?;
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

#[derive(Default)]
pub struct Program {
    imports: Vec<Import>,
    // inv: no element is removed from this array
    comps: Vec<Rc<RefCell<Component>>>,
}

impl Program {
    pub fn new() -> Self {
        Self {
            imports: vec![],
            comps: vec![],
        }
    }

    pub fn import<S: ToString>(&mut self, path: S) {
        self.imports.push(Import {
            path: path.to_string(),
        });
    }

    pub fn comp<S: ToString>(&mut self, name: S) -> RRC<Component> {
        let comp = rrc(Component {
            is_comb: false,
            name: name.to_string(),
            inputs: vec![],
            outputs: vec![],
            cells: vec![],
            groups: vec![],
            control: Control::seq(vec![]),
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
