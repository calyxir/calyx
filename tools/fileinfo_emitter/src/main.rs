use argh::FromArgs;
use calyx_frontend::{
    self as frontend, SetAttr, SetAttribute, source_info::FileId,
    source_info::PositionId, source_info::SourceInfoTable,
    source_info::SourceLocation,
};
use calyx_ir::GetAttributes;
use calyx_ir::{self as ir, Id};
use calyx_utils::{CalyxResult, OutputFile};
use core::panic;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs::read_to_string;
use std::io;
use std::path::{Path, PathBuf};

// Emits a JSON mapping components, cells, and groups to their @pos filenames and line numbers.
// Used by the profiler to (1) map Calyx components/cells/groups to ADLs (currently only)

// NOTE: Current implementation is hacky because it uses the
//

#[derive(PartialEq, Eq, Hash, Clone, Serialize)]
enum Adl {
    Calyx,
    Py,
    Dahlia,
}

#[derive(FromArgs)]
/// Path for library and path for file to read from
struct Args {
    /// file path to read data from
    #[argh(positional, from_str_fn(read_path))]
    file_path: Option<PathBuf>,

    /// library path
    #[argh(option, short = 'l', default = "Path::new(\".\").into()")]
    pub lib_path: PathBuf,

    /// output file for Calyx control nodes
    #[argh(option, short = 'c', default = "OutputFile::Stdout")]
    pub control_output: OutputFile,

    /// output file for ADLs
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
struct AdlInfo {
    pub adl: Adl,
    pub components: Vec<ComponentInfo>,
}

#[derive(PartialEq, Eq, Hash, Clone, Serialize)]
struct ComponentInfo {
    // components may not have metadata attached.
    #[serde(serialize_with = "id_serialize_passthrough")]
    pub component: Id,
    pub filename: Option<String>,
    pub linenum: Option<usize>,
    // association name
    pub varname: Option<String>,
    pub cells: Vec<PosInfo>,
    pub groups: Vec<PosInfo>,
}

#[derive(PartialEq, Eq, Hash, Clone, Serialize)]
struct PosInfo {
    #[serde(serialize_with = "id_serialize_passthrough")]
    pub name: Id,
    pub filename: String,
    pub linenum: usize,
    pub varname: String,
}

// Obtaining the original line numbers of Calyx
#[derive(PartialEq, Eq, Hash, Clone, Serialize)]
struct ControlCalyxPosInfo {
    // TODO: Probably good to add filename as well in case the Calyx component
    // TODO: Also we need to make sure that we pull out the line number from Calyx (check if file is .futil)
    // pub filename: String,
    pub pos_num: u32,
    pub linenum: u32,
    pub ctrl_node: String,
}

struct ComponentPosIds {
    pub component_pos_id: Option<u32>,
    // cell name to positions
    pub cells: HashMap<Id, u32>,
    // group name to positions
    pub groups: HashMap<Id, u32>,
}

/// Collects @pos{} attributes for each component, group, and cell.
fn gen_component_info(
    ctx: &ir::Context,
    comp: &ir::Component,
    adl_posids: &HashSet<u32>,
    component_info: &mut HashMap<Id, ComponentPosIds>,
    adl: &Adl,
) -> CalyxResult<()> {
    // FIXME: currently assumes that there is a one-to-one mapping between groups and ADL posids

    // get pos for component
    let component_pos_first =
        comp.attributes.get_set(SetAttribute::Set(SetAttr::Pos));
    let component_pos = match component_pos_first {
        Some(pos_set) => {
            let a = pos_set.iter().find(|x| adl_posids.contains(x));
            if let Some(num_ref) = a {
                Some(*num_ref)
            } else {
                None
            }
        }
        None => None,
    };
    let mut component_pos_id = ComponentPosIds {
        component_pos_id: component_pos,
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
        let group_pos = group_set_attr
            .iter()
            .find(|x| adl_posids.contains(x))
            .unwrap();
        component_pos_id
            .groups
            .insert(group.borrow().name(), *group_pos);
    }

    // get pos for cell
    for cell in comp.cells.iter() {
        let cell_ref = cell.borrow();
        match cell_ref.attributes.get_set(SetAttribute::Set(SetAttr::Pos)) {
            None => {
                dbg!("Ignoring cell without pos: {}", cell_ref.name());
                continue;
            }
            Some(cell_set_attr) => {
                let cell_pos = cell_set_attr
                    .iter()
                    .find(|x| adl_posids.contains(x))
                    .unwrap();
                component_pos_id.cells.insert(cell_ref.name(), *cell_pos);
                if let ir::CellType::Component { name } = cell_ref.prototype {
                    let component = ctx
                        .components
                        .iter()
                        .find(|comp| comp.name == name)
                        .unwrap();
                    gen_component_info(
                        ctx,
                        component,
                        adl_posids,
                        component_info,
                        adl,
                    )?;
                }
            }
        }
    }

    component_info.insert(comp.name, component_pos_id);
    Ok(())
}

/// Uses a position Id and the source info table to obtain the filename and line number for the given Calyx construct.
fn obtain_pos_info(
    name: &Id,
    pos_id: &u32,
    source_info_table: &SourceInfoTable,
    file_lines_map: &HashMap<String, Vec<String>>,
    adl: &Adl,
) -> CalyxResult<PosInfo> {
    let SourceLocation { file, line } =
        source_info_table.lookup_position(PositionId::from(*pos_id));
    let filename = source_info_table
        .lookup_file_path(*file)
        .as_path()
        .to_str()
        .unwrap();
    Ok(PosInfo {
        name: *name,
        filename: filename.to_string(),
        linenum: line.as_usize(),
        varname: get_adl_var_name(
            &filename.to_string(),
            line.as_usize(),
            file_lines_map,
            name,
            adl,
        ),
    })
}

fn get_dahlia_var_name(
    filename: &String,
    linenum: usize,
    file_lines_map: &HashMap<String, Vec<String>>,
    calyx_var_name: &Id,
) -> String {
    if filename.ends_with("*.futil") {
        // If the original file is a Calyx file, the var name is the Calyx var name.
        return calyx_var_name.to_string();
    }
    // let unnamed = format!("'{calyx_var_name}'"); // fallback: Calyx-level construct variable name
    let file_lines: &Vec<String> = file_lines_map.get(filename).unwrap();
    let og_line_cloned = file_lines[linenum - 1].clone();
    let line = og_line_cloned.trim();
    return line.to_string();
}

/// Attempts to retrieve the Calyx-py ADL-level variable name of the component/group/cell by scanning the source line.
/// If an ADL-level variable name cannot be found, "'<calyx_var_name>'" will be returned as a substitute.
fn get_calyx_py_var_name(
    filename: &String,
    linenum: usize,
    file_lines_map: &HashMap<String, Vec<String>>,
    calyx_var_name: &Id,
) -> String {
    if filename.ends_with(".futil") {
        // If the original file is a Calyx file, the var name is the Calyx var name.
        return calyx_var_name.to_string();
    }
    // NOTE: This function currently only supports calyx-py eDSL.
    let unnamed = format!("'{calyx_var_name}'"); // fallback: Calyx-level construct variable name
    let file_lines: &Vec<String> = file_lines_map.get(filename).unwrap();
    let og_line_cloned = file_lines[linenum - 1].clone();
    let line = og_line_cloned.trim();
    if line.starts_with("with") && line.contains(".group(") {
        line.split(":")
            .next()
            .unwrap()
            .split(" ")
            .last()
            .unwrap()
            .to_string()
    } else if line.contains("=") {
        let before_equals = line.split("=").collect::<Vec<&str>>()[0]
            .split(":")
            .collect::<Vec<&str>>()[0]
            .trim()
            .to_string();
        let word_count = before_equals.chars().filter(|c| *c == ' ').count();
        if word_count == 0 {
            before_equals
        } else {
            unnamed
        }
    } else {
        unnamed
    }
}

fn get_adl_var_name(
    filename: &String,
    linenum: usize,
    file_lines_map: &HashMap<String, Vec<String>>,
    calyx_var_name: &Id,
    adl: &Adl,
) -> String {
    match adl {
        Adl::Py => get_calyx_py_var_name(
            filename,
            linenum,
            file_lines_map,
            calyx_var_name,
        ),
        Adl::Dahlia => get_dahlia_var_name(
            filename,
            linenum,
            file_lines_map,
            calyx_var_name,
        ),
        Adl::Calyx => panic!("Not supposed to be called on Calyx positions"),
    }
}

/// Resolves all position Ids with their corresponding file names, line numbers, and ADL-level variable names.
fn resolve(
    source_info_table: &SourceInfoTable,
    component_pos_ids: &HashMap<Id, ComponentPosIds>,
    component_info: &mut HashSet<ComponentInfo>,
    file_lines_map: &HashMap<String, Vec<String>>,
    adl: &Adl,
) -> CalyxResult<()> {
    for (curr_component, curr_component_pos_ids) in component_pos_ids.iter() {
        let mut curr_component_info =
            if let Some(pos_id) = curr_component_pos_ids.component_pos_id {
                let SourceLocation { file, line } =
                    source_info_table.lookup_position(PositionId::from(pos_id));
                let curr_component_filename = match adl {
                    Adl::Dahlia => {
                        file_lines_map.keys().last().unwrap().to_string()
                    }
                    Adl::Py => source_info_table
                        .lookup_file_path(*file)
                        .as_path()
                        .to_str()
                        .unwrap()
                        .to_string(),
                    Adl::Calyx => {
                        panic!("resolve() should only be called on ADLs!")
                    }
                };
                let varname = get_adl_var_name(
                    &curr_component_filename,
                    line.as_usize(),
                    file_lines_map,
                    curr_component,
                    &adl,
                );
                ComponentInfo {
                    component: *curr_component,
                    filename: Some(curr_component_filename.clone()),
                    linenum: Some(line.as_usize()),
                    varname: Some(varname),
                    cells: Vec::new(),
                    groups: Vec::new(),
                }
            } else {
                ComponentInfo {
                    component: *curr_component,
                    filename: None,
                    linenum: None,
                    varname: None,
                    cells: Vec::new(),
                    groups: Vec::new(),
                }
            };
        for (cell_name, cell_pos_id) in curr_component_pos_ids.cells.iter() {
            if let Ok(pos_info) = obtain_pos_info(
                cell_name,
                cell_pos_id,
                source_info_table,
                file_lines_map,
                &adl,
            ) {
                curr_component_info.cells.push(pos_info);
            };
        }
        for (group_name, group_pos_id) in curr_component_pos_ids.groups.iter() {
            if let Ok(pos_info) = obtain_pos_info(
                group_name,
                group_pos_id,
                source_info_table,
                file_lines_map,
                &adl,
            ) {
                curr_component_info.groups.push(pos_info);
            }
        }

        component_info.insert(curr_component_info);
    }
    Ok(())
}

/// Write the collected set of component information to a JSON file.
fn write_adl_json(
    component_info: HashSet<ComponentInfo>,
    mut file: OutputFile,
    adl: Adl,
) -> Result<(), io::Error> {
    let created_vec: Vec<ComponentInfo> = component_info.into_iter().collect();
    let adl_info: AdlInfo = AdlInfo {
        adl: adl,
        components: created_vec,
    };
    serde_json::to_writer_pretty(file.get_write(), &adl_info)?;
    Ok(())
}

/// Read all lines from all files for lookup in resolve()
fn create_file_map(
    source_info_table: &SourceInfoTable,
) -> HashMap<String, Vec<String>> {
    let mut toplevel_file_map: HashMap<String, Vec<String>> = HashMap::new();
    for (_, path) in source_info_table.iter_file_map() {
        let file_lines: Vec<String> = read_to_string(path)
            .unwrap()
            .lines()
            .map(String::from)
            .collect();
        let filename = path.as_path().to_str().unwrap().to_string();
        toplevel_file_map.insert(filename, file_lines);
    }
    toplevel_file_map
}

/// Categorizes position ids and file ids based on the ADL signified by the
/// file id's extension.
/// FIXME: In the future we maybe have two versions of the tool and inputs,
/// one that deals with ADL positions and another to deal with Calyx positions?
fn create_lang_to_posid_map(
    source_info_table: &SourceInfoTable,
) -> CalyxResult<HashMap<Adl, HashMap<u32, u32>>> {
    let mut fileid_to_lang: HashMap<FileId, Adl> = HashMap::new();
    let mut lang_to_posids_to_fileids: HashMap<Adl, HashMap<u32, u32>> =
        HashMap::new();

    // iterate through fileids to find the corresponding Adl
    for (fileid, path) in source_info_table.iter_file_map() {
        if let Some(path_ext) = path.extension() {
            if let Some(path_ext_str) = path_ext.to_str() {
                match path_ext_str {
                    "futil" => {
                        fileid_to_lang.insert(*fileid, Adl::Calyx);
                    }
                    "py" => {
                        fileid_to_lang.insert(*fileid, Adl::Py);
                    }
                    "fuse" => {
                        fileid_to_lang.insert(*fileid, Adl::Dahlia);
                    }
                    _ => println!("Unsupported file extension: {path_ext_str}"),
                }
            }
        }
    }

    for (posid, source_loc) in source_info_table.iter_position_map() {
        let fileid = source_loc.file;
        let linenum = source_loc.line.as_usize() as u32;
        if let Some(adl) = fileid_to_lang.get(&fileid) {
            match lang_to_posids_to_fileids.get_mut(adl) {
                Some(set_ref) => {
                    set_ref.insert(posid.value(), linenum);
                }
                None => {
                    let mut item_set = HashMap::new();
                    item_set.insert(posid.value(), linenum);
                    lang_to_posids_to_fileids.insert(adl.clone(), item_set);
                }
            }
        }
    }

    Ok(lang_to_posids_to_fileids)
}

fn get_control_name(control: &ir::Control) -> CalyxResult<Option<&str>> {
    let out_str = match control {
        ir::Control::Seq(_) => Some("seq"),
        ir::Control::Par(_) => Some("par"),
        ir::Control::If(_) => Some("if"),
        ir::Control::While(_) => Some("while"),
        ir::Control::Repeat(_) => Some("repeat"),
        _ => None,
    };
    Ok(out_str)
}

fn gen_control_info_helper(
    control: &ir::Control,
    calyx_posids_to_linenums: &HashMap<u32, u32>,
    control_pos_infos: &mut Vec<ControlCalyxPosInfo>,
) -> CalyxResult<()> {
    // add information from this particular control node.

    let control_name = get_control_name(control)?;
    if let Some(control_id) = control_name {
        if let Some(pos_set) = control
            .get_attributes()
            .get_set(SetAttribute::Set(SetAttr::Pos))
        {
            for calyx_pos in pos_set
                .iter()
                .filter(|pos| calyx_posids_to_linenums.contains_key(pos))
            {
                // theoretically there should only be one, but let's consider all positions
                let calyx_linenum =
                    calyx_posids_to_linenums.get(calyx_pos).unwrap();
                control_pos_infos.push(ControlCalyxPosInfo {
                    pos_num: *calyx_pos,
                    linenum: *calyx_linenum,
                    ctrl_node: String::from(control_id),
                })
            }
        }
    }

    // recurse into child control nodes.
    match control {
        ir::Control::Seq(ir::Seq { stmts, .. })
        | ir::Control::Par(ir::Par { stmts, .. }) => {
            for stmt in stmts {
                gen_control_info_helper(
                    stmt,
                    calyx_posids_to_linenums,
                    control_pos_infos,
                )?;
            }
        }
        ir::Control::If(ir::If {
            tbranch, fbranch, ..
        }) => {
            gen_control_info_helper(
                tbranch,
                calyx_posids_to_linenums,
                control_pos_infos,
            )?;
            gen_control_info_helper(
                fbranch,
                calyx_posids_to_linenums,
                control_pos_infos,
            )?;
        }
        ir::Control::While(ir::While { body, .. })
        | ir::Control::Repeat(ir::Repeat { body, .. }) => {
            gen_control_info_helper(
                body,
                calyx_posids_to_linenums,
                control_pos_infos,
            )?;
        }
        ir::Control::Static(_) => todo!(),
        _ => (),
    }

    Ok(())
}

/// Recursively populates component_info with ControlCalyxPosInfos for each component.
/// For each control node in the component, generate information about the control node's identifier,
/// the Calyx-file position Id, and the Calyx-file line number.
fn gen_control_info(
    ctx: &ir::Context,
    adl_to_posids_to_linenums: &HashMap<Adl, HashMap<u32, u32>>,
    component_info: &mut HashMap<Id, Vec<ControlCalyxPosInfo>>,
) -> CalyxResult<()> {
    match adl_to_posids_to_linenums.get(&Adl::Calyx) {
        Some(calyx_posid_map) => {
            for comp in ctx.components.iter() {
                let mut component_control_info: Vec<ControlCalyxPosInfo> =
                    Vec::new();

                // gather information about this component.
                let control = comp.control.borrow();

                gen_control_info_helper(
                    &control,
                    calyx_posid_map,
                    &mut component_control_info,
                )?;
                component_info.insert(comp.name, component_control_info);
            }
        }

        None => {
            println!(
                "Calyx-level metadata not given! Run `-p metadata-table-generation`"
            );
        }
    }

    Ok(())
}

fn calyx_ctrl_wrapper(
    ctx: &ir::Context,
    adl_to_posids_to_linenums: &HashMap<Adl, HashMap<u32, u32>>,
    mut file: OutputFile,
) -> CalyxResult<()> {
    let mut component_to_calyx_control_info = HashMap::new();
    gen_control_info(
        ctx,
        adl_to_posids_to_linenums,
        &mut component_to_calyx_control_info,
    )?;

    serde_json::to_writer_pretty(
        file.get_write(),
        &component_to_calyx_control_info,
    )?;

    Ok(())
}

fn adl_wrapper(
    ctx: &ir::Context,
    source_info_table: &SourceInfoTable,
    adl_to_posids_to_linenums: &HashMap<Adl, HashMap<u32, u32>>,
    file_lines_map: &HashMap<String, Vec<String>>,
    file: OutputFile,
    adl: Adl,
) -> CalyxResult<()> {
    let main_comp = ctx.entrypoint();

    let mut component_pos_ids: HashMap<Id, ComponentPosIds> = HashMap::new();
    let mut component_info: HashSet<ComponentInfo> = HashSet::new();

    match adl_to_posids_to_linenums.get(&adl) {
        Some(py_posid_map) => {
            let py_posids: HashSet<u32> =
                py_posid_map.keys().cloned().collect();
            gen_component_info(
                ctx,
                main_comp,
                &py_posids,
                &mut component_pos_ids,
                &adl,
            )?;

            resolve(
                source_info_table,
                &component_pos_ids,
                &mut component_info,
                file_lines_map,
                &adl,
            )?;
            write_adl_json(component_info.clone(), file, adl)?;
        }
        None => {
            println!("Python-level metadata not given!");
        }
    }

    Ok(())
}

fn main() -> CalyxResult<()> {
    let p: Args = argh::from_env();

    let ws = frontend::Workspace::construct(&p.file_path, &[p.lib_path])?;

    let ctx: ir::Context = ir::from_ast::ast_to_ir(
        ws,
        ir::from_ast::AstConversionConfig::default(),
    )?;

    // FIXME: should provide argument(s) about what ADLs to look out for, if any

    match &ctx.source_info_table {
        Some(source_info_table) => {
            let adl_to_posids_to_linenums =
                create_lang_to_posid_map(source_info_table)?;

            let file_lines_map = create_file_map(source_info_table);

            calyx_ctrl_wrapper(
                &ctx,
                &adl_to_posids_to_linenums,
                p.control_output,
            )?;

            if let Some(_) = adl_to_posids_to_linenums.get(&Adl::Py) {
                adl_wrapper(
                    &ctx,
                    source_info_table,
                    &adl_to_posids_to_linenums,
                    &file_lines_map,
                    p.output,
                    Adl::Py,
                )?;
            } else if let Some(_) = adl_to_posids_to_linenums.get(&Adl::Dahlia)
            {
                adl_wrapper(
                    &ctx,
                    source_info_table,
                    &adl_to_posids_to_linenums,
                    &file_lines_map,
                    p.output,
                    Adl::Dahlia,
                )?;
            }

            Ok(())
        }
        None => panic!("No fileinfo table to read from!"),
    }
}
