use calyx_frontend::GetAttributes;
use calyx_ir::{self as ir};
use itertools::Itertools;
use std::collections::HashSet;
use std::io::{self};

#[derive(Default)]
pub struct ToEggPrinter;

impl ToEggPrinter {
    fn format_attributes(
        attrs: &ir::Attributes,
        latency: Option<u64>,
        promotable: Option<u64>,
    ) -> String {
        let mut s: String = "(map-empty)".to_string();
        for attribute in attrs
            .to_vec(|k, v| format!("\"{k}\" {v}"))
            .iter()
            // Let's avoid promotable attributes for sake of simplicity.
            .filter(|x| !x.contains("pos"))
        {
            s = format!("(map-insert {} {})", s, attribute);
        }
        if let Some(i) = latency {
            s = format!(r#"(map-insert {} "static" {})"#, s, i);
        }
        if let Some(i) = promotable {
            s = format!(r#"(map-insert {} "promotable" {})"#, s, i);
        }
        format!("(Attributes {})", s)
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
        for group in comp.get_static_groups().iter() {
            Self::write_static_group(&group.borrow(), f)?;
        }
        for group in comp.comb_groups.iter() {
            todo!(
                "`combinational-group` is not supported in CalyxEgg: {:?}",
                group
            )
        }
        // Write the continuous assignments
        for assignment in &comp.continuous_assignments {
            todo!(
                "`continuous assignment` is not supported in CalyxEgg: {:?}",
                assignment
            )
        }

        // Add the control program
        if matches!(&*comp.control.borrow(), ir::Control::Empty(..)) {
            todo!("`empty` control is not supported in CalyxEgg")
        }
        write!(f, "(let {} ", comp.name)?;
        Self::write_control(&comp.control.borrow(), f)?;
        write!(f, ")")?;
        Ok(())
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
                    .sorted_by(Ord::cmp)
                    .map(|x| format!("c-{}", x))
                    .collect_vec()
                    .join(" ")
            )?;
        }
        write!(f, "))")
    }

    // TODO(cgyurgyik): Reduce duplication between dynamic/static IR.
    pub fn write_static_group<F: io::Write>(
        group: &ir::StaticGroup,
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
                    .sorted_by(Ord::cmp)
                    .map(|x| format!("c-{}", x))
                    .collect_vec()
                    .join(" ")
            )?;
        }
        write!(f, "))")
    }

    fn write_control_list<F: io::Write>(
        f: &mut F,
        name: &str,
        statements: &Vec<ir::Control>,
        attr: &calyx_ir::Attributes,
    ) -> io::Result<()> {
        write!(f, "({} {}", name, Self::format_attributes(attr, None, None))?;

        for stmt in statements {
            write!(f, " (Cons ")?;
            Self::write_control(stmt, f)?;
        }
        write!(f, " (Nil){}", ")".repeat(statements.len()))?;
        write!(f, ")")?;
        Ok(())
    }

    fn write_static_control_list<F: io::Write>(
        f: &mut F,
        name: &str,
        statements: &Vec<ir::StaticControl>,
        attr: &calyx_ir::Attributes,
        latency: u64,
    ) -> io::Result<()> {
        write!(
            f,
            "({} {}",
            name,
            Self::format_attributes(attr, Some(latency), None)
        )?;

        for stmt in statements {
            write!(f, " (Cons ")?;
            Self::write_static_control(stmt, f)?;
        }
        write!(f, " (Nil){}", ")".repeat(statements.len()))?;

        write!(f, ")")?;
        Ok(())
    }

    pub fn write_repeat<F: io::Write>(
        attributes: &calyx_ir::Attributes,
        body: &calyx_ir::Control,
        num_repeats: &u64,
        f: &mut F,
    ) -> io::Result<()> {
        write!(
            f,
            "(Repeat {} {}",
            Self::format_attributes(attributes, None, None),
            num_repeats,
        )?;
        Self::write_control(body, f)?;
        write!(f, ")")?;
        Ok(())
    }

    pub fn write_static_control<F: io::Write>(
        control: &ir::StaticControl,
        f: &mut F,
    ) -> io::Result<()> {
        let attr = control.get_attributes();
        match control {
            ir::StaticControl::Enable(ir::StaticEnable { group, .. }) => {
                // TODO(cgyurgyik): Add promotable attribute if it exists in the group.
                write!(
                    f,
                    "(Enable {} {})",
                    group.borrow().name().id,
                    Self::format_attributes(attr, None, None),
                )
            }
            ir::StaticControl::Seq(calyx_ir::StaticSeq {
                stmts,
                latency,
                ..
            }) => {
                Self::write_static_control_list(
                    f, "Seq", stmts, attr, *latency,
                )?;
                Ok(())
            }
            ir::StaticControl::Par(calyx_ir::StaticPar {
                stmts,
                latency,
                ..
            }) => {
                Self::write_static_control_list(
                    f, "Par", stmts, attr, *latency,
                )?;
                Ok(())
            }
            ir::StaticControl::Repeat(calyx_ir::StaticRepeat {
                body,
                num_repeats,
                latency,
                ..
            }) => {
                write!(
                    f,
                    "(Repeat {} {} ",
                    Self::format_attributes(attr, Some(*latency), None),
                    num_repeats
                )?;
                Self::write_static_control(body, f)?;
                write!(f, ")")?;
                Ok(())
            }
            _ => todo!("`static control`: {:?} is not implemented", control),
        }
    }

    /// Format and write a control program
    pub fn write_control<F: io::Write>(
        control: &ir::Control,
        f: &mut F,
    ) -> io::Result<()> {
        let attr = control.get_attributes();
        match control {
            ir::Control::Enable(ir::Enable { group, .. }) => {
                write!(
                    f,
                    "(Enable {} {})",
                    group.borrow().name().id,
                    Self::format_attributes(
                        attr,
                        None,
                        group
                            .borrow()
                            .attributes
                            .get(calyx_ir::NumAttr::Promotable)
                    ),
                )
            }
            ir::Control::Par(ir::Par { stmts, .. }) => {
                Self::write_control_list(f, "Par", stmts, attr)?;
                Ok(())
            }
            ir::Control::Seq(ir::Seq { stmts, .. }) => {
                Self::write_control_list(f, "Seq", stmts, attr)?;
                Ok(())
            }
            ir::Control::Static(static_control) => {
                Self::write_static_control(static_control, f)?;
                Ok(())
            }
            ir::Control::Invoke(ir::Invoke { .. }) => {
                todo!("`invoke` is not supported in CalyxEgg")
            }
            ir::Control::Repeat(ir::Repeat {
                attributes,
                body,
                num_repeats,
            }) => {
                Self::write_repeat(attributes, body, num_repeats, f)?;
                Ok(())
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
