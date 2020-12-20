//! Implements a formatter for the in-memory representation of Components.
//! The printing operation clones inner nodes and doesn't perform any mutation
//! to the Component.
use crate::ir;
use std::io;

/// Printer for the IR.
pub struct IRPrinter;

impl IRPrinter {
    /// Formats and writes the Component to the formatter.
    pub fn write_component<F: io::Write>(
        comp: &ir::Component,
        f: &mut F,
    ) -> io::Result<()> {
        let sig = comp.signature.borrow();
        let (inputs, outputs): (Vec<_>, Vec<_>) =
            sig.ports.iter().partition(|p| {
                matches!(p.borrow().direction, ir::Direction::Input)
            });

        writeln!(
            f,
            "component {}({}) -> ({}) {{",
            comp.name.id,
            inputs
                .iter()
                .map(|p| format!(
                    "{}: {}",
                    p.borrow().name.id.to_string(),
                    p.borrow().width
                ))
                .collect::<Vec<_>>()
                .join(", "),
            outputs
                .iter()
                .map(|p| format!(
                    "{}: {}",
                    p.borrow().name.id.to_string(),
                    p.borrow().width
                ))
                .collect::<Vec<_>>()
                .join(", ")
        )?;

        // Add the cells
        writeln!(f, "  cells {{")?;
        for cell in &comp.cells {
            Self::write_cell(&cell.borrow(), 4, f)?;
        }
        writeln!(f, "  }}")?;

        // Add the wires
        writeln!(f, "  wires {{")?;
        for group in &comp.groups {
            Self::write_group(&group.borrow(), 4, f)?;
            writeln!(f)?;
        }
        // Write the continuous assignments
        for assign in &comp.continuous_assignments {
            Self::write_assignment(assign, 4, f)?;
            writeln!(f)?;
        }
        writeln!(f, "  }}\n")?;

        // Add the control program
        if matches!(&*comp.control.borrow(), ir::Control::Empty(..)) {
            writeln!(f, "  control {{}}")?;
        } else {
            writeln!(f, "  control {{")?;
            Self::write_control(&comp.control.borrow(), 4, f)?;
            writeln!(f, "  }}")?;
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
                write!(f, "{} = prim ", cell.name.id)?;
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
            ir::CellType::Constant { .. } => Ok(()),
            ir::CellType::Component { name } => {
                write!(f, "{}", " ".repeat(indent_level))?;
                writeln!(f, "{} = {};", cell.name.id, name)
            }
            _ => unimplemented!(),
        }
    }

    /// Format and write an assignment.
    pub fn write_assignment<F: io::Write>(
        assign: &ir::Assignment,
        indent_level: usize,
        f: &mut F,
    ) -> io::Result<()> {
        write!(f, "{}", " ".repeat(indent_level))?;
        write!(f, "{} = ", Self::get_port_access(&assign.dst.borrow()))?;
        if !matches!(&assign.guard, ir::Guard::True) {
            write!(
                f,
                "{} ? ",
                Self::guard_str(&assign.guard.clone().flatten())
            )?;
        }
        write!(f, "{};", Self::get_port_access(&assign.src.borrow()))
    }

    /// Format and write a group.
    pub fn write_group<F: io::Write>(
        group: &ir::Group,
        indent_level: usize,
        f: &mut F,
    ) -> io::Result<()> {
        write!(f, "{}", " ".repeat(indent_level))?;
        write!(f, "group {}", group.name.id)?;
        if !group.attributes.is_empty() {
            write!(
                f,
                "<{}>",
                group
                    .attributes
                    .iter()
                    .map(|(k, v)| { format!("\"{}\"={}", k, v) })
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
        }
        writeln!(f, " {{")?;

        for assign in &group.assignments {
            Self::write_assignment(assign, indent_level + 2, f)?;
            writeln!(f)?;
        }
        write!(f, "{}}}", " ".repeat(indent_level))
    }

    /// Format and write a control program
    pub fn write_control<F: io::Write>(
        control: &ir::Control,
        indent_level: usize,
        f: &mut F,
    ) -> io::Result<()> {
        write!(f, "{}", " ".repeat(indent_level))?;
        match control {
            ir::Control::Enable(ir::Enable { group }) => {
                writeln!(f, "{};", group.borrow().name.id)
            }
            ir::Control::Invoke(ir::Invoke {
                comp,
                inputs,
                outputs,
            }) => {
                write!(f, "invoke {}(", comp.borrow().name)?;
                for (arg, port) in inputs {
                    write!(
                        f,
                        "\n{}{} = {},",
                        " ".repeat(indent_level + 2),
                        arg,
                        Self::get_port_access(&port.borrow())
                    )?;
                }
                if inputs.is_empty() {
                    write!(f, ")(")?;
                } else {
                    write!(f, "\n{})(", " ".repeat(indent_level))?;
                }
                for (arg, port) in outputs {
                    write!(
                        f,
                        "\n{}{} = {},",
                        " ".repeat(indent_level + 2),
                        arg,
                        Self::get_port_access(&port.borrow())
                    )?;
                }
                if outputs.is_empty() {
                    writeln!(f, ");")
                } else {
                    writeln!(f, "\n{});", " ".repeat(indent_level))
                }
            }
            ir::Control::Seq(ir::Seq { stmts }) => {
                writeln!(f, "seq {{")?;
                for stmt in stmts {
                    Self::write_control(stmt, indent_level + 2, f)?;
                }
                writeln!(f, "{}}}", " ".repeat(indent_level))
            }
            ir::Control::Par(ir::Par { stmts }) => {
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
            }) => {
                writeln!(
                    f,
                    "if {} with {} {{",
                    Self::get_port_access(&port.borrow()),
                    cond.borrow().name.id
                )?;
                Self::write_control(tbranch, indent_level + 2, f)?;
                write!(f, "{}}}", " ".repeat(indent_level))?;
                // TODO(rachit): don't print else when its empty
                if let ir::Control::Empty(_) = **fbranch {
                    writeln!(f)
                } else {
                    writeln!(f, " else {{")?;
                    Self::write_control(fbranch, indent_level + 2, f)?;
                    writeln!(f, "{}}}", " ".repeat(indent_level))
                }
            }
            ir::Control::While(ir::While { port, cond, body }) => {
                writeln!(
                    f,
                    "while {} with {} {{",
                    Self::get_port_access(&port.borrow()),
                    cond.borrow().name.id
                )?;
                Self::write_control(body, indent_level + 2, f)?;
                writeln!(f, "{}}}", " ".repeat(indent_level))
            }
            ir::Control::Empty(_) => writeln!(f),
        }
    }

    /// Generate a String-based representation for a guard.
    pub fn guard_str(guard: &ir::Guard) -> String {
        match &guard {
            ir::Guard::And(gs) => gs
                .iter()
                .map(|g| {
                    let s = Self::guard_str(g);
                    if g > guard {
                        s.surround("(", ")")
                    } else {
                        s
                    }
                })
                .filter(|s| s != "")
                .collect::<Vec<_>>()
                .join(&format!(" {} ", guard.op_str())),
            ir::Guard::Or(gs) => gs
                .iter()
                .map(|g| {
                    let s = Self::guard_str(g);
                    if g > guard {
                        s.surround("(", ")")
                    } else {
                        s
                    }
                })
                .filter(|s| s != "")
                .collect::<Vec<_>>()
                .join(&format!(" {} ", guard.op_str())),
            ir::Guard::Eq(l, r)
            | ir::Guard::Neq(l, r)
            | ir::Guard::Gt(l, r)
            | ir::Guard::Lt(l, r)
            | ir::Guard::Geq(l, r)
            | ir::Guard::Leq(l, r) => {
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
            ir::Guard::Not(g) => {
                let s = if &**g > guard {
                    format!("({})", Self::guard_str(g))
                } else {
                    Self::guard_str(g)
                };
                format!("!{}", s)
            }
            ir::Guard::Port(port_ref) => {
                Self::get_port_access(&port_ref.borrow())
            }
            ir::Guard::True => "1'b1".to_string(),
        }
    }

    /// Get the port access expression.
    fn get_port_access(port: &ir::Port) -> String {
        match &port.parent {
            ir::PortParent::Cell(cell_wref) => {
                let cell_ref =
                    cell_wref.internal.upgrade().unwrap_or_else(|| {
                        panic!(
                        "Malformed AST: No reference to Cell for port `{:#?}'",
                        port
                    )
                    });
                let cell = cell_ref.borrow();
                match cell.prototype {
                    ir::CellType::Constant { val, width } => {
                        format!("{}'d{}", width, val)
                    }
                    ir::CellType::ThisComponent => port.name.to_string(),
                    _ => format!("{}.{}", cell.name.id, port.name.id),
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
                    .name
                    .id,
                port.name.id
            ),
        }
    }
}

trait Surround {
    fn surround(self, pre: &str, suf: &str) -> Self;
}

impl Surround for String {
    fn surround(mut self, pre: &str, suf: &str) -> Self {
        self.insert_str(0, pre);
        self.push_str(suf);
        self
    }
}
