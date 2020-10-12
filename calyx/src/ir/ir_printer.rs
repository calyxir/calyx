//! Implements a formatter for the in-memory representation of Components.
//! The printing operation clones inner nodes and doesn't perform any mutation
//! to the Component.
use crate::ir;
use std::fmt;

/// Printer for the IR.
pub struct IRPrinter {}

impl IRPrinter {
    /// Format a given Component into a printable string.
    pub fn print(comp: &ir::Component) -> fmt::Result {
        unimplemented!()
    }

    /// Format a given cell into a printable string.
    pub fn print_cell(cell: &ir::Cell) -> fmt::Result {
        unimplemented!()
    }

    /// Format a given assignment into a printable string.
    pub fn print_assignment(assign: &ir::Assignment) -> fmt::Result {
        unimplemented!()
    }

    /// Format a given group into a printable string.
    pub fn print_group(group: &ir::Group) -> fmt::Result {
        unimplemented!()
    }

    /// Format a control program into a printable string.
    pub fn print_control(control: &ir::Control) -> fmt::Result {
        unimplemented!()
    }

    ///////////////////// Internal methods //////////////////////
    fn write_cell(
        cell: &ir::Cell,
        indent_level: usize,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "{}", " ".repeat(indent_level))?;
        write!(f, "{} = ", cell.name.id)?;
        match &cell.prototype {
            ir::CellType::Primitive {
                name,
                param_binding,
                ..
            } => write!(
                f,
                "{}({});",
                name.id,
                param_binding
                    .iter()
                    .map(|(_, v)| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            _ => unimplemented!(),
        }
    }

    fn guard_str(guard: &ir::Guard) -> String {
        match guard {
            ir::Guard::And(gs) | ir::Guard::Or(gs) => gs
                .iter()
                .map(|g| Self::guard_str(g))
                .collect::<Vec<_>>()
                .join(&guard.op_str()),
            ir::Guard::Eq(l, r)
            | ir::Guard::Neq(l, r)
            | ir::Guard::Gt(l, r)
            | ir::Guard::Lt(l, r)
            | ir::Guard::Geq(l, r)
            | ir::Guard::Leq(l, r) => format!(
                "{} {} {}",
                Self::guard_str(l),
                &guard.op_str(),
                Self::guard_str(r)
            ),
            ir::Guard::Not(g) => format!("!{}", Self::guard_str(g)),
            ir::Guard::Port(port_ref) => {
                Self::get_port_access(&port_ref.borrow())
            }
        }
    }

    /// Get the port access expression.
    fn get_port_access(port: &ir::Port) -> String {
        match &port.parent {
            ir::PortParent::Cell(cell_wref) => format!(
                "{}.{}",
                cell_wref.upgrade().unwrap().borrow().name.id,
                port.name.id
            ),
            ir::PortParent::Group(group_wref) => format!(
                "{}[{}]",
                group_wref.upgrade().unwrap().borrow().name.id,
                port.name.id
            ),
        }
    }

    fn write_assignment(
        assign: &ir::Assignment,
        indent_level: usize,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "{}", " ".repeat(indent_level))?;
        write!(f, "{} = ", Self::get_port_access(&assign.dst.borrow()))?;
        if let Some(g) = &assign.guard {
            write!(f, "{} ?", Self::guard_str(&g))?;
        }
        write!(f, "{};", Self::get_port_access(&assign.src.borrow()))
    }

    fn write_group(
        group: &ir::Group,
        indent_level: usize,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "{}", " ".repeat(indent_level))?;
        write!(f, "group {} {{\n", group.name.id)?;
        for assign in &group.assignments {
            Self::write_assignment(assign, indent_level + 2, f)?;
            write!(f, "\n")?;
        }
        write!(f, "{}}}", " ".repeat(indent_level))
    }

    fn write_control(
        control: &ir::Control,
        indent_level: usize,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "{}", " ".repeat(indent_level))?;
        match control {
            ir::Control::Enable(ir::Enable { group }) => {
                write!(f, "{};\n", group.borrow().name.id)
            }
            ir::Control::Seq(ir::Seq { stmts }) => {
                write!(f, "seq {{\n");
                for stmt in stmts {
                    Self::write_control(stmt, indent_level + 2, f)?;
                }
                write!(f, "{}}}", " ".repeat(indent_level))
            }
            ir::Control::Par(ir::Par { stmts }) => {
                write!(f, "par {{\n");
                for stmt in stmts {
                    Self::write_control(stmt, indent_level + 2, f)?;
                }
                write!(f, "{}}}", " ".repeat(indent_level))
            }
            ir::Control::If(ir::If {
                port,
                cond,
                tbranch,
                fbranch,
            }) => {
                write!(
                    f,
                    "if {} with {} {{\n",
                    Self::get_port_access(&port.borrow()),
                    cond.borrow().name.id
                )?;
                Self::write_control(tbranch, indent_level + 2, f)?;
                write!(f, "{}}}", " ".repeat(indent_level))?;
                // TODO(rachit): don't print else when its empty
                write!(f, "else {}{{", " ".repeat(indent_level))?;
                Self::write_control(fbranch, indent_level + 2, f)?;
                write!(f, "{}}}", " ".repeat(indent_level))
            }
            ir::Control::While(ir::While { port, cond, body }) => {
                write!(
                    f,
                    "while {} with {} {{\n",
                    Self::get_port_access(&port.borrow()),
                    cond.borrow().name.id
                )?;
                Self::write_control(body, indent_level + 2, f)?;
                write!(f, "{}}}", " ".repeat(indent_level))
            }
            ir::Control::Empty(Empty) => {
                write!(f, "")
            }
        }
    }
}
