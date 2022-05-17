use argh::FromArgs;
use calyx::{errors::CalyxResult, frontend, ir};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

lazy_static::lazy_static! {
    static ref SIZEMAP: HashMap<&'static str, Vec<&'static str>> = {
        vec![
            ("std_mem_d1", vec!["SIZE"]),
            ("std_mem_d2", vec!["D0_SIZE", "D1_SIZE"]),
            ("std_mem_d3", vec!["D0_SIZE", "D1_SIZE", "D2_SIZE"]),
            (
                "std_mem_d4",
                vec!["D0_SIZE", "D1_SIZE", "D2_SIZE", "D3_SIZE"],
            ),
        ]
        .into_iter()
        .collect::<HashMap<&'static str, Vec<&'static str>>>()
    };
}

/// Holds data for std_mem cells, including name of cell, width, and sizes
/// Name is the name of cell itself, not its type. Sizes is a vector
/// that holds the dimensions of the cell (ex: for a 2 x 3 std_mem_d2 cell it would be [2,3])
struct CellData {
    name: String,
    width: u64,
    sizes: Vec<usize>,
}

#[derive(FromArgs)]
/// Path for library and path for file to read from
struct FilePaths {
    /// file path to read data from
    #[argh(positional, from_str_fn(read_path))]
    file_path: Option<PathBuf>,

    /// library path
    #[argh(option, short = 'l', default = "Path::new(\".\").into()")]
    pub lib_path: PathBuf,
}

fn read_path(path: &str) -> Result<PathBuf, String> {
    Ok(Path::new(path).into())
}

fn main() -> CalyxResult<()> {
    let p: FilePaths = argh::from_env();

    let ws = frontend::Workspace::construct(&p.file_path, &p.lib_path)?;
    let ctx: ir::Context = ir::from_ast::ast_to_ir(ws)?;

    let comp = ctx
        .components
        .into_iter()
        .find(|comp| comp.name == ctx.entrypoint)
        .expect("No top-level component found.");

    let data_vec: Vec<CellData> =
        comp.cells.iter().filter_map(get_data).collect();

    let mut map = Map::new();

    for CellData { name, width, sizes } in data_vec {
        let json_comp = gen_comp(&sizes[..], width);
        map.insert(name, json_comp);
    }

    let json_map: Value = map.into();
    println!("{}", json_map);
    Ok(())
}

//generates a json value associated with sizes_vec and width
fn gen_comp(sizes_vec: &[usize], width: u64) -> serde_json::Value {
    let data = match *sizes_vec {
        [d0] => serde_json::to_value(vec![0_u64; d0]),
        [d0, d1] => serde_json::to_value(vec![vec![0_u64; d1]; d0]),
        [d0, d1, d2] => {
            serde_json::to_value(vec![vec![vec![0_u64; d2]; d1]; d0])
        }
        [d0, d1, d2, d3] => {
            serde_json::to_value(vec![vec![vec![vec![0_u64; d3]; d2]; d1]; d0])
        }
        _ => panic!("Sizes Vec is not 1-4 dimensional"),
    }
    .unwrap_or_else(|_| panic!("could not unwrap data to put into json"));
    json!({
        "data": data,
        "format": {
            "numeric_type": "bitnum",
            "is_signed": false,
            "width": width,
        }
    })
}

//Returns Some(CellData)) if cell is a std_mem cell, None otherwise
fn get_data(cell: &ir::RRC<ir::Cell>) -> Option<CellData> {
    let final_cell = cell.borrow();
    if !final_cell.attributes.has("external") {
        return None;
    }
    match final_cell.prototype {
        ir::CellType::Primitive { ref name, .. } => {
            SIZEMAP.get(&name.id.as_str()).map(|sizes_vec| CellData {
                name: final_cell.name().id.clone(),
                width: final_cell
                    .get_parameter("WIDTH")
                    .unwrap_or_else(|| panic!("unable to get width of cell")),
                sizes: sizes_vec
                    .iter()
                    .map(|size| {
                        final_cell.get_parameter(size).unwrap_or_else(|| {
                            panic!("unable to get sizes of cell")
                        }) as usize
                    })
                    .collect(),
            })
        }
        _ => None,
    }
}
