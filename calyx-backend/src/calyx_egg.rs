//! Generation for the Calyx Egg backend of the Calyx compiler.
use super::traits::Backend;
use calyx_frontend::GetAttributes;
use calyx_ir::{self as ir};
use calyx_utils::Error;
use itertools::Itertools;
use std::collections::HashSet;
use std::io;
use std::io::Write;

#[derive(Default)]
pub struct CalyxEggBackend;

impl Backend for CalyxEggBackend {
    fn name(&self) -> &'static str {
        "calyx-egg"
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
            if ctx.components.len() > 1 {
                todo!("multiple components not supported in CalyxEgg")
            }

            ctx.components.iter().try_for_each(|comp| {
                Self::write_component(comp, f)?;
                writeln!(f)
            })?;
            write!(f, "\n")
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

impl CalyxEggBackend {
    fn format_attributes(attrs: &ir::Attributes) -> String {
        let mut s: String = format!("(map-empty)");
        for attribute in attrs.to_vec(|k, v| format!("\"{k}\" {v}")) {
            s = format!("(map-insert {} {})", s, attribute);
        }
        format!("(Attributes {})", s)
    }

    fn format_demands<F: io::Write>(
        demands: Vec<&str>,
        lists: &Vec<String>,
        f: &mut F,
    ) -> io::Result<()> {
        writeln!(f)?;
        for demand in demands {
            for list in lists {
                writeln!(f, "({}{})", demand, list)?;
            }
        }
        Ok(())
    }

    /// Formats and writes the Component to the formatter.
    pub fn write_component<F: io::Write>(
        comp: &ir::Component,
        f: &mut F,
    ) -> io::Result<()> {
        for cell in comp.cells.iter() {
            Self::write_cell(&cell.borrow(), f)?;
        }
        for group in comp.get_groups().iter() {
            Self::write_group(&group.borrow(), f)?;
            writeln!(f)?;
        }
        for _ in comp.comb_groups.iter() {
            todo!("`combinational-group` is not supported in CalyxEgg")
        }
        // Write the continuous assignments
        for _ in &comp.continuous_assignments {
            todo!("`continuous assignment` is not supported in CalyxEgg")
        }

        // Add the control program
        if matches!(&*comp.control.borrow(), ir::Control::Empty(..)) {
            todo!("`empty` control is not supported in CalyxEgg")
        }
        let ename: String = format!("egg-{}", comp.name);
        write!(f, "(let {} ", ename)?;
        let mut lists = Vec::new();
        Self::write_control(&comp.control.borrow(), &mut lists, f)?;
        write!(f, ")\n")?;

        // Make demands.
        Self::format_demands(
            [
                "list-length-demand",
                "sum-latency-demand",
                "max-latency-demand",
            ]
            .to_vec(),
            &lists,
            f,
        )
    }

    /// Format and write a cell.
    pub fn write_cell<F: io::Write>(
        cell: &ir::Cell,
        f: &mut F,
    ) -> io::Result<()> {
        let name = cell.name().id;
        writeln!(f, "(let c-{} (Cell \"{}\"))", name, name)
    }

    /// Format and write a group.
    pub fn write_group<F: io::Write>(
        group: &ir::Group,
        f: &mut F,
    ) -> io::Result<()> {
        let name = group.name().id;
        write!(f, "(let {} (Group \"{}\" ", name, name)?;

        let mut cells: HashSet<String> = HashSet::new();
        for assign in &group.assignments {
            // Currently, the set of cells is used to determine whether two groups have "exclusive" cells, i.e.,
            // the two groups may run in parallel with no semantic changes to the program. In this case, we don't
            // really care if constants are shared between groups.
            if !assign.dst.borrow().is_any_constant() {
                if let ir::PortParent::Cell(cell) = &assign.dst.borrow().parent
                {
                    cells.insert(cell.upgrade().borrow().name().id.to_string());
                }
            }
            if !assign.src.borrow().is_any_constant() {
                if let ir::PortParent::Cell(cell) = &assign.src.borrow().parent
                {
                    cells.insert(cell.upgrade().borrow().name().id.to_string());
                }
            }
        }
        if cells.is_empty() {
            write!(f, "(CellSet (set-empty))")?;
        } else {
            write!(
                f,
                "(CellSet (set-of {}))",
                Vec::from_iter(cells)
                    .into_iter()
                    .map(|x| format!("c-{}", x))
                    .collect_vec()
                    .join(" ")
            )?;
        }
        write!(f, "))")
    }

    /// Format and write a control program
    pub fn write_control<F: io::Write>(
        control: &ir::Control,
        lists: &mut Vec<String>,
        f: &mut F,
    ) -> io::Result<()> {
        let attr = control.get_attributes();
        match control {
            ir::Control::Enable(ir::Enable { group, .. }) => {
                write!(
                    f,
                    "(Enable {} {})",
                    group.borrow().name().id,
                    Self::format_attributes(attr),
                )
            }
            ir::Control::Par(ir::Par { stmts, .. })
            | ir::Control::Seq(ir::Seq { stmts, .. }) => {
                let operator = match control {
                    ir::Control::Par(..) => "Par",
                    ir::Control::Seq(..) => "Seq",
                    _ => panic!("unreachable"),
                };
                write!(f, "({} {}", operator, Self::format_attributes(attr))?;
                // We need to keep track of the "list of lists" so we can perform analyses through demands.
                let mut s = Vec::new();
                let mut b = io::BufWriter::new(&mut s);

                for stmt in stmts {
                    write!(b, " (Cons ")?;
                    write!(f, " (Cons ")?;
                    Self::write_control(stmt, lists, &mut b)?;
                    Self::write_control(stmt, lists, f)?;
                }
                write!(b, " (Nil){}", ")".repeat(stmts.len()))?;
                write!(f, " (Nil){}", ")".repeat(stmts.len()))?;
                lists.push(String::from_utf8(b.buffer().to_vec()).unwrap());

                write!(f, ")")
            }
            ir::Control::Static(_) => {
                todo!("`static` is not supported in CalyxEgg")
            }
            ir::Control::Invoke(ir::Invoke { .. }) => {
                todo!("`invoke` is not supported in CalyxEgg")
            }
            ir::Control::Repeat(ir::Repeat { .. }) => {
                todo!("`repeat` is not supported in CalyxEgg")
            }
            ir::Control::If(ir::If { .. }) => {
                todo!("`if` is not supported in CalyxEgg")
            }
            ir::Control::While(ir::While { .. }) => {
                todo!("`while` is not supported in CalyxEgg")
            }
            ir::Control::Empty(_) => writeln!(f),
        }
    }
}
