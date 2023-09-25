use crate::traits::Backend;
use calyx_ir as ir;
use calyx_utils::CalyxResult;
use serde::Serialize;
/// Backend that generates the Interface Design Language
/// that we need to create AXI wrappers on arbitrary programs
#[derive(Default)]
pub struct IdlBackend;

/// The root element of the `kernel.xml` file that describes an `.xo` package for the
/// Xilinx toolchain, as documented [in the Vitis user guide][ug].
///
/// [ug]: https://docs.xilinx.com/r/en-US/ug1393-vitis-application-acceleration/RTL-Kernel-XML-File
#[derive(Serialize)]
struct ProgramInterface<'a> {
    toplevel_name: &'a str,
    memories: Vec<Memory<'a>>,
}

#[derive(Serialize)]
struct Memory<'a> {
    name: &'a str,
    width: u64,
    size: u64, //size of width size memory, as defined in stdlib.
}

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
        let mem_infos = get_mem_info(toplevel);

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
            toplevel_name: toplevel.name.as_ref(),
            memories,
        };

        //TODO: we want to have na array of component names
        // check how to get them

        write!(
            file.get_write(),
            "{}",
            serde_json::to_string_pretty(&program_interface)
                .expect("IDL Serialization failed")
        )?;

        Ok(())
    }
}

/// Parameters for single dimensional memory
struct MemInfo {
    width: u64,
    size: u64,
}

// Gets all external memory cells in top level
fn external_memories_cells(comp: &ir::Component) -> Vec<ir::RRC<ir::Cell>> {
    comp.cells
        .iter()
        // find external memories
        .filter(|cell_ref| {
            let cell = cell_ref.borrow();
            cell.attributes.has(ir::BoolAttr::External)
        })
        .cloned()
        .collect()
}

fn get_mem_info(comp: &ir::Component) -> Vec<MemInfo> {
    external_memories_cells(comp)
        .iter()
        .map(|cr| {
            let cell = cr.borrow();
            let cell_size = match cell.prototype.get_name().unwrap().as_ref() {
                "std_mem_d1" | "seq_mem_d1" => {
                    cell.get_parameter("SIZE").unwrap()
                }
                "std_mem_d2" | "seq_mem_d2" => {
                    cell.get_parameter("D0_SIZE").unwrap()
                        * cell.get_parameter("D1_SIZE").unwrap()
                }
                "std_mem_d3" | "seq_mem_d3" => {
                    cell.get_parameter("D0_SIZE").unwrap()
                        * cell.get_parameter("D1_SIZE").unwrap()
                        * cell.get_parameter("D2_SIZE").unwrap()
                }

                "std_mem_d4" | "seq_mem_d4" => {
                    cell.get_parameter("D0_SIZE").unwrap()
                        * cell.get_parameter("D1_SIZE").unwrap()
                        * cell.get_parameter("D2_SIZE").unwrap()
                        * cell.get_parameter("D3_SIZE").unwrap()
                }
                _ => {
                    panic!("cell `{}' marked with `@external' but is not a memory primitive.", cell.name())
                }
            };

            MemInfo {
                width: cell.get_parameter("WIDTH").unwrap(),
                size: cell_size,
            }
        })
        .collect()
}

// Returns Vec<String> of memory names
fn external_memories(comp: &ir::Component) -> Vec<String> {
    external_memories_cells(comp)
        .iter()
        .map(|cell_ref| cell_ref.borrow().name().to_string())
        .collect()
}
