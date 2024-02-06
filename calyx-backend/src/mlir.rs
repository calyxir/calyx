//! Generation for the MLIR backend of the Calyx compiler
use super::traits::Backend;
use calyx_frontend::GetAttributes;
use calyx_ir::{self as ir, RRC};
use calyx_utils::Error;
use std::collections::HashMap;
use std::io;
use std::rc::Rc;

#[derive(Default)]
pub struct MlirBackend;

impl Backend for MlirBackend {
    fn name(&self) -> &'static str {
        "mlir"
    }

    fn validate(_prog: &ir::Context) -> calyx_utils::CalyxResult<()> {
        Ok(())
    }

    fn emit(
        ctx: &ir::Context,
        file: &mut calyx_utils::OutputFile,
    ) -> calyx_utils::CalyxResult<()> {
        let res = {
            let f = &mut file.get_write();
            writeln!(
                f,
                "module attributes {{calyx.entrypoint = \"{}\" }} {{",
                ctx.entrypoint
            )?;
            ctx.components.iter().try_for_each(|comp| {
                Self::write_component(comp, f)?;
                writeln!(f)
            })?;
            write!(f, "\n}}\n")
        };
        res.map_err(|err| {
            let std::io::Error { .. } = err;
            Error::write_error(format!(
                "File not found: {}",
                file.as_path_string()
            ))
        })
    }

    fn link_externs(
        _prog: &ir::Context,
        _write: &mut calyx_utils::OutputFile,
    ) -> calyx_utils::CalyxResult<()> {
        Ok(())
    }
}

impl MlirBackend {
    fn format_attributes(attrs: &ir::Attributes) -> String {
        if attrs.is_empty() {
            "".to_string()
        } else {
            format!(
                " {{{}}}",
                attrs.to_string_with(", ", |k, v| { format!("{k}={v}") })
            )
        }
    }

    /// Formats port definitions in signatures
    fn format_port_def(ports: &[RRC<ir::Port>]) -> String {
        ports
            .iter()
            .map(|p| {
                format!(
                    "%{}: i{}{}",
                    p.borrow().name.id,
                    p.borrow().width,
                    Self::format_attributes(&p.borrow().attributes)
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
            sig.ports.iter().map(Rc::clone).partition(|p| {
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
            Self::write_cell(&cell.borrow(), 2, f)?;
        }

        // Add the wires
        writeln!(f, "  calyx.wires {{")?;
        for group in comp.get_groups().iter() {
            Self::write_group(&group.borrow(), 4, f)?;
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

    /// Write the operation corresponding to the primitve and return true if the
    /// operation accepts attributes.
    pub fn write_prototype_sig<F: io::Write, S: ToString>(
        cell_type: &ir::CellType,
        cell_name: S,
        f: &mut F,
    ) -> io::Result<bool> {
        let cell_name = cell_name.to_string();
        match cell_type {
            ir::CellType::Primitive {
                name,
                param_binding,
                ..
            } => {
                let bind: HashMap<&str, u64> = param_binding
                    .iter()
                    .map(|(k, v)| (k.as_ref(), *v))
                    .collect();
                match name.as_ref() {
                    "undef" => {
                        write!(f, "calyx.undefined @{cell_name}")?
                    }
                    "std_reg" => {
                        write!(f, "calyx.register @{cell_name}")?
                    }
                    "comb_mem_d1" => write!(
                        f,
                        "calyx.memory @{cell_name} <[{}] x {}> [{}]",
                        bind["SIZE"],
                        bind["WIDTH"],
                        bind["IDX_SIZE"]
                    )?,
                    "comb_mem_d2" => write!(
                        f,
                        "calyx.memory @{cell_name} <[{}, {}] x {}> [{}, {}]",
                        bind["D0_SIZE"],
                        bind["D1_SIZE"],
                        bind["WIDTH"],
                        bind["D0_IDX_SIZE"],
                        bind["D1_IDX_SIZE"]
                    )?,
                    "comb_mem_d3" => write!(
                        f,
                        "calyx.memory @{cell_name} <[{}, {}, {}] x {}> [{}, {}, {}]",
                        bind["D0_SIZE"],
                        bind["D1_SIZE"],
                        bind["D2_SIZE"],
                        bind["WIDTH"],
                        bind["D0_IDX_SIZE"],
                        bind["D1_IDX_SIZE"],
                        bind["D2_IDX_SIZE"]
                    )?,
                    "comb_mem_d4" => write!(
                        f,
                        "calyx.memory @{cell_name} <[{}, {}, {}, {}] x {}> [{}, {}, {}, {}]",
                        bind["D0_SIZE"],
                        bind["D1_SIZE"],
                        bind["D2_SIZE"],
                        bind["D3_SIZE"],
                        bind["WIDTH"],
                        bind["D0_IDX_SIZE"],
                        bind["D1_IDX_SIZE"],
                        bind["D2_IDX_SIZE"],
                        bind["D3_IDX_SIZE"]
                    )?,
                    prim => write!(f, "calyx.{prim} @{cell_name}")?,
                }
            }
            ir::CellType::Component { name } => {
                write!(f, "calyx.instance @{cell_name} of @{name}")?;
            }
            ir::CellType::Constant { val, .. } => {
                write!(f, "hw.constant {val}")?;
                return Ok(false);
            }
            _ => (),
        };
        Ok(true)
    }

    /// Format and write a cell.
    pub fn write_cell<F: io::Write>(
        cell: &ir::Cell,
        indent_level: usize,
        f: &mut F,
    ) -> io::Result<()> {
        write!(f, "{}", " ".repeat(indent_level))?;
        let name = cell.name().id;
        let all_ports = cell
            .ports()
            .iter()
            .map(|p| format!("%{}.{}", name, p.borrow().name))
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "{} = ", all_ports)?;
        let supports_attrs =
            Self::write_prototype_sig(&cell.prototype, name.as_str(), f)?;
        if supports_attrs {
            write!(f, "{}", Self::format_attributes(&cell.attributes))?;
        }
        write!(f, " : ")?;
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
        assign: &ir::Assignment<ir::Nothing>,
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
        if let ir::Guard::Port(p) = &*assign.guard {
            write!(f, "{} ? ", Self::get_port_access(&p.borrow()))?;
        } else if matches!(&*assign.guard, ir::Guard::True) {
            /* Print nothing */
        } else {
            panic!("Failed to compile guard: {}.\nFirst run the `lower-guards` pass. If you did, report this as an issue.", ir::Printer::guard_str(&assign.guard));
        }
        write!(f, "{}", Self::get_port_access(&assign.src.borrow()),)?;
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
        write!(f, "{}}}", " ".repeat(indent_level))?;
        if let Some(attr) = group.get_attributes() {
            write!(f, "{}", Self::format_attributes(attr))?;
        }
        Ok(())
    }

    /// Format and write combinational groups
    pub fn write_comb_group<F: io::Write>(
        group: &ir::CombGroup,
        indent_level: usize,
        f: &mut F,
    ) -> io::Result<()> {
        write!(f, "{}", " ".repeat(indent_level))?;
        write!(f, "calyx.comb_group @{}", group.name().id)?;
        writeln!(f, " {{")?;

        for assign in &group.assignments {
            Self::write_assignment(assign, indent_level + 2, f)?;
            writeln!(f)?;
        }
        write!(f, "{}}}", " ".repeat(indent_level))?;
        if let Some(attr) = group.get_attributes() {
            write!(f, "{}", Self::format_attributes(attr))?;
        }
        Ok(())
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
                write!(f, "calyx.enable @{}", group.borrow().name().id)
            }
            ir::Control::Static(_) => {
                todo!("static control not currently supported for MLIR")
            }
            ir::Control::Invoke(ir::Invoke { .. }) => {
                todo!("invoke operator for MLIR backend")
            }
            ir::Control::Repeat(ir::Repeat { .. }) => {
                todo!("repeat operator for MLIR backend")
            }
            ir::Control::Seq(ir::Seq { stmts, .. }) => {
                writeln!(f, "calyx.seq {{")?;
                for stmt in stmts {
                    Self::write_control(stmt, indent_level + 2, f)?;
                }
                write!(f, "{}}}", " ".repeat(indent_level))
            }
            ir::Control::Par(ir::Par { stmts, .. }) => {
                writeln!(f, "calyx.par {{")?;
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
                ..
            }) => {
                write!(
                    f,
                    "calyx.if {}",
                    Self::get_port_access(&port.borrow())
                )?;
                if let Some(cond) = cond {
                    write!(f, " with @{}", cond.borrow().name().id)?;
                }
                writeln!(f, " {{")?;
                Self::write_control(tbranch, indent_level + 2, f)?;
                write!(f, "{}}}", " ".repeat(indent_level))?;
                if let ir::Control::Empty(_) = **fbranch {
                    Ok(())
                } else {
                    writeln!(f, " else {{")?;
                    Self::write_control(fbranch, indent_level + 2, f)?;
                    write!(f, "{}}}", " ".repeat(indent_level))
                }
            }
            ir::Control::While(ir::While {
                port, cond, body, ..
            }) => {
                write!(
                    f,
                    "calyx.while {}",
                    Self::get_port_access(&port.borrow())
                )?;
                if let Some(cond) = cond {
                    write!(f, " with @{}", cond.borrow().name().id)?;
                }
                writeln!(f, " {{")?;
                Self::write_control(body, indent_level + 2, f)?;
                write!(f, "{}}}", " ".repeat(indent_level))
            }
            ir::Control::Empty(_) => writeln!(f),
        }?;
        let attr = control.get_attributes();
        write!(f, "{}", Self::format_attributes(attr))?;
        writeln!(f)
    }

    /// Get the port access expression.
    fn get_port_access(port: &ir::Port) -> String {
        match &port.parent {
            ir::PortParent::Cell(cell_wref) => {
                let cell_ref = cell_wref.upgrade();
                let cell = cell_ref.borrow();
                match cell.prototype {
                    ir::CellType::Constant { val, width } => {
                        format!("%{}.out", ir::Cell::constant_name(val, width))
                    }
                    ir::CellType::ThisComponent => {
                        format!("%{}", port.name)
                    }
                    _ => format!("%{}.{}", cell.name().id, port.name.id),
                }
            }
            ir::PortParent::Group(_) => unimplemented!(),
            ir::PortParent::StaticGroup(_) => unimplemented!(),
        }
    }
}
