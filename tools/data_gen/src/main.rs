use argh::FromArgs;
use calyx_frontend as frontend;
use calyx_ir as ir;
use calyx_ir::utils::GetMemInfo;
use calyx_utils::CalyxResult;
use rand::Rng;
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// *How to use*
// run: cargo run -p data_gen -- <calyx file> to generate data w/ all 0s and
// type int
// add -f true if you want random values of type fix<32,16>
// add -r true if you want randomized values

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
            ("comb_mem_d1", vec!["SIZE"]),
            ("comb_mem_d2", vec!["D0_SIZE", "D1_SIZE"]),
            ("comb_mem_d3", vec!["D0_SIZE", "D1_SIZE", "D2_SIZE"]),
            (
                "comb_mem_d4",
                vec!["D0_SIZE", "D1_SIZE", "D2_SIZE", "D3_SIZE"],
            ),
        ]
        .into_iter()
        .collect::<HashMap<&'static str, Vec<&'static str>>>()
    };
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

    /// whether data is randomized or 0
    #[argh(option, short = 'r', default = "false")]
    pub random_data: bool,
}

fn read_path(path: &str) -> Result<PathBuf, String> {
    Ok(Path::new(path).into())
}

fn main() -> CalyxResult<()> {
    let p: FilePaths = argh::from_env();
    let fp_data = p.fp_data;
    let rand_data = p.random_data;

    let ws = frontend::Workspace::construct(&p.file_path, &[p.lib_path])?;
    let ctx: ir::Context = ir::from_ast::ast_to_ir(
        ws,
        ir::from_ast::AstConversionConfig::default(),
    )?;
    let top = ctx.entrypoint();
    let data_vec = top.get_mem_info();

    let mut map = Map::new();
    for ir::utils::MemInfo {
        name,
        data_width: width,
        dimension_sizes,
        ..
    } in data_vec
    {
        let sizes: Vec<usize> =
            dimension_sizes.into_iter().map(|x| x as usize).collect();
        let json_comp = if !(fp_data) {
            gen_comp(&sizes[..], width, rand_data)
        } else {
            gen_comp_float(&sizes[..], width, rand_data)
        };
        map.insert(name, json_comp);
    }

    let json_map: Value = map.into();
    println!("{json_map}");
    Ok(())
}

// generates random of size usize
fn gen_random_int_vec(d0: usize) -> Vec<u64> {
    let mut rng = rand::rng();
    (0..d0).map(|_| rng.random_range(0..100)).collect()
}

fn gen_random_2d_int(d0: usize, d1: usize) -> Vec<Vec<u64>> {
    (0..d0).map(|_| gen_random_int_vec(d1)).collect()
}

// generates random 3d vec of size usize
fn gen_random_3d_int(d0: usize, d1: usize, d2: usize) -> Vec<Vec<Vec<u64>>> {
    (0..d0)
        .map(|_| (0..d1).map(|_| gen_random_int_vec(d2)).collect())
        .collect()
}

fn gen_random_4d_int(
    d0: usize,
    d1: usize,
    d2: usize,
    d3: usize,
) -> Vec<Vec<Vec<Vec<u64>>>> {
    (0..d0)
        .map(|_| {
            (0..d1)
                .map(|_| (0..d2).map(|_| gen_random_int_vec(d3)).collect())
                .collect()
        })
        .collect()
}

// generates random of size usize
fn gen_random_vec(d0: usize) -> Vec<f32> {
    let mut rng = rand::rng();
    (0..d0).map(|_| rng.random_range(-5.0..5.0)).collect()
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

// generates random 4d vec of size usize
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
fn gen_comp(sizes_vec: &[usize], width: u64, rand: bool) -> serde_json::Value {
    let data = match *sizes_vec {
        [d0] => serde_json::to_value(if rand {
            gen_random_int_vec(d0)
        } else {
            vec![0_u64; d0]
        }),
        [d0, d1] => serde_json::to_value(if rand {
            gen_random_2d_int(d0, d1)
        } else {
            vec![vec![0_u64; d1]; d0]
        }),
        [d0, d1, d2] => serde_json::to_value(if rand {
            gen_random_3d_int(d0, d1, d2)
        } else {
            vec![vec![vec![0_u64; d2]; d1]; d0]
        }),
        [d0, d1, d2, d3] => serde_json::to_value(if rand {
            gen_random_4d_int(d0, d1, d2, d3)
        } else {
            vec![vec![vec![vec![0_u64; d3]; d2]; d1]; d0]
        }),
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
fn gen_comp_float(
    sizes_vec: &[usize],
    width: u64,
    rand_data: bool,
) -> serde_json::Value {
    let data = match *sizes_vec {
        [d0] => serde_json::to_value(if rand_data {
            gen_random_vec(d0)
        } else {
            vec![0_f32; d0]
        }),
        [d0, d1] => serde_json::to_value(if rand_data {
            gen_random_2d(d0, d1)
        } else {
            vec![vec![0_f32; d1]; d0]
        }),
        [d0, d1, d2] => serde_json::to_value(if rand_data {
            gen_random_3d(d0, d1, d2)
        } else {
            vec![vec![vec![0_f32; d2]; d1]; d0]
        }),
        [d0, d1, d2, d3] => serde_json::to_value(if rand_data {
            gen_random_4d(d0, d1, d2, d3)
        } else {
            vec![vec![vec![vec![0_f32; d3]; d2]; d1]; d0]
        }),
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
