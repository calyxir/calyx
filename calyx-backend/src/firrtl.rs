//! FIRRTL backend for the Calyx compiler.
//!
//! Transforms an [`ir::Context`](crate::ir::Context) into a formatted string that represents a
//! valid FIRRTL program.

use crate::{traits::Backend, VerilogBackend};
use calyx_ir::{self as ir};
use calyx_utils::{CalyxResult, Error, OutputFile};
// use ir::Nothing;
// use itertools::Itertools;
use std::io;
// use std::{collections::HashMap, rc::Rc};
use std::time::Instant;
// use vast::v17::ast as v;

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
            let out = emit_component(
                comp,
                ctx.bc.synthesis_mode,
                ctx.bc.enable_verification,
                ctx.bc.flat_assign,
                out,
            );
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

fn emit_component<F: io::Write>(
    comp: &ir::Component,
    _synthesis_mode: bool,
    _enable_verification: bool,
    _flat_assign: bool,
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
                direction_string,
                port.name,
                port.width.to_string()
            )?;
        }
    }

    // Add a COMPONENT START: <name> anchor before any code in the component
    writeln!(f, "; COMPONENT START: {}", comp.name)?;

    // TODO: Cells

    // TODO: Guards

    // TODO: assignments

    // Add COMPONENT END: <name> anchor
    writeln!(f, "; COMPONENT END: {}\n", comp.name)?;

    Ok(())
}
