//! FIRRTL backend for the Calyx compiler.
//!
//! Transforms an [`ir::Context`](crate::ir::Context) into a formatted string that represents a
//! valid FIRRTL program.

use crate::traits::Backend;
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
        prog: &calyx_ir::Context,
        write: &mut calyx_utils::OutputFile,
    ) -> calyx_utils::CalyxResult<()> {
        todo!("Ayaka: Not yet implemented");
    }

    fn run(
        &self,
        prog: calyx_ir::Context,
        mut file: calyx_utils::OutputFile,
    ) -> calyx_utils::CalyxResult<()> {
        todo!("Ayaka: Not yet implemented");
    }

    fn validate(prog: &calyx_ir::Context) -> calyx_utils::CalyxResult<()> {
        todo!("Ayaka: Not yet implemented");
    }

    fn emit(ctx: &ir::Context, file: &mut OutputFile) -> CalyxResult<()> {
        log::info!("Writing!");
        Ok(())
    }
}
