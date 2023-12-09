//! FIRRTL backend for the Calyx compiler.
//!
//! Transforms an [`ir::Context`](crate::ir::Context) into a formatted string that represents a
//! valid FIRRTL program.

use crate::{traits::Backend, verilog, VerilogBackend};
use calyx_ir::{self as ir, Control, FlatGuard, Group, Guard, GuardRef, RRC};
use calyx_utils::{CalyxResult, Error, OutputFile};
// use ir::Nothing;
// use itertools::Itertools;
// use std::io;
// use std::{collections::HashMap, rc::Rc};
// use std::{fs::File, time::Instant};
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
        todo!("Not yet implemented");
    }

    fn validate(prog: &calyx_ir::Context) -> calyx_utils::CalyxResult<()> {
        VerilogBackend::validate(prog) // FIXME: would this work if we wanted to check for the same things?
    }

    fn emit(_ctx: &ir::Context, _file: &mut OutputFile) -> CalyxResult<()> {
        todo!("Not yet implemented");
    }
}
