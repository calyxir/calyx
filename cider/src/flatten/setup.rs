use calyx_frontend as frontend;
use calyx_ir as ir;
use calyx_opt::pass_manager::PassManager;
use std::path::{Path, PathBuf};

use crate::{debugger::source::structures::NewSourceMap, errors::CiderResult};

use super::structures::context::Context;

#[inline]
fn do_setup(
    file: &Option<PathBuf>,
    lib_path: &Path,
    skip_verification: bool,
    gen_metadata: bool,
) -> CiderResult<(Context, CiderResult<NewSourceMap>)> {
    // Construct IR
    let ws = frontend::Workspace::construct(file, &[lib_path.to_path_buf()])?;
    let mut ctx = ir::from_ast::ast_to_ir(ws)?;
    let pm = PassManager::default_passes()?;

    if !skip_verification {
        pm.execute_plan(&mut ctx, &["validate".to_string()], &[], &[], false)?;
    }

    let mapping = if gen_metadata {
        pm.execute_plan(
            &mut ctx,
            &["metadata-table-generation".to_string()],
            &[],
            &[],
            false,
        )?;
        ctx.metadata
            .as_ref()
            .map(|metadata| {
                crate::debugger::source::new_parser::parse_metadata(metadata)
            })
            .unwrap_or_else(|| {
                Err(crate::errors::CiderError::MissingMetaData.into())
            })
    } else {
        Err(crate::errors::CiderError::MissingMetaData.into())
    };

    // general setup
    Ok((crate::flatten::flat_ir::translate(&ctx), mapping))
}

/// This function sets up the simulation context for the given program. This is
/// meant to be used in contexts where calyx metadata is not required. For other
/// cases, use [setup_simulation_with_metadata]
pub fn setup_simulation(
    file: &Option<PathBuf>,
    lib_path: &Path,
    skip_verification: bool,
) -> CiderResult<Context> {
    let (ctx, _) = do_setup(file, lib_path, skip_verification, false)?;
    Ok(ctx)
}

/// Constructs the simulation context for the given program. Additionally
/// attempts to construct the metadata table for the program.
///
/// For cases where the metadata is not required, use [setup_simulation], which
/// has less of a performance impact.
pub fn setup_simulation_with_metadata(
    file: &Option<PathBuf>,
    lib_path: &Path,
    skip_verification: bool,
) -> CiderResult<(Context, NewSourceMap)> {
    let (ctx, mapping) = do_setup(file, lib_path, skip_verification, true)?;
    Ok((ctx, mapping?))
}
