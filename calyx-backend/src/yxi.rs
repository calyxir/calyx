use crate::traits::Backend;
use calyx_ir as ir;
use calyx_ir::utils::GetMemInfo;
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
    width: u64,
    size: u64, //number of cells in memory
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

        let memory_names = ir::utils::external_memories_names(toplevel);
        let mem_infos = toplevel.get_mem_info();

        let memories: Vec<Memory> = memory_names
            .iter()
            .zip(mem_infos.iter())
            .map(|(memory_name, mem_info)| Memory {
                name: memory_name,
                width: mem_info.width,
                size: mem_info.size,
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

