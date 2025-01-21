use argh::FromArgs;
use calyx_frontend::{
    self as frontend, MetadataTable, PositionId, SetAttr, SetAttribute,
    SourceLocation,
};
use calyx_ir::{self as ir, Id};
use calyx_utils::{CalyxResult, OutputFile};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

// emits a JSON mapping components, cells, and groups to their @pos filenames and line numbers

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
    pub filename: String,
    pub linenum: usize,
    pub cells: Vec<PosInfo>,
    pub groups: Vec<PosInfo>,
}

#[derive(PartialEq, Eq, Hash, Clone, Serialize)]
struct PosInfo {
    #[serde(serialize_with = "id_serialize_passthrough")]
    pub name: Id,
    pub filename: String,
    pub linenum: usize,
}

struct ComponentPosIds {
    pub component: Id,
    pub component_pos_id: u32,
    pub cells: HashMap<Id, u32>,
    pub groups: HashMap<Id, u32>,
}

fn gen_component_info(
    ctx: &ir::Context,
    comp: &ir::Component,
    component_info: &mut HashMap<Id, ComponentPosIds>,
) -> CalyxResult<()> {
    // get pos for component
    let component_set_attr = comp
        .attributes
        .get_set(SetAttribute::Set(SetAttr::Pos))
        .unwrap();
    // FIXME: currently assuming that there is only one element in the set attr.
    let component_pos = component_set_attr.iter().next().unwrap();
    let mut component_pos_id = ComponentPosIds {
        component: comp.name,
        component_pos_id: *component_pos,
        cells: HashMap::new(),
        groups: HashMap::new(),
    };

    // get pos for groups
    for group in comp.groups.iter() {
        let group_ref = group.borrow();
        let group_set_attr = group_ref
            .attributes
            .get_set(SetAttribute::Set(SetAttr::Pos))
            .unwrap();
        let group_pos = group_set_attr.iter().next().unwrap();
        component_pos_id
            .groups
            .insert(group.borrow().name(), *group_pos);
    }

    // get pos for cell
    for cell in comp.cells.iter() {
        let cell_ref = cell.borrow();
        match cell_ref.attributes.get_set(SetAttribute::Set(SetAttr::Pos)) {
            None => {
                println!("Ignoring cell without pos: {}", cell_ref.name());
                continue;
            }
            Some(cell_set_attr) => {
                // FIXME: currently assuming that there is only one element in the set attr.
                let cell_pos = cell_set_attr.iter().next().unwrap();
                component_pos_id.cells.insert(cell_ref.name(), *cell_pos);
                if let ir::CellType::Component { name } = cell_ref.prototype {
                    let component = ctx
                        .components
                        .iter()
                        .find(|comp| comp.name == name)
                        .unwrap();
                    gen_component_info(ctx, component, component_info)?;
                }
            }
        }
    }
    component_info.insert(comp.name, component_pos_id);
    Ok(())
}

fn obtain_pos_info(
    name: &Id,
    pos_id: &u32,
    metadata_table: &MetadataTable,
) -> CalyxResult<PosInfo> {
    let SourceLocation { file, line } =
        metadata_table.lookup_position(PositionId::from(*pos_id));
    let filename = metadata_table
        .lookup_file_path(*file)
        .as_path()
        .to_str()
        .unwrap();
    Ok(PosInfo {
        name: *name,
        filename: filename.to_string(),
        linenum: line.as_usize(),
    })
}

fn resolve(
    ctx: &ir::Context,
    component_pos_ids: &HashMap<Id, ComponentPosIds>,
    component_info: &mut HashSet<ComponentInfo>,
) -> CalyxResult<()> {
    match &ctx.file_info_table {
        Some(metadata_table) => {
            for (curr_component, curr_component_pos_ids) in
                component_pos_ids.iter()
            {
                let SourceLocation { file, line } = metadata_table
                    .lookup_position(PositionId::from(
                        curr_component_pos_ids.component_pos_id,
                    ));
                let mut curr_component_info = ComponentInfo {
                    component: *curr_component,
                    filename: metadata_table
                        .lookup_file_path(*file)
                        .as_path()
                        .to_str()
                        .unwrap()
                        .to_string(),
                    linenum: line.as_usize(),
                    cells: Vec::new(),
                    groups: Vec::new(),
                };
                for (cell_name, cell_pos_id) in
                    curr_component_pos_ids.cells.iter()
                {
                    if let Ok(pos_info) =
                        obtain_pos_info(cell_name, cell_pos_id, metadata_table)
                    {
                        curr_component_info.cells.push(pos_info);
                    };
                }
                for (group_name, group_pos_id) in
                    curr_component_pos_ids.groups.iter()
                {
                    if let Ok(pos_info) = obtain_pos_info(
                        group_name,
                        group_pos_id,
                        metadata_table,
                    ) {
                        curr_component_info.groups.push(pos_info);
                    }
                }
                component_info.insert(curr_component_info);
            }
            Ok(())
        }
        None => panic!("No fileinfo table to read from!"),
    }
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

fn main() -> CalyxResult<()> {
    let p: Args = argh::from_env();

    let ws = frontend::Workspace::construct(&p.file_path, &p.lib_path)?;

    let ctx: ir::Context = ir::from_ast::ast_to_ir(ws)?;

    let main_comp = ctx.entrypoint();

    let mut component_pos_ids: HashMap<Id, ComponentPosIds> = HashMap::new();

    let mut component_info: HashSet<ComponentInfo> = HashSet::new();

    gen_component_info(&ctx, main_comp, &mut component_pos_ids)?;

    resolve(&ctx, &component_pos_ids, &mut component_info)?;

    write_json(component_info.clone(), p.output)?;

    Ok(())
}
