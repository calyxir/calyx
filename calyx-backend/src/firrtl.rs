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
    writeln!(f, "   module {}:", comp.name)?;

    // TODO: Inputs and Outputs
    let sig = comp.signature.borrow();
    for (_idx, port_ref) in sig.ports.iter().enumerate() {
        let port = port_ref.borrow();
        let direction_string =
        // NOTE: The signature port definitions are reversed inside the component.
        match port.direction {
            ir::Direction::Input => {"output"}
            ir::Direction::Output => {"input"}
            ir::Direction::Inout => {
                panic!("Unexpected Inout port on Component: {}", port.name) // FIXME
            }
        };
        // FIXME: Hack to get clock declaration right. Should check for attribute name instead.
        if port.name == "clk" {
            writeln!(f, "{} {}: Clock", direction_string, port.name)?;
        } else {
            writeln!(
                f,
                "{} {}: UInt<{}>",
                direction_string, port.name, port.width
            )?;
        }
    }

    // Add a COMPONENT START: <name> anchor before any code in the component
    writeln!(f, "; COMPONENT START: {}", comp.name)?;

    // TODO: Cells. NOTE: leaving this one for last

    // TODO: simple assignments

    // below code is borrowed from verilog.rs, but pretty confused.
    // let mut map: HashMap<_, (RRC<ir::Port>, Vec<_>)> = HashMap::new();
    for asgn in &comp.continuous_assignments {
        match asgn.guard.as_ref() {
            ir::Guard::Or(_, _) => todo!(),
            ir::Guard::And(_, _) => todo!(),
            ir::Guard::Not(_) => todo!(),
            ir::Guard::True =>
            // There is no guard here
            {
                // FIXME: This is just a first pass to get things working. Definitely need to fix
            }
            ir::Guard::CompOp(_, _, _) => todo!(),
            ir::Guard::Port(port_ref) => {
                // FIXME: remove
                let borrow = port_ref.borrow();
                writeln!(f, "when {}:", borrow.canonical())?;
            }
            ir::Guard::Info(_) => todo!(),
        }
        write_assignment(asgn, f);
    }

    // Add COMPONENT END: <name> anchor
    writeln!(f, "; COMPONENT END: {}\n", comp.name)?;

    Ok(())
}

fn write_assignment<F: io::Write>(
    asgn: &ir::Assignment<ir::Nothing>,
    f: &mut F,
) {
    let dest_port = asgn.dst.borrow();
    match &dest_port.parent {
        ir::PortParent::Cell(cell) => {
            let parent_ref = cell.upgrade();
            let parent = parent_ref.borrow();
            match parent.prototype {
                ir::CellType::ThisComponent => {
                    write!(f, "{}", dest_port.name.as_ref());
                }
                _ => {
                    write!(
                        f,
                        "{}.{}",
                        parent.name().as_ref(),
                        dest_port.name.as_ref()
                    );
                }
            }
        }
        _ => {
            unreachable!()
        }
    }
    write!(f, " <= ");
    let source_port = asgn.src.borrow();
    match &source_port.parent {
        ir::PortParent::Cell(cell) => {
            let parent_ref = cell.upgrade();
            let parent = parent_ref.borrow();
            match parent.prototype {
                ir::CellType::Constant { val, width } => {
                    write!(f, "UInt({})", val.to_string());
                }
                ir::CellType::ThisComponent => {
                    write!(f, "{}", asgn.src.borrow().name);
                }
                _ => {
                    write!(
                        f,
                        "{}.{}",
                        parent.name().as_ref(),
                        source_port.name.as_ref()
                    );
                }
            }
        }
        _ => {
            unreachable!()
        }
    }
    writeln!(f, "");
}
