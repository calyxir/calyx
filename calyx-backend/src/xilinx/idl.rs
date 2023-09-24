use crate::traits::Backend;
use calyx_ir as ir;
use calyx_utils::CalyxResult;
use serde::Serialize;
use serde_json::Result;
use super::toplevel::{get_mem_info, external_memories};
/// Backend that generates the Interface Design Language
/// that we need to create AXI wrappers on arbitrary programs
#[derive(Default)]
pub struct IdlBackend;

/// The root element of the `kernel.xml` file that describes an `.xo` package for the
/// Xilinx toolchain, as documented [in the Vitis user guide][ug].
///
/// [ug]: https://docs.xilinx.com/r/en-US/ug1393-vitis-application-acceleration/RTL-Kernel-XML-File
#[derive(Serialize)]
struct IDL<'a> {
    name: &'a str,
    memories : Memories<'a>,
}

#[derive(Serialize)]
struct Memory<'a> {
    name: &'a str,
    width: u64,
    size: u64, //size of width size memory, as defined in stdlib.
}

//XXX(nathanielnrn): Do we need this vs just a vec?
#[derive(Serialize)]
struct Memories<'a> {
    memories: Vec<Memory<'a>>,
}

impl<'a> From<Vec<Memory<'a>>> for Memories<'a> {
    fn from(memories: Vec<Memory<'a>>) -> Self {
        Memories {memories}
    }
}


///TOOD: OLD stuff, delete this

impl Backend for IdlBackend {
    fn name(&self) -> &'static str {
        "idl"
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

        let memory_names = external_memories(toplevel);
        let mem_infos  = get_mem_info(toplevel);
        
        let memories: Vec<Memory> = memory_names
        .iter()
        .zip(mem_infos.iter())
        .map(|(memory_name, mem_info)| {
            Memory{
                name: memory_name,
                width: mem_info.width,
                size: mem_info.size,
            }
        }).collect();



        write!(
            file.get_write(),
            quick_xml::se::to_string(&root).expect("XML Serialization failed")
        )?;

        Ok(())
    }
}
