//! Primitive instantiations backend for the Calyx compiler.
//!
//! Transforms an [`ir::Context`](crate::ir::Context) into a JSON file that
//! records the unique primitive instantiations in a program.
//! Usage: -b primitive-inst [-o <OUTPUT_FILE>]
//! Adapted from resources.rs.

use std::{collections::HashSet, io};

use crate::traits::Backend;
use calyx_ir as ir;
use calyx_utils::{CalyxResult, OutputFile};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct PrimitiveUsesBackend;

impl Backend for PrimitiveUsesBackend {
    fn name(&self) -> &'static str {
        "primitive_uses"
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
        let main_comp = ctx.entrypoint();

        let mut primitive_set: HashSet<PrimitiveUse> = HashSet::new();

        gen_primitive_set(ctx, main_comp, &mut primitive_set);

        write_json(primitive_set.clone(), file)?;

        Ok(())
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
struct PrimitiveUse {
    name: String,
    params: Vec<PrimitiveParam>,
}

#[derive(PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
struct PrimitiveParam {
    param_name: String,
    param_value: u64,
}

/// Accumulates a set with each primitive with a given set of parameters
/// in the program with entrypoint `main_comp`.
fn gen_primitive_set(
    ctx: &ir::Context,
    main_comp: &ir::Component,
    primitive_set: &mut HashSet<PrimitiveUse>,
) {
    for cell in main_comp.cells.iter() {
        let cell_ref = cell.borrow();
        match &cell_ref.prototype {
            ir::CellType::Primitive {
                name,
                param_binding,
                ..
            } => {
                let curr_params = param_binding
                    .iter()
                    .map(|(param_name, param_size)| PrimitiveParam {
                        param_name: param_name.to_string(),
                        param_value: *param_size,
                    })
                    .collect();
                let curr_primitive = PrimitiveUse {
                    name: name.to_string(),
                    params: curr_params,
                };
                (*primitive_set).insert(curr_primitive);
            }
            ir::CellType::Component { name } => {
                let component = ctx
                    .components
                    .iter()
                    .find(|comp| comp.name == name)
                    .unwrap();
                gen_primitive_set(ctx, component, primitive_set);
            }
            _ => (),
        }
    }
}

/// Write the collected set of primitive instantiations to a JSON file.
fn write_json(
    primitive_set: HashSet<PrimitiveUse>,
    file: &mut OutputFile,
) -> Result<(), io::Error> {
    let created_vec: Vec<PrimitiveUse> = primitive_set.into_iter().collect();
    serde_json::to_writer_pretty(file.get_write(), &created_vec)?;
    Ok(())
}
