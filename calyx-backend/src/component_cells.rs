//! Component cells backend for the Calyx compiler.
//!
//! Transforms an [`ir::Context`](crate::ir::Context) into a JSON file that
//! records all of the component (non-primitive) cells in a program.
//! This is used for multi-component program profiling.
//! Usage: -b component_cells [-o <OUTPUT_FILE>]
//! Adapted from resources.rs.

use std::{collections::HashSet, io};

use crate::traits::Backend;
use calyx_ir::{self as ir, Id};
use calyx_utils::{CalyxResult, OutputFile};
use serde::Serialize;

#[derive(Default)]
pub struct ComponentCellsBackend;

impl Backend for ComponentCellsBackend {
    fn name(&self) -> &'static str {
        "component_cells"
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

        let mut component_info: HashSet<ComponentInfo> = HashSet::new();

        gen_component_info(ctx, main_comp, true, &mut component_info);

        write_json(component_info.clone(), file)?;

        Ok(())
    }
}

fn id_serialize_passthrough<S>(id: &Id, ser: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    id.to_string().serialize(ser)
}

#[derive(PartialEq, Eq, Hash, Clone, Serialize)]
struct ComponentInfo {
    #[serde(serialize_with = "id_serialize_passthrough")]
    pub component: Id,
    pub is_main_component: bool,
    pub cell_info: Vec<ComponentCellInfo>,
}

#[derive(PartialEq, Eq, Hash, Clone, Serialize)]
struct ComponentCellInfo {
    #[serde(serialize_with = "id_serialize_passthrough")]
    pub cell_name: Id,
    #[serde(serialize_with = "id_serialize_passthrough")]
    pub component_name: Id,
}

/// Accumulates a set of components to the cells that they contain
/// in the program with entrypoint `main_comp`. The contained cells
/// are denoted with the name of the cell and the name of the component
/// the cell is associated with.
fn gen_component_info(
    ctx: &ir::Context,
    comp: &ir::Component,
    is_main_comp: bool,
    component_info: &mut HashSet<ComponentInfo>,
) {
    let mut curr_comp_info = ComponentInfo {
        component: comp.name,
        is_main_component: is_main_comp,
        cell_info: Vec::new(),
    };
    for cell in comp.cells.iter() {
        let cell_ref = cell.borrow();
        if let ir::CellType::Component { name } = cell_ref.prototype {
            curr_comp_info.cell_info.push(ComponentCellInfo {
                cell_name: cell_ref.name(),
                component_name: name,
            });
            let component = ctx
                .components
                .iter()
                .find(|comp| comp.name == name)
                .unwrap();
            gen_component_info(ctx, component, false, component_info);
        }
    }
    component_info.insert(curr_comp_info);
}

/// Write the collected set of component information to a JSON file.
fn write_json(
    component_info: HashSet<ComponentInfo>,
    file: &mut OutputFile,
) -> Result<(), io::Error> {
    let created_vec: Vec<ComponentInfo> = component_info.into_iter().collect();
    serde_json::to_writer_pretty(file.get_write(), &created_vec)?;
    Ok(())
}
