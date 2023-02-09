//! Pretty-printer for Calyx syntax.
//! For now, only prints s-expressions.

use crate::backend::traits::Backend;
use calyx::{errors::CalyxResult, ir, utils::OutputFile};

#[derive(Default)]
pub struct PrintBackend;

impl Backend for PrintBackend {
    fn name(&self) -> &'static str {
        "print"
    }

    /// OK to run this analysis on any Calyx program
    fn validate(_ctx: &ir::Context) -> CalyxResult<()> {
        Ok(())
    }

    /// Don't need to take care of this for this pass
    fn link_externs(
        _ctx: &ir::Context,
        _file: &mut OutputFile,
    ) -> CalyxResult<()> {
        Ok(())
    }

    fn emit(ctx: &ir::Context, file: &mut OutputFile) -> CalyxResult<()> {
        let out = &mut file.get_write();
        writeln!(out, "{}", serde_sexpr::to_string(ctx).unwrap())?;

        Ok(())
    }
}
