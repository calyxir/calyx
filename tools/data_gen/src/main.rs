use argh::FromArgs;
use calyx::{errors::CalyxResult, frontend, ir};
use rand::Rng;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// *How to use*
// run: cargo run -p data_gen -- <calyx file> to generate data w/ all 0s and
// type int
// add -f true if you want random values of type fix<32,16>

lazy_static::lazy_static! {
    static ref SIZEMAP: HashMap<&'static str, Vec<&'static str>> = {
        vec![
            ("seq_mem_d1", vec!["SIZE"]),
            ("seq_mem_d2", vec!["D0_SIZE", "D1_SIZE"]),
            ("seq_mem_d3", vec!["D0_SIZE", "D1_SIZE", "D2_SIZE"]),
            (
                "seq_mem_d4",
                vec!["D0_SIZE", "D1_SIZE", "D2_SIZE", "D3_SIZE"],
            ),
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

#[derive(Debug, FromArgs)]
/// Path for library and path for file to read from
struct FilePaths {
    /// file path to read data from
    #[argh(positional, from_str_fn(read_path))]
    file_path: Option<PathBuf>,

    /// library path
    #[argh(option, short = 'l', default = "Path::new(\".\").into()")]
    pub lib_path: PathBuf,

    /// whether data is fixpoint or int
    #[argh(option, short = 'f', default = "false")]
    pub fp_data: bool,
}

fn read_path(path: &str) -> Result<PathBuf, String> {
    Ok(Path::new(path).into())
}

fn main() -> CalyxResult<()> {
    let p: FilePaths = argh::from_env();
    let fp_data = p.fp_data;

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
        let json_comp = if !(fp_data) {
            gen_comp(&sizes[..], width)
        } else {
            gen_comp_float(&sizes[..], width)
        };
        map.insert(name, json_comp);
    }

    let json_map: Value = map.into();
    println!("{}", json_map);
    Ok(())
}

// generates random of size usize
fn gen_random_vec(d0: usize) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    (0..d0).map(|_| rng.gen_range(0.0..1.0)).collect()
}

// generates random 2d vec of size usize
fn gen_random_2d(d0: usize, d1: usize) -> Vec<Vec<f32>> {
    (0..d0).map(|_| gen_random_vec(d1)).collect()
}

// generates random 3d vec of size usize
fn gen_random_3d(d0: usize, d1: usize, d2: usize) -> Vec<Vec<Vec<f32>>> {
    (0..d0)
        .map(|_| (0..d1).map(|_| gen_random_vec(d2)).collect())
        .collect()
}

// generates random 3d vec of size usize
fn gen_random_4d(
    d0: usize,
    d1: usize,
    d2: usize,
    d3: usize,
) -> Vec<Vec<Vec<Vec<f32>>>> {
    (0..d0)
        .map(|_| {
            (0..d1)
                .map(|_| (0..d2).map(|_| gen_random_vec(d3)).collect())
                .collect()
        })
        .collect()
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

// generates a fix<32,16> json value associated with sizes_vec and width
fn gen_comp_float(sizes_vec: &[usize], width: u64) -> serde_json::Value {
    let data = match *sizes_vec {
        [d0] => serde_json::to_value(gen_random_vec(d0)),
        [d0, d1] => serde_json::to_value(gen_random_2d(d0, d1)),
        [d0, d1, d2] => serde_json::to_value(gen_random_3d(d0, d1, d2)),
        [d0, d1, d2, d3] => serde_json::to_value(gen_random_4d(d0, d1, d2, d3)),
        _ => panic!("Sizes Vec is not 1-4 dimensional"),
    }
    .unwrap_or_else(|_| panic!("could not unwrap data to put into json"));
    json!({
        "data": data,
        "format": {
            "frac_width": 16,
            "is_signed": true,
            "numeric_type": "fixed_point",
            "width": width
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
