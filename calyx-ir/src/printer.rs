//! Implements a formatter for the in-memory representation of Components.
//! The printing operation clones inner nodes and doesn't perform any mutation
//! to the Component.
use itertools::Itertools;

use crate::control::StaticInvoke;
use crate::{self as ir, RRC};
use std::io;
use std::path::Path;
use std::rc::Rc;

/// Printer for the IR.
pub struct Printer;

impl Printer {
    /// Format attributes of the form `@static(1)`.
    /// Returns the empty string if the `attrs` is empty.
    fn format_at_attributes(attrs: &ir::Attributes) -> String {
        let mut buf = String::new();
        for (name, val) in attrs {
            if *val == 1 {
                buf.push_str(&format!("@{name} "));
            } else {
                buf.push_str(&format!("@{name}({val}) "));
            }
        }
        buf
    }

    /// Format attributes of the form `<"static"=1>`.
    /// Returns the empty string if the `attrs` is empty.
    fn format_attributes(attrs: &ir::Attributes) -> String {
        if attrs.is_empty() {
            "".to_string()
        } else {
            format!(
                "<{}>",
                attrs
                    .into_iter()
                    .map(|(k, v)| { format!("\"{}\"={}", k, v) })
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    /// Formats port definitions in signatures
    fn format_ports(ports: &[RRC<ir::Port>]) -> String {
        ports
            .iter()
            .map(|p| {
                format!(
                    "{}{}: {}",
                    Self::format_at_attributes(&p.borrow().attributes),
                    p.borrow().name.id,
                    p.borrow().width
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn format_port_def<W: std::fmt::Display>(
        port_defs: &[&ir::PortDef<W>],
    ) -> String {
        port_defs
            .iter()
            .map(|pd| {
                format!(
                    "{}{}: {}",
                    Self::format_at_attributes(&pd.attributes),
                    pd.name,
                    pd.width
                )
            })
            .collect_vec()
            .join(", ")
    }

    /// Prints out the program context.
    /// `skip_primitives` controls whether the primitives are printed out.
    pub fn write_context<F: io::Write>(
        ctx: &ir::Context,
        skip_primitives: bool,
        f: &mut F,
    ) -> io::Result<()> {
        if !skip_primitives {
            for (path, prims) in ctx.lib.all_prims() {
                match path {
                    Some(p) => {
                        ir::Printer::write_externs(
                            (&p, prims.into_iter().map(|(_, v)| v)),
                            f,
                        )?;
                    }
                    None => {
                        for (_, prim) in prims {
                            ir::Printer::write_primitive(prim, 2, f)?;
                        }
                    }
                }
            }
        }
        for comp in &ctx.components {
            ir::Printer::write_component(comp, f)?;
            writeln!(f)?
        }
        write!(f, "{}", ir::Printer::format_metadata(&ctx.metadata))
    }

    /// Formats and writes extern statements.
    pub fn write_externs<'a, F, I>(
        (path, prims): (&Path, I),
        f: &mut F,
    ) -> io::Result<()>
    where
        F: io::Write,
        I: Iterator<Item = &'a ir::Primitive>,
    {
        writeln!(f, "extern \"{}\" {{", path.to_string_lossy())?;
        for prim in prims {
            Self::write_primitive(prim, 2, f)?;
        }
        writeln!(f, "}}")
    }

    pub fn write_primitive<F: io::Write>(
        prim: &ir::Primitive,
        indent_level: usize,
        f: &mut F,
    ) -> io::Result<()> {
        write!(f, "{}", " ".repeat(indent_level))?;
        if prim.is_comb {
            write!(f, "comb ")?;
        }
        write!(
            f,
            "primitive {}{}",
            prim.name,
            Self::format_attributes(&prim.attributes)
        )?;
        if !prim.params.is_empty() {
            write!(
                f,
                "[{}]",
                prim.params
                    .iter()
                    .map(|p| p.to_string())
                    .collect_vec()
                    .join(", ")
            )?
        }
        let (mut inputs, mut outputs) = (vec![], vec![]);
        for pd in &prim.signature {
            if pd.direction == ir::Direction::Input {
                inputs.push(pd)
            } else {
                outputs.push(pd)
            }
        }
        write!(
            f,
            "({}) -> ({})",
            Self::format_port_def(&inputs),
            Self::format_port_def(&outputs)
        )?;
        if prim.body.is_some() {
            write!(f, "{{ {} }}", prim.body.as_ref().unwrap())?;
        };
        writeln!(f, ";")
    }

    /// Formats and writes the Component to the formatter.
    pub fn write_component<F: io::Write>(
        comp: &ir::Component,
        f: &mut F,
    ) -> io::Result<()> {
        let sig = comp.signature.borrow();
        let (inputs, outputs): (Vec<_>, Vec<_>) =
            sig.ports.iter().map(Rc::clone).partition(|p| {
                // Cell signature stores the ports in reversed direction.
                matches!(p.borrow().direction, ir::Direction::Output)
            });

        let pre = if comp.is_comb { "comb " } else { "" };

        writeln!(
            f,
            "{}component {}{}({}) -> ({}) {{",
            pre,
            comp.name.id,
            Self::format_attributes(&comp.attributes),
            Self::format_ports(&inputs),
            Self::format_ports(&outputs),
        )?;

        // Add the cells
        writeln!(f, "  cells {{")?;
        for cell in comp.cells.iter() {
            Self::write_cell(&cell.borrow(), 4, f)?;
        }
        writeln!(f, "  }}")?;

        // Add the wires
        writeln!(f, "  wires {{")?;
        for group in comp.get_groups().iter() {
            Self::write_group(&group.borrow(), 4, f)?;
            writeln!(f)?;
        }
        for group in comp.get_static_groups().iter() {
            Self::write_static_group(&group.borrow(), 4, f)?;
            writeln!(f)?;
        }
        for comb_group in comp.comb_groups.iter() {
            Self::write_comb_group(&comb_group.borrow(), 4, f)?;
            writeln!(f)?;
        }
        // Write the continuous assignments
        for assign in &comp.continuous_assignments {
            Self::write_assignment(assign, 4, f)?;
            writeln!(f)?;
        }
        writeln!(f, "  }}")?;

        // Add the control program.
        // Since the syntax doesn't allow combinational components to have a control block, the attributes will always be empty
        if !comp.is_comb {
            let con = &*comp.control.borrow();
            match con {
                ir::Control::Empty(ir::Empty { attributes })
                    if attributes.is_empty() =>
                {
                    writeln!(f, "\n  control {{}}")?;
                }
                _ => {
                    writeln!(f, "\n  control {{")?;
                    Self::write_control(&comp.control.borrow(), 4, f)?;
                    writeln!(f, "  }}")?;
                }
            }
        }

        write!(f, "}}")
    }

    /// Format and write a cell.
    pub fn write_cell<F: io::Write>(
        cell: &ir::Cell,
        indent_level: usize,
        f: &mut F,
    ) -> io::Result<()> {
        match &cell.prototype {
            ir::CellType::Primitive {
                name,
                param_binding,
                ..
            } => {
                write!(f, "{}", " ".repeat(indent_level))?;
                write!(f, "{}", Self::format_at_attributes(&cell.attributes))?;
                if cell.is_reference() {
                    write!(f, "ref ")?
                }
                write!(f, "{} = ", cell.name().id)?;
                writeln!(
                    f,
                    "{}({});",
                    name.id,
                    param_binding
                        .iter()
                        .map(|(_, v)| v.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            ir::CellType::Component { name, .. } => {
                write!(f, "{}", " ".repeat(indent_level))?;
                write!(f, "{}", Self::format_at_attributes(&cell.attributes))?;
                if cell.is_reference() {
                    write!(f, "ref ")?
                }
                writeln!(f, "{} = {}();", cell.name().id, name)
            }
            ir::CellType::Constant { .. } => Ok(()),
            _ => unimplemented!(),
        }
    }

    /// Format and write an assignment.
    pub fn write_assignment<F: io::Write, T: Clone + ToString + Eq>(
        assign: &ir::Assignment<T>,
        indent_level: usize,
        f: &mut F,
    ) -> io::Result<()> {
        write!(f, "{}", " ".repeat(indent_level))?;
        write!(f, "{}", Self::format_at_attributes(&assign.attributes))?;
        write!(f, "{} = ", Self::port_to_str(&assign.dst.borrow()))?;
        if !matches!(&*assign.guard, ir::Guard::True) {
            write!(f, "{} ? ", Self::guard_str(&assign.guard.clone()))?;
        }
        write!(f, "{};", Self::port_to_str(&assign.src.borrow()))
    }

    /// Convinience method to get string representation of [ir::Assignment].
    pub fn assignment_to_str<T>(assign: &ir::Assignment<T>) -> String
    where
        T: ToString + Clone + Eq,
    {
        let mut buf = Vec::new();
        Self::write_assignment(assign, 0, &mut buf).ok();
        String::from_utf8_lossy(buf.as_slice()).to_string()
    }

    /// Convinience method to get string representation of [ir::Control].
    pub fn control_to_str(assign: &ir::Control) -> String {
        let mut buf = Vec::new();
        Self::write_control(assign, 0, &mut buf).ok();
        String::from_utf8_lossy(buf.as_slice()).to_string()
    }

    /// Format and write a combinational group.
    pub fn write_comb_group<F: io::Write>(
        group: &ir::CombGroup,
        indent_level: usize,
        f: &mut F,
    ) -> io::Result<()> {
        write!(f, "{}", " ".repeat(indent_level))?;
        write!(f, "comb group {}", group.name().id)?;
        if !group.attributes.is_empty() {
            write!(f, "{}", Self::format_attributes(&group.attributes))?;
        }
        writeln!(f, " {{")?;

        for assign in &group.assignments {
            Self::write_assignment(assign, indent_level + 2, f)?;
            writeln!(f)?;
        }
        write!(f, "{}}}", " ".repeat(indent_level))
    }

    /// Format and write a group.
    pub fn write_group<F: io::Write>(
        group: &ir::Group,
        indent_level: usize,
        f: &mut F,
    ) -> io::Result<()> {
        write!(f, "{}", " ".repeat(indent_level))?;
        write!(f, "group {}", group.name().id)?;
        if !group.attributes.is_empty() {
            write!(f, "{}", Self::format_attributes(&group.attributes))?;
        }
        writeln!(f, " {{")?;

        for assign in &group.assignments {
            Self::write_assignment(assign, indent_level + 2, f)?;
            writeln!(f)?;
        }
        write!(f, "{}}}", " ".repeat(indent_level))
    }

    /// Format and write a static group.
    pub fn write_static_group<F: io::Write>(
        group: &ir::StaticGroup,
        indent_level: usize,
        f: &mut F,
    ) -> io::Result<()> {
        write!(f, "{}", " ".repeat(indent_level))?;
        write!(
            f,
            "static group {}<{}>",
            group.name().id,
            group.get_latency()
        )?;
        if !group.attributes.is_empty() {
            write!(f, "{}", Self::format_attributes(&group.attributes))?;
        }
        writeln!(f, " {{")?;

        for assign in &group.assignments {
            Self::write_assignment(assign, indent_level + 2, f)?;
            writeln!(f)?;
        }
        write!(f, "{}}}", " ".repeat(indent_level))
    }

    /// Format and write a static control program
    pub fn write_static_control<F: io::Write>(
        scontrol: &ir::StaticControl,
        indent_level: usize,
        f: &mut F,
    ) -> io::Result<()> {
        write!(f, "{}", " ".repeat(indent_level))?;
        match scontrol {
            ir::StaticControl::Enable(ir::StaticEnable {
                group,
                attributes,
            }) => {
                write!(f, "{}", Self::format_at_attributes(attributes))?;
                writeln!(f, "{};", group.borrow().name().id)
            }
            ir::StaticControl::Repeat(ir::StaticRepeat {
                num_repeats,
                attributes,
                body,
                ..
            }) => {
                write!(f, "{}", Self::format_at_attributes(attributes))?;
                writeln!(f, "static repeat {} ", num_repeats)?;
                writeln!(f, "{{")?;
                Self::write_static_control(body, indent_level + 2, f)?;
                writeln!(f, "{}}}", " ".repeat(indent_level))
            }
            ir::StaticControl::Seq(ir::StaticSeq {
                stmts,
                attributes,
                latency,
            }) => {
                write!(f, "{}", Self::format_at_attributes(attributes))?;
                writeln!(f, "static seq <{}> {{", latency)?;
                for stmt in stmts {
                    Self::write_static_control(stmt, indent_level + 2, f)?;
                }
                writeln!(f, "{}}}", " ".repeat(indent_level))
            }
            ir::StaticControl::Par(ir::StaticPar {
                stmts,
                attributes,
                latency,
            }) => {
                write!(f, "{}", Self::format_at_attributes(attributes))?;
                writeln!(f, "static par <{}> {{", latency)?;
                for stmt in stmts {
                    Self::write_static_control(stmt, indent_level + 2, f)?;
                }
                writeln!(f, "{}}}", " ".repeat(indent_level))
            }
            ir::StaticControl::Empty(ir::Empty { attributes }) => {
                if !attributes.is_empty() {
                    writeln!(f, "{};", Self::format_at_attributes(attributes))
                } else {
                    writeln!(f)
                }
            }
            ir::StaticControl::If(ir::StaticIf {
                port,
                latency,
                tbranch,
                fbranch,
                attributes,
            }) => {
                write!(f, "{}", Self::format_at_attributes(attributes))?;
                write!(
                    f,
                    "static if <{}> {} ",
                    latency,
                    Self::port_to_str(&port.borrow()),
                )?;
                writeln!(f, "{{")?;
                Self::write_static_control(tbranch, indent_level + 2, f)?;
                write!(f, "{}}}", " ".repeat(indent_level))?;
                if let ir::StaticControl::Empty(_) = **fbranch {
                    writeln!(f)
                } else {
                    writeln!(f, " else {{")?;
                    Self::write_static_control(fbranch, indent_level + 2, f)?;
                    writeln!(f, "{}}}", " ".repeat(indent_level))
                }
            }
            ir::StaticControl::Invoke(StaticInvoke {
                comp,
                latency,
                inputs,
                outputs,
                attributes,
                ref_cells,
            }) => {
                write!(f, "{}", Self::format_at_attributes(attributes))?;
                write!(
                    f,
                    "static invoke<{}> {}",
                    latency,
                    comp.borrow().name()
                )?;
                if !ref_cells.is_empty() {
                    write!(f, "[")?;
                    for (i, (outcell, incell)) in ref_cells.iter().enumerate() {
                        write!(
                            f,
                            "{}{} = {}",
                            if i == 0 { "" } else { "," },
                            outcell,
                            incell.borrow().name()
                        )?
                    }
                    write!(f, "]")?;
                }
                write!(f, "(")?;
                for (i, (arg, port)) in inputs.iter().enumerate() {
                    write!(
                        f,
                        "{}\n{}{} = {}",
                        if i == 0 { "" } else { "," },
                        " ".repeat(indent_level + 2),
                        arg,
                        Self::port_to_str(&port.borrow())
                    )?;
                }
                if inputs.is_empty() {
                    write!(f, ")(")?;
                } else {
                    write!(f, "\n{})(", " ".repeat(indent_level))?;
                }
                for (i, (arg, port)) in outputs.iter().enumerate() {
                    write!(
                        f,
                        "{}\n{}{} = {}",
                        if i == 0 { "" } else { "," },
                        " ".repeat(indent_level + 2),
                        arg,
                        Self::port_to_str(&port.borrow())
                    )?;
                }
                if outputs.is_empty() {
                    write!(f, ")")?;
                } else {
                    write!(f, "\n{})", " ".repeat(indent_level))?;
                }
                writeln!(f, ";")
            }
        }
    }

    /// Format and write a control program
    pub fn write_control<F: io::Write>(
        control: &ir::Control,
        indent_level: usize,
        f: &mut F,
    ) -> io::Result<()> {
        // write_static_control will indent already so we don't want to indent twice
        if !matches!(control, ir::Control::Static(_)) {
            write!(f, "{}", " ".repeat(indent_level))?;
        }
        match control {
            ir::Control::Enable(ir::Enable { group, attributes }) => {
                write!(f, "{}", Self::format_at_attributes(attributes))?;
                writeln!(f, "{};", group.borrow().name().id)
            }
            ir::Control::Invoke(ir::Invoke {
                comp,
                inputs,
                outputs,
                attributes,
                comb_group,
                ref_cells,
            }) => {
                if !attributes.is_empty() {
                    write!(f, "{}", Self::format_at_attributes(attributes))?
                }
                write!(f, "invoke {}", comp.borrow().name())?;
                if !ref_cells.is_empty() {
                    write!(f, "[")?;
                    for (i, (outcell, incell)) in ref_cells.iter().enumerate() {
                        write!(
                            f,
                            "{}{} = {}",
                            if i == 0 { "" } else { "," },
                            outcell,
                            incell.borrow().name()
                        )?
                    }
                    write!(f, "]")?;
                }
                write!(f, "(")?;
                for (i, (arg, port)) in inputs.iter().enumerate() {
                    write!(
                        f,
                        "{}\n{}{} = {}",
                        if i == 0 { "" } else { "," },
                        " ".repeat(indent_level + 2),
                        arg,
                        Self::port_to_str(&port.borrow())
                    )?;
                }
                if inputs.is_empty() {
                    write!(f, ")(")?;
                } else {
                    write!(f, "\n{})(", " ".repeat(indent_level))?;
                }
                for (i, (arg, port)) in outputs.iter().enumerate() {
                    write!(
                        f,
                        "{}\n{}{} = {}",
                        if i == 0 { "" } else { "," },
                        " ".repeat(indent_level + 2),
                        arg,
                        Self::port_to_str(&port.borrow())
                    )?;
                }
                if outputs.is_empty() {
                    write!(f, ")")?;
                } else {
                    write!(f, "\n{})", " ".repeat(indent_level))?;
                }
                if let Some(group) = comb_group {
                    writeln!(f, " with {};", group.borrow().name)
                } else {
                    writeln!(f, ";")
                }
            }
            ir::Control::Seq(ir::Seq { stmts, attributes }) => {
                write!(f, "{}", Self::format_at_attributes(attributes))?;
                writeln!(f, "seq {{")?;
                for stmt in stmts {
                    Self::write_control(stmt, indent_level + 2, f)?;
                }
                writeln!(f, "{}}}", " ".repeat(indent_level))
            }
            ir::Control::Par(ir::Par { stmts, attributes }) => {
                write!(f, "{}", Self::format_at_attributes(attributes))?;
                writeln!(f, "par {{")?;
                for stmt in stmts {
                    Self::write_control(stmt, indent_level + 2, f)?;
                }
                writeln!(f, "{}}}", " ".repeat(indent_level))
            }
            ir::Control::If(ir::If {
                port,
                cond,
                tbranch,
                fbranch,
                attributes,
            }) => {
                write!(f, "{}", Self::format_at_attributes(attributes))?;
                write!(f, "if {} ", Self::port_to_str(&port.borrow()),)?;
                if let Some(c) = cond {
                    write!(f, "with {} ", c.borrow().name.id)?;
                }
                writeln!(f, "{{")?;
                Self::write_control(tbranch, indent_level + 2, f)?;
                write!(f, "{}}}", " ".repeat(indent_level))?;
                if let ir::Control::Empty(_) = **fbranch {
                    writeln!(f)
                } else {
                    writeln!(f, " else {{")?;
                    Self::write_control(fbranch, indent_level + 2, f)?;
                    writeln!(f, "{}}}", " ".repeat(indent_level))
                }
            }
            ir::Control::While(ir::While {
                port,
                cond,
                body,
                attributes,
            }) => {
                write!(f, "{}", Self::format_at_attributes(attributes))?;
                write!(f, "while {} ", Self::port_to_str(&port.borrow()),)?;
                if let Some(c) = cond {
                    write!(f, "with {} ", c.borrow().name.id)?;
                }
                writeln!(f, "{{")?;
                Self::write_control(body, indent_level + 2, f)?;
                writeln!(f, "{}}}", " ".repeat(indent_level))
            }
            ir::Control::Empty(ir::Empty { attributes }) => {
                if !attributes.is_empty() {
                    writeln!(f, "{};", Self::format_at_attributes(attributes))
                } else {
                    writeln!(f)
                }
            }
            ir::Control::Static(sc) => {
                Self::write_static_control(sc, indent_level, f)
            }
        }
    }

    /// Generate a String-based representation for a guard.
    pub fn guard_str<T: ToString>(guard: &ir::Guard<T>) -> String
    where
        T: Eq,
    {
        match &guard {
            ir::Guard::And(l, r) | ir::Guard::Or(l, r) => {
                let left = if &**l > guard {
                    format!("({})", Self::guard_str(l))
                } else {
                    Self::guard_str(l)
                };
                let right = if &**r > guard {
                    format!("({})", Self::guard_str(r))
                } else {
                    Self::guard_str(r)
                };
                format!("{} {} {}", left, &guard.op_str(), right)
            }
            ir::Guard::CompOp(_, l, r) => {
                format!(
                    "{} {} {}",
                    Self::port_to_str(&l.borrow()),
                    &guard.op_str(),
                    Self::port_to_str(&r.borrow())
                )
            }
            ir::Guard::Not(g) => {
                let s = if &**g > guard {
                    format!("({})", Self::guard_str(g))
                } else {
                    Self::guard_str(g)
                };
                format!("!{}", s)
            }
            ir::Guard::Port(port_ref) => Self::port_to_str(&port_ref.borrow()),
            ir::Guard::True => "1'b1".to_string(),
            ir::Guard::Info(i) => i.to_string(),
        }
    }

    /// Get the port access expression.
    pub fn port_to_str(port: &ir::Port) -> String {
        match &port.parent {
            ir::PortParent::Cell(cell_wref) => {
                let cell_ref =
                    cell_wref.internal.upgrade().unwrap_or_else(|| {
                        panic!(
                            "Malformed AST: No reference to Cell for port `{}'",
                            port.name
                        )
                    });
                let cell = cell_ref.borrow();
                match cell.prototype {
                    ir::CellType::Constant { val, width } => {
                        format!("{}'d{}", width, val)
                    }
                    ir::CellType::ThisComponent => port.name.to_string(),
                    _ => format!("{}.{}", cell.name().id, port.name.id),
                }
            }
            ir::PortParent::Group(group_wref) => format!(
                "{}[{}]",
                group_wref
                    .internal
                    .upgrade()
                    .unwrap_or_else(|| panic!(
                        "Malformed AST: No reference to Group for port `{:#?}'",
                        port
                    ))
                    .borrow()
                    .name()
                    .id,
                port.name.id
            ),
            ir::PortParent::StaticGroup(group_wref) => format!(
                "{}[{}]",
                group_wref
                    .internal
                    .upgrade()
                    .unwrap_or_else(|| panic!(
                        "Malformed AST: No reference to Group for port `{:#?}'",
                        port
                    ))
                    .borrow()
                    .name()
                    .id,
                port.name.id
            ),
        }
    }

    /// Formats the top-level metadata if present
    pub fn format_metadata(metadata: &Option<String>) -> String {
        if let Some(metadata_str) = metadata {
            format!("metadata #{{\n{}\n}}#\n", metadata_str)
        } else {
            String::new()
        }
    }
}
