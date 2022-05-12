#[macro_use]
extern crate lazy_static;

use argh::FromArgs;
use calyx::{errors::CalyxResult, frontend, ir};
use serde::ser::{Serialize, Serializer};
use serde_json::{json, Map, Value};
use std::borrow::Borrow;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

lazy_static! {
    static ref SIZEMAP: HashMap<&'static str, Vec<&'static str>> = {
        let mut m = HashMap::new();
        m.insert("std_mem_d1", vec!["SIZE"]);
        m.insert("std_mem_d2", vec!["D0_SIZE", "D1_SIZE"]);
        m.insert("std_mem_d3", vec!["D0_SIZE", "D1_SIZE", "D2_SIZE"]);
        m.insert(
            "std_mem_d4",
            vec!["D0_SIZE", "D1_SIZE", "D2_SIZE", "D3_SIZE"],
        );
        m
    };
}

struct CellData {
    name: String,
    width: u64,
    sizes: Vec<u64>,
}

enum SizeVec {
    Quad(Vec<Vec<Vec<Vec<u64>>>>),
    Triple(Vec<Vec<Vec<u64>>>),
    Double(Vec<Vec<u64>>),
    Single(Vec<u64>),
}

impl Serialize for SizeVec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &*self {
            SizeVec::Single(v) => {
                serializer.serialize_newtype_variant("SizeVec", 3, "data", &v)
            }
            SizeVec::Double(v) => {
                serializer.serialize_newtype_variant("SizeVec", 2, "data", &v)
            }
            SizeVec::Triple(v) => {
                serializer.serialize_newtype_variant("SizeVec", 1, "data", &v)
            }
            SizeVec::Quad(v) => {
                serializer.serialize_newtype_variant("SizeVec", 0, "data", &v)
            }
        }
    }
}

#[derive(FromArgs)]
/// Used to read a file
struct FilePath {
    /// file path to read data from
    #[argh(positional, from_str_fn(read_path))]
    file: Option<PathBuf>,
}

fn read_path(path: &str) -> Result<PathBuf, String> {
    Ok(Path::new(path).into())
}

fn main() -> CalyxResult<()> {
    let fp: FilePath = argh::from_env();

    let lib_path: &Path = &Path::new("../../../calyx");

    let ws = frontend::Workspace::construct(&fp.file, lib_path)?;
    let ctx: ir::Context = (ir::from_ast::ast_to_ir(ws))?;

    let comp = ctx
        .components
        .into_iter()
        .find(|comp| comp.name == ctx.entrypoint)
        .expect("No top-level component found.");

    let data_vec: Vec<CellData> = comp
        .cells
        .iter()
        .filter_map(|cell| get_data(cell))
        .collect();

    let mut map = Map::new();

    for CellData {
        name,
        width,
        mut sizes,
    } in data_vec
    {
        let json_comp: serde_json::Value = gen_comp(&mut sizes, width);
        map.insert(name, json_comp);
    }

    let json_map: Value = map.into();
    println!("{}", json_map.to_string());
    Ok(())
}

//generates a json value associated with sizes_vec and width
fn gen_comp(sizes_vec: &mut Vec<u64>, width: u64) -> serde_json::Value {
    let initial: SizeVec =
        SizeVec::Single(vec![0; sizes_vec.pop().unwrap().try_into().unwrap()]);
    sizes_vec.reverse();
    let data_vec: SizeVec =
        sizes_vec.iter().fold(initial, |acc, x| accumulate(*x, acc));
    let data = serde_json::to_value(data_vec).unwrap();
    json!({
        "data": data.get("data").unwrap(),
        "format": {
            "numeric_type": "bitnum",
            "is_signed": false,
            "width": width,
        }
    })
}

//creates a vector consisting of i v's.
fn add_dimension<T: std::clone::Clone>(i: u64, v: Vec<T>) -> Vec<Vec<T>> {
    vec![v; i.try_into().unwrap()]
}

//function to help build vectors of multiple dimensions using fold()
fn accumulate(i: u64, v: SizeVec) -> SizeVec {
    match v {
        SizeVec::Single(v) => SizeVec::Double(add_dimension(i, v)),
        SizeVec::Double(v) => SizeVec::Triple(add_dimension(i, v)),
        SizeVec::Triple(v) => SizeVec::Quad(add_dimension(i, v)),
        SizeVec::Quad(_) => panic!("trying to add dimension to 4d"),
    }
}

//Takes in cell, returns the reference to the cell
fn get_ref(cell: &Rc<RefCell<ir::Cell>>) -> Ref<ir::Cell> {
    let ref_cell: &RefCell<ir::Cell> = cell.borrow();
    let final_cell: Ref<ir::Cell> = ref_cell.borrow();
    final_cell
}

//Returns Some(CellData)) if cell is a std_mem cell, None otherwise
fn get_data(cell: &Rc<RefCell<ir::Cell>>) -> Option<CellData> {
    let final_cell = get_ref(cell);
    if !final_cell.attributes.has("external") {
        return None;
    }
    match &(*final_cell).prototype {
        ir::CellType::Primitive { ref name, .. } => SIZEMAP
            .get(&name.id.clone().as_str())
            .and_then(|sizes_vec| {
                Some(CellData {
                    name: final_cell.name().id.clone(),
                    width: final_cell.get_parameter("WIDTH").unwrap(),
                    sizes: sizes_vec
                        .iter()
                        .map(|size| final_cell.get_parameter(size).unwrap())
                        .collect(),
                })
            }),
        _ => None,
    }
}
