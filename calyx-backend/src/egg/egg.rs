//! Generation for the egglog backend of the Calyx compiler.
use super::calyx_to_egg::ToEggPrinter;
use crate::egg::utils;
use crate::traits::Backend;
use calyx_frontend::GetAttributes;
use calyx_ir::{self as ir};
use calyx_utils::Error;
use itertools::Itertools;
use std::collections::HashSet;
use std::io::Write;
use std::io::{self, Read};

#[derive(Default)]
pub struct EggBackend;

impl Backend for EggBackend {
    fn name(&self) -> &'static str {
        "egg"
    }

    fn validate(_prog: &ir::Context) -> calyx_utils::CalyxResult<()> {
        Ok(())
    }

    fn emit(
        ctx: &ir::Context,
        file: &mut calyx_utils::OutputFile,
    ) -> calyx_utils::CalyxResult<()> {
        {
            let f = &mut file.get_write();
            if ctx.components.len() > 1 {
                todo!("multiple components not supported in CalyxEgg")
            }

            let component: &calyx_ir::Component = ctx
                .components
                .first()
                .unwrap_or_else(|| panic!("no components found"));

            ToEggPrinter::write_component(component, f)?;
            writeln!(f)?;

            if ctx.bc.display_egraph {
                let mut ntf = tempfile::NamedTempFile::new()?;
                ToEggPrinter::write_component(component, &mut ntf)?;

                let mut buf = String::new();
                ntf.read_to_string(&mut buf)?;
                let (term, termdag) = utils::extract_egglog(
                    ctx.bc.display_egraph,
                    component.name.id.into(),
                    &buf,
                );
            }
            Ok(())
        }
        .map_err(|err| {
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
