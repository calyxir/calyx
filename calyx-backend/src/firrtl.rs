//! FIRRTL backend for the Calyx compiler.
//!
//! Transforms an [`ir::Context`](crate::ir::Context) into a formatted string that represents a
//! valid FIRRTL program.

use crate::{traits::Backend, VerilogBackend};
use calyx_ir::{self as ir};
use calyx_utils::{CalyxResult, OutputFile};
use std::io;

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
        for comp in ctx.components.iter() {
            emit_component(comp, out)?
        }
        Ok(())
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
        if port.has_attribute(ir::BoolAttr::Clk) {
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

    for asgn in &comp.continuous_assignments {
        // TODO: guards
        match asgn.guard.as_ref() {
            ir::Guard::Or(_, _) => todo!(),
            ir::Guard::And(_, _) => todo!(),
            ir::Guard::Not(_) => todo!(),
            ir::Guard::True => {
                // Simple assignment with no guard
                let _ = write_assignment(asgn, f);
            }
            ir::Guard::CompOp(_, _, _) => todo!(),
            ir::Guard::Port(_) => {}
            ir::Guard::Info(_) => todo!(),
        }
    }

    // Add COMPONENT END: <name> anchor
    writeln!(f, "{}; COMPONENT END: {}", SPACING.repeat(2), comp.name)?;

    Ok(())
}

// Writes a FIRRTL assignment
fn write_assignment<F: io::Write>(
    asgn: &ir::Assignment<ir::Nothing>,
    f: &mut F,
) -> CalyxResult<()> {
    let dest_port = asgn.dst.borrow();
    let dest_string = get_port_string(&dest_port, true);
    let source_port = asgn.src.borrow();
    let src_string = get_port_string(&source_port, false);
    writeln!(f, "{}{} <= {}", SPACING.repeat(2), dest_string, src_string)?;
    Ok(())
}

// returns the FIRRTL translation of a port.
// if is_dst is true, then the port is a destination of an assignment, and shouldn't be a constant.
fn get_port_string(port: &calyx_ir::Port, is_dst: bool) -> String {
    match &port.parent {
        ir::PortParent::Cell(cell) => {
            let parent_ref = cell.upgrade();
            let parent = parent_ref.borrow();
            match parent.prototype {
                ir::CellType::Constant { val, width: _ } => {
                    if !is_dst {
                        format!("UInt({})", val)
                    } else {
                        unreachable!()
                    }
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
