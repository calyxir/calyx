use super::traits::Backend;
use crate::{errors::FutilResult, ir};

#[derive(Default)]
pub struct XilinxInterfaceBackend;

impl Backend for XilinxInterfaceBackend {
    fn name(&self) -> &'static str {
        "xilinx-axi"
    }

    fn validate(_ctx: &ir::Context) -> FutilResult<()> {
        Ok(())
    }

    fn link_externs(
        _prog: &ir::Context,
        _write: &mut crate::utils::OutputFile,
    ) -> FutilResult<()> {
        Ok(())
    }

    fn emit(
        _prog: &ir::Context,
        file: &mut crate::utils::OutputFile,
    ) -> FutilResult<()> {
        write!(file.get_write(), "test")?;

        Ok(())
    }
}
