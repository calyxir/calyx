use argh::FromArgs;
use calyx_frontend as frontend;
use calyx_ir::{self as ir, Id};
use calyx_utils::{CalyxResult, OutputFile};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::{collections::HashSet, io};

/// Tool to obtain list of names and original component names for all non-primitive cells in each component.

#[derive(FromArgs)]
/// Path for library and path for file to read from
struct Args {
    /// file path to read data from
    #[argh(positional, from_str_fn(read_path))]
    file_path: Option<PathBuf>,

    /// library path
    #[argh(option, short = 'l', default = "Path::new(\".\").into()")]
    pub lib_path: PathBuf,

    /// output file
    #[argh(option, short = 'o', default = "OutputFile::Stdout")]
    pub output: OutputFile,
}

fn read_path(path: &str) -> Result<PathBuf, String> {
    Ok(Path::new(path).into())
}

fn main() -> CalyxResult<()> {
    let p: Args = argh::from_env();

    let ws = frontend::Workspace::construct(&p.file_path, &[p.lib_path])?;

    let ctx: ir::Context = ir::from_ast::ast_to_ir(
        ws,
        ir::from_ast::AstConversionConfig::default(),
    )?;

    let main_comp = ctx.entrypoint();

    let mut component_info: HashSet<ComponentInfo> = HashSet::new();
    gen_component_info(&ctx, main_comp, true, &mut component_info);
    write_json(component_info.clone(), p.output)?;
    Ok(())
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
    pub primitive_info: Vec<PrimitiveCellInfo>,
}

#[derive(PartialEq, Eq, Hash, Clone, Serialize)]
struct ComponentCellInfo {
    #[serde(serialize_with = "id_serialize_passthrough")]
    pub cell_name: Id,
    #[serde(serialize_with = "id_serialize_passthrough")]
    pub component_name: Id,
}

#[derive(PartialEq, Eq, Hash, Clone, Serialize)]
struct PrimitiveCellInfo {
    #[serde(serialize_with = "id_serialize_passthrough")]
    pub cell_name: Id,
    #[serde(serialize_with = "id_serialize_passthrough")]
    pub primitive_type: Id,
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
        primitive_info: Vec::new(),
    };
    for cell in comp.cells.iter() {
        let cell_ref = cell.borrow();
        match &cell_ref.prototype {
            calyx_ir::CellType::Primitive { name, .. } => {
                curr_comp_info.primitive_info.push(PrimitiveCellInfo {
                    cell_name: cell_ref.name(),
                    primitive_type: *name,
                })
            }
            calyx_ir::CellType::Component { name } => {
                curr_comp_info.cell_info.push(ComponentCellInfo {
                    cell_name: cell_ref.name(),
                    component_name: *name,
                });
                let component = ctx
                    .components
                    .iter()
                    .find(|comp| comp.name == name)
                    .unwrap();
                gen_component_info(ctx, component, false, component_info);
            }
            _ => {}
        }
        // if let ir::CellType::Component { name } = cell_ref.prototype {
        //     curr_comp_info.cell_info.push(ComponentCellInfo {
        //         cell_name: cell_ref.name(),
        //         component_name: name,
        //     });
        //     let component = ctx
        //         .components
        //         .iter()
        //         .find(|comp| comp.name == name)
        //         .unwrap();
        //     gen_component_info(ctx, component, false, component_info);
        // }
    }
    component_info.insert(curr_comp_info);
}

/// Write the collected set of component information to a JSON file.
fn write_json(
    component_info: HashSet<ComponentInfo>,
    mut file: OutputFile,
) -> Result<(), io::Error> {
    let created_vec: Vec<ComponentInfo> = component_info.into_iter().collect();
    serde_json::to_writer_pretty(file.get_write(), &created_vec)?;
    Ok(())
}
