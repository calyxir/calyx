use calyx::errors::Error;
use calyx::ir::IRPrinter;

use crate::ir::{self, RRC};
use std::collections::HashMap;
use std::io;
use std::rc::Rc;

use super::traits::Backend;

#[derive(Default)]
pub struct CirctBackend;

impl Backend for CirctBackend {
    fn name(&self) -> &'static str {
        "circt"
    }

    fn validate(_prog: &ir::Context) -> calyx::errors::CalyxResult<()> {
        Ok(())
    }

    fn emit(
        ctx: &ir::Context,
        file: &mut calyx::utils::OutputFile,
    ) -> calyx::errors::CalyxResult<()> {
        let f = &mut file.get_write();
        write!(f, "calyx.program {{").unwrap();
        ctx.components
            .iter()
            .try_for_each(|comp| {
                Self::write_component(comp, f)?;
                writeln!(f)
            })
            .map_err(|err| {
                let std::io::Error { .. } = err;
                Error::WriteError(format!(
                    "File not found: {}",
                    file.as_path_string()
                ))
            })?;
        write!(f, "}}").unwrap();
        Ok(())
    }

    fn link_externs(
        _prog: &ir::Context,
        _write: &mut calyx::utils::OutputFile,
    ) -> calyx::errors::CalyxResult<()> {
        Ok(())
    }
}

impl CirctBackend {
    // Format attributes of the form `@static(1)`.
    // Returns the empty string if the `attrs` is empty.
    // TODO: Attributes currently not supported in the CIRCT frontend.
    /* fn format_at_attributes(attrs: &ir::Attributes) -> String {
        attrs
            .attrs
            .iter()
            .map(|(k, v)| {
                if *v == 1 {
                    format!("@{}", k)
                } else {
                    format!("@{}({})", k, v)
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    } */

    // Format attributes of the form `<"static"=1>`.
    // Returns the empty string if the `attrs` is empty.
    // TODO: Attributes currently not supported in the CIRCT frontend.
    /* fn format_attributes(attrs: &ir::Attributes) -> String {
        if attrs.is_empty() {
            "".to_string()
        } else {
            format!(
                "<{}>",
                attrs
                    .attrs
                    .iter()
                    .map(|(k, v)| { format!("\"{}\"={}", k, v) })
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    } */

    /// Formats port definitions in signatures
    fn format_port_def(ports: &[RRC<ir::Port>]) -> String {
        ports
            .iter()
            .map(|p| {
                format!(
                    "%{}: i{}",
                    p.borrow().name.id.to_string(),
                    p.borrow().width
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Formats and writes the Component to the formatter.
    pub fn write_component<F: io::Write>(
        comp: &ir::Component,
        f: &mut F,
    ) -> io::Result<()> {
        let sig = comp.signature.borrow();
        let (inputs, outputs): (Vec<_>, Vec<_>) =
            sig.ports.iter().map(|p| Rc::clone(p)).partition(|p| {
                // Cell signature stores the ports in reversed direction.
                matches!(p.borrow().direction, ir::Direction::Output)
            });

        writeln!(
            f,
            "calyx.component @{}({}) -> ({}) {{",
            comp.name.id,
            Self::format_port_def(&inputs),
            Self::format_port_def(&outputs),
        )?;

        // Add the cells
        for cell in comp.cells.iter() {
            // Only print out non-constant cells.
            // Constants are printed inside calyx.wires.
            if cell.borrow().type_name().is_some() {
                Self::write_cell(&cell.borrow(), 2, f)?;
            }
        }

        // Add the wires
        writeln!(f, "  calyx.wires {{")?;
        // Print out all the constants
        comp.cells.iter().try_for_each(|cell_ref| {
            let cell = cell_ref.borrow();
            match &cell.prototype {
                ir::CellType::Constant { val, width } => {
                    writeln!(
                        f,
                        "    %{} = hw.constant {} : i{}",
                        cell.name(),
                        val,
                        width
                    )
                }
                _ => Ok(()),
            }
        })?;
        for group in comp.groups.iter() {
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
            writeln!(f, "  calyx.control {{}}")?;
        } else {
            writeln!(f, "  calyx.control {{")?;
            Self::write_control(&comp.control.borrow(), 4, f)?;
            writeln!(f, "  }}")?;
        }

        write!(f, "}}")
    }

    pub fn write_prototype_sig<F: io::Write>(
        cell_type: &ir::CellType,
        cell_name: String,
        f: &mut F,
    ) -> io::Result<()> {
        match cell_type {
            ir::CellType::Primitive {
                name,
                param_binding,
            } => {
                let bind: HashMap<&str, u64> = param_binding
                    .iter()
                    .map(|(k, v)| (k.as_ref(), *v))
                    .collect();
                match name.as_ref() {
                    "std_reg" => {
                        write!(f, "calyx.register \"{}\" : ", cell_name)
                    }
                    "std_mem_d1" => write!(
                        f,
                        "calyx.memory \"{}\"<[{}] x {}> [{}] : ",
                        cell_name,
                        bind["SIZE"],
                        bind["WIDTH"],
                        bind["IDX_SIZE"]
                    ),
                    "std_mem_d2" => write!(
                        f,
                        "calyx.memory \"{}\"<[{}, {}] x {}> [{}, {}] : ",
                        cell_name,
                        bind["D0_SIZE"],
                        bind["D1_SIZE"],
                        bind["WIDTH"],
                        bind["D0_IDX_SIZE"],
                        bind["D1_IDX_SIZE"]
                    ),
                    "std_mem_d3" => write!(
                        f,
                        "calyx.memory \"{}\"<[{}, {}, {}] x {}> [{}, {}, {}] : ",
                        cell_name,
                        bind["D0_SIZE"],
                        bind["D1_SIZE"],
                        bind["D2_SIZE"],
                        bind["WIDTH"],
                        bind["D0_IDX_SIZE"],
                        bind["D1_IDX_SIZE"],
                        bind["D2_IDX_SIZE"]
                    ),
                    "std_mem_d4" => write!(
                        f,
                        "calyx.memory \"{}\"<[{}, {}, {}, {}] x {}> [{}, {}, {}, {}] : ",
                        cell_name,
                        bind["D0_SIZE"],
                        bind["D1_SIZE"],
                        bind["D2_SIZE"],
                        bind["D3_SIZE"],
                        bind["WIDTH"],
                        bind["D0_IDX_SIZE"],
                        bind["D1_IDX_SIZE"],
                        bind["D2_IDX_SIZE"],
                        bind["D3_IDX_SIZE"]
                    ),
                    prim => write!(f, "calyx.{} \"{}\" : ", prim, cell_name)
                }
            }
            ir::CellType::Component { name } => {
                write!(f, "calyx.instance \"{}\" @{} : ", cell_name, name)
            }
            ir::CellType::Constant { .. } => {
                /* Constants go in the calyx.wires section */
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Format and write a cell.
    pub fn write_cell<F: io::Write>(
        cell: &ir::Cell,
        indent_level: usize,
        f: &mut F,
    ) -> io::Result<()> {
        write!(f, "{}", " ".repeat(indent_level))?;
        let name = cell.name().id.clone();
        let all_ports = cell
            .ports()
            .iter()
            .map(|p| format!("%{}.{}", name, p.borrow().name))
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "{} = ", all_ports)?;
        Self::write_prototype_sig(&cell.prototype, name, f)?;
        let all_port_widths = cell
            .ports()
            .iter()
            .map(|p| format!("i{}", p.borrow().width))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(f, "{}", all_port_widths)
    }

    /// Format and write an assignment.
    pub fn write_assignment<F: io::Write>(
        assign: &ir::Assignment,
        indent_level: usize,
        f: &mut F,
    ) -> io::Result<()> {
        write!(f, "{}", " ".repeat(indent_level))?;
        let dst = assign.dst.borrow();
        match (dst.is_hole(), dst.name.as_ref()) {
            (true, "done") => write!(f, "calyx.group_done ")?,
            (true, "go") => write!(f, "calyx.group_go ")?,
            (true, _) => unreachable!(),
            (false, _) => {
                write!(
                    f,
                    "calyx.assign {} = ",
                    Self::get_port_access(&assign.dst.borrow())
                )?;
            }
        }
        write!(f, "{}", Self::get_port_access(&assign.src.borrow()),)?;
        if let ir::Guard::Port(p) = &*assign.guard {
            write!(f, ", {} ?", Self::get_port_access(&p.borrow()))?;
        } else if matches!(&*assign.guard, ir::Guard::True) {
            /* Print nothing */
        } else {
            panic!("Failed to compile guard: {}.\nFirst run the `lower-guards` pass. If you did, report this as an issue.", IRPrinter::guard_str(&*assign.guard));
        }
        write!(f, " : i{}", assign.src.borrow().width)
    }

    /// Format and write a group.
    pub fn write_group<F: io::Write>(
        group: &ir::Group,
        indent_level: usize,
        f: &mut F,
    ) -> io::Result<()> {
        write!(f, "{}", " ".repeat(indent_level))?;
        write!(f, "calyx.group @{}", group.name().id)?;
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
            ir::Control::Enable(ir::Enable { group, .. }) => {
                writeln!(f, "calyx.enable @{}", group.borrow().name().id)
            }
            ir::Control::Invoke(ir::Invoke { .. }) => {
                todo!("invoke operator for CIRCT backend")
            }
            ir::Control::Seq(ir::Seq { stmts, .. }) => {
                writeln!(f, "calyx.seq {{")?;
                for stmt in stmts {
                    Self::write_control(stmt, indent_level + 2, f)?;
                }
                writeln!(f, "{}}}", " ".repeat(indent_level))
            }
            ir::Control::Par(ir::Par { stmts, .. }) => {
                writeln!(f, "calyx.par {{")?;
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
                ..
            }) => {
                writeln!(
                    f,
                    "calyx.if {} with @{} {{",
                    Self::get_port_access(&port.borrow()),
                    cond.borrow().name().id
                )?;
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
                port, cond, body, ..
            }) => {
                writeln!(
                    f,
                    "calyx.while {} with @{} {{",
                    Self::get_port_access(&port.borrow()),
                    cond.borrow().name().id
                )?;
                Self::write_control(body, indent_level + 2, f)?;
                writeln!(f, "{}}}", " ".repeat(indent_level))
            }
            ir::Control::Empty(_) => writeln!(f),
        }
    }

    /// Get the port access expression.
    fn get_port_access(port: &ir::Port) -> String {
        match &port.parent {
            ir::PortParent::Cell(cell_wref) => {
                let cell_ref = cell_wref.upgrade();
                let cell = cell_ref.borrow();
                match cell.prototype {
                    ir::CellType::Constant { val, width } => {
                        format!("%{}", ir::Cell::constant_name(val, width))
                    }
                    ir::CellType::ThisComponent => port.name.to_string(),
                    _ => format!("%{}.{}", cell.name().id, port.name.id),
                }
            }
            ir::PortParent::Group(_) => unimplemented!(),
        }
    }
}
