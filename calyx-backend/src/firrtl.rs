//! FIRRTL backend for the Calyx compiler.
//!
//! Transforms an [`ir::Context`](crate::ir::Context) into a formatted string that represents a
//! valid FIRRTL program.

use crate::{traits::Backend, VerilogBackend};
use calyx_ir::{self as ir};
use calyx_utils::{CalyxResult, Error, OutputFile};
use std::io;
use std::time::Instant;

pub(super) const SPACING: &str = "    ";

/// Implements a simple FIRRTL backend. The backend only accepts Calyx programs with no control
/// and no groups.
#[derive(Default)]
pub struct FirrtlBackend;

impl Backend for FirrtlBackend {
    fn name(&self) -> &'static str {
        "firrtl"
    }

    fn link_externs(
        _prog: &calyx_ir::Context,
        _write: &mut calyx_utils::OutputFile,
    ) -> calyx_utils::CalyxResult<()> {
        Ok(()) // FIXME: Need to implement
    }

    fn validate(prog: &calyx_ir::Context) -> calyx_utils::CalyxResult<()> {
        VerilogBackend::validate(prog) // FIXME: would this work if we wanted to check for the same things?
    }

    fn emit(ctx: &ir::Context, file: &mut OutputFile) -> CalyxResult<()> {
        let out = &mut file.get_write();
        let comps = ctx.components.iter().try_for_each(|comp| {
            // Time the generation of the component.
            let time = Instant::now();
            let out = emit_component(comp, out);
            log::info!("Generated `{}` in {:?}", comp.name, time.elapsed());
            out
        });
        comps.map_err(|err| {
            let std::io::Error { .. } = err;
            Error::write_error(format!(
                "File not found: {}",
                file.as_path_string()
            ))
        })
    }
}

// TODO: Ask about the other backend configurations in verilog.rs and see if I need any of it
fn emit_component<F: io::Write>(
    comp: &ir::Component,
    f: &mut F,
) -> io::Result<()> {
    writeln!(f, "circuit {}:", comp.name)?;
    writeln!(f, "{}module {}:", SPACING, comp.name)?;

    // Inputs and Outputs
    let sig = comp.signature.borrow();
    for (_idx, port_ref) in sig.ports.iter().enumerate() {
        let port = port_ref.borrow();
        let direction_string =
        // NOTE: The signature port definitions are reversed inside the component.
        match port.direction {
            ir::Direction::Input => {"output"}
            ir::Direction::Output => {"input"}
            ir::Direction::Inout => {
                panic!("Unexpected Inout port on Component: {}", port.name)
            }
        };
        // FIXME: Hack to get clock declaration right. Should check for attribute name instead.
        if port.name == "clk" {
            writeln!(
                f,
                "{}{} {}: Clock",
                SPACING.repeat(2),
                direction_string,
                port.name
            )?;
        } else {
            writeln!(
                f,
                "{}{} {}: UInt<{}>",
                SPACING.repeat(2),
                direction_string,
                port.name,
                port.width
            )?;
        }
    }

    // Add a COMPONENT START: <name> anchor before any code in the component
    writeln!(f, "{}; COMPONENT START: {}", SPACING.repeat(2), comp.name)?;

    // TODO: Cells. NOTE: leaving this one for last

    // Emit assignments
    for asgn in &comp.continuous_assignments {
        let mut num_indent = 3; // if we have a guard, then the assignment should be nested
        match asgn.guard.as_ref() {
            ir::Guard::True => {
                // Simple assignment with no guard
                num_indent = 2;
            }
            _ => {
                // need to write out the guard.
                let guard_string = get_guard_string(asgn.guard.as_ref());
                writeln!(f, "{}when {}:", SPACING.repeat(2), guard_string)?;
            }
        }
        let _ = write_assignment(asgn, f, num_indent, false);
    }

    // Add COMPONENT END: <name> anchor
    writeln!(f, "{}; COMPONENT END: {}", SPACING.repeat(2), comp.name)?;

    Ok(())
}

// recursive function that writes the FIRRTL representation for a guard.
fn get_guard_string(guard: &ir::Guard<ir::Nothing>) -> String {
    match guard {
        ir::Guard::Or(l, r) => {
            let l_str = get_guard_string(l.as_ref());
            let r_str = get_guard_string(r.as_ref());
            format!("or({}, {})", l_str, r_str)
        }
        ir::Guard::And(l, r) => {
            let l_str = get_guard_string(l.as_ref());
            let r_str = get_guard_string(r.as_ref());
            format!("and({}, {})", l_str, r_str)
        }
        ir::Guard::Not(g) => {
            let g_str = get_guard_string(g);
            format!("not({})", g_str)
        }
        ir::Guard::True => String::from(""),
        ir::Guard::CompOp(op, l, r) => {
            let l_str = get_port_string(&l.borrow());
            let r_str = get_port_string(&r.borrow());
            let op_str = match op {
                ir::PortComp::Eq => "eq",
                ir::PortComp::Neq => "neq",
                ir::PortComp::Gt => "gt",
                ir::PortComp::Lt => "lt",
                ir::PortComp::Geq => "geq",
                ir::PortComp::Leq => "leq",
            };
            format!("{}({}, {})", op_str, l_str, r_str)
        }
        ir::Guard::Port(port) => get_port_string(&port.borrow().clone()),
        ir::Guard::Info(_) => {
            panic!("guard should not have info") // FIXME: What should I write here?
        }
    }
}

// returns the FIRRTL translation of a port.
fn get_port_string(port: &calyx_ir::Port) -> String {
    match &port.parent {
        ir::PortParent::Cell(cell) => {
            let parent_ref = cell.upgrade();
            let parent = parent_ref.borrow();
            match parent.prototype {
                ir::CellType::Constant { val, width: _ } => {
                    format!("UInt({})", val)
                }
                ir::CellType::ThisComponent => String::from(port.name.as_ref()),
                _ => {
                    format!("{}.{}", parent.name().as_ref(), port.name.as_ref())
                }
            }
        }
        _ => {
            unreachable!()
        }
    }
}

// Writes a FIRRTL assignment
fn write_assignment<F: io::Write>(
    asgn: &ir::Assignment<ir::Nothing>,
    f: &mut F,
    num_indent: usize,
    default_assignment: bool,
) -> CalyxResult<()> {
    let dest_port = asgn.dst.borrow();
    let mut dest_string = SPACING.repeat(num_indent);
    // This match may be worth keeping (instead of replacing with get_port_string()), since dst should never be a Constant.
    match &dest_port.parent {
        ir::PortParent::Cell(cell) => {
            let parent_ref = cell.upgrade();
            let parent = parent_ref.borrow();
            match parent.prototype {
                ir::CellType::ThisComponent => {
                    dest_string.push_str(dest_port.name.as_ref());
                }
                _ => {
                    let formatted = format!(
                        "{}.{}",
                        parent.name().as_ref(),
                        dest_port.name.as_ref()
                    );
                    dest_string.push_str(&formatted);
                }
            }
        }
        _ => {
            unreachable!()
        }
    }
    let src_string;
    if !default_assignment {
        // We will assign to 0 if
        let source_port = asgn.src.borrow();
        src_string = get_port_string(&source_port);
    } else {
        src_string = String::from("UInt(0)");
    }
    writeln!(f, "{} <= {}", dest_string, src_string)?;
    Ok(())
}
