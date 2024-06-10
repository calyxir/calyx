use argh::FromArgs;
use calyx_frontend as frontend;
use calyx_ir as ir;
use calyx_ir::utils::{GetMemInfo, MemoryType};
use calyx_utils::CalyxResult;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(FromArgs)]
/// Path for library and path for file to read from
struct Args {
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

/// Backend that generates the YXI Interface Definition Language.
/// YXI aims to be a description of toplevel hardware modules that we can then consume
/// to create things like AXI wrappers on arbitrary programs
#[derive(Default)]
pub struct YxiBackend;

#[derive(Serialize)]
struct ProgramInterface<'a> {
    toplevel: &'a str,
    memories: Vec<Memory<'a>>,
}

#[derive(Serialize)]
struct Memory<'a> {
    name: &'a str,
    memory_type: MemoryType,
    data_width: u64,
    dimensions: u64,
    dimension_sizes: Vec<u64>,
    total_size: u64, //number of cells in memory
    idx_sizes: Vec<u64>,
}

fn main() -> CalyxResult<()> {
    let p: Args = argh::from_env();

    let ws = frontend::Workspace::construct(&p.file_path, &p.lib_path)?;
    let ctx: ir::Context = ir::from_ast::ast_to_ir(ws)?;

    let toplevel = ctx.entrypoint();

    let memory_names = ir::utils::external_and_ref_memories_names(toplevel);
    let mem_infos = toplevel.get_mem_info();

    let memories: Vec<Memory> = memory_names
        .iter()
        .zip(mem_infos.iter())
        .map(|(memory_name, mem_info)| Memory {
            name: memory_name,
            memory_type: mem_info.memory_type,
            data_width: mem_info.data_width,
            dimensions: mem_info.dimensions,
            dimension_sizes: mem_info.dimension_sizes.clone(),
            total_size: mem_info.total_size,
            idx_sizes: mem_info.idx_sizes.clone(),
        })
        .collect();

    let program_interface = ProgramInterface {
        toplevel: toplevel.name.as_ref(),
        memories,
    };

    serde_json::to_writer_pretty(std::io::stdout(), &program_interface)?;

    Ok(())
}
