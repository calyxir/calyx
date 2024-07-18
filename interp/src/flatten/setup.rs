use calyx_frontend as frontend;
use calyx_ir as ir;
use calyx_opt::pass_manager::PassManager;
use std::path::{Path, PathBuf};

use crate::{
    debugger::source::structures::NewSourceMap, errors::InterpreterResult,
};

use super::structures::context::Context;

#[inline]
fn do_setup(
    file: &Option<PathBuf>,
    lib_path: &Path,
    skip_verification: bool,
) -> InterpreterResult<(Context, InterpreterResult<NewSourceMap>)> {
    // Construct IR
    let ws = frontend::Workspace::construct(file, lib_path)?;
    let mut ctx = ir::from_ast::ast_to_ir(ws)?;
    let pm = PassManager::default_passes()?;

    if !skip_verification {
        pm.execute_plan(&mut ctx, &["validate".to_string()], &[], &[], false)?;
    }
    pm.execute_plan(
        &mut ctx,
        &["metadata-table-generation".to_string()],
        &[],
        &[],
        false,
    )?;

    let mapping = ctx
        .metadata
        .as_ref()
        .map(|metadata| {
            crate::debugger::source::new_parser::parse_metadata(metadata)
        })
        .unwrap_or_else(|| {
            Err(crate::errors::InterpreterError::MissingMetaData.into())
        });

    // general setup
    Ok((crate::flatten::flat_ir::translate(&ctx), mapping))
}

pub fn setup_simulation(
    file: &Option<PathBuf>,
    lib_path: &Path,
    skip_verification: bool,
) -> InterpreterResult<Context> {
    let (ctx, _) = do_setup(file, lib_path, skip_verification)?;
    Ok(ctx)
}

pub fn setup_simulation_with_metadata(
    file: &Option<PathBuf>,
    lib_path: &Path,
    skip_verification: bool,
) -> InterpreterResult<(Context, NewSourceMap)> {
    let (ctx, mapping) = do_setup(file, lib_path, skip_verification)?;
    Ok((ctx, mapping?))
}
