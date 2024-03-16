use crate::traits::Backend;
use calyx_ir as ir;
use calyx_ir::utils::{GetMemInfo, MemoryType};
use calyx_utils::CalyxResult;
use serde::Serialize;
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

impl Backend for YxiBackend {
    fn name(&self) -> &'static str {
        "yxi"
    }

    fn validate(_ctx: &ir::Context) -> CalyxResult<()> {
        Ok(())
    }

    fn link_externs(
        _prog: &ir::Context,
        _write: &mut calyx_utils::OutputFile,
    ) -> CalyxResult<()> {
        Ok(())
    }

    fn emit(
        prog: &ir::Context,
        file: &mut calyx_utils::OutputFile,
    ) -> CalyxResult<()> {
        let toplevel = prog
            .components
            .iter()
            .find(|comp| comp.name == prog.entrypoint)
            .unwrap();

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

        serde_json::to_writer_pretty(file.get_write(), &program_interface)?;

        Ok(())
    }
}
