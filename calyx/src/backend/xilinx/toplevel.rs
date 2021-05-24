use super::{
    axi, control_axi::ControlInterface, fsm, memory_axi::bram,
    memory_axi::MemoryInterface, utils,
};
use crate::{
    backend::traits::Backend,
    errors::{Error, FutilResult},
    ir,
};
use vast::v05::ast as v;

/// A backend that generates the Xilinx interfacing for a Calyx program.
#[derive(Default)]
pub struct XilinxInterfaceBackend;

impl Backend for XilinxInterfaceBackend {
    fn name(&self) -> &'static str {
        "xilinx-axi"
    }

    fn validate(_ctx: &ir::Context) -> FutilResult<()> {
        Ok(())
    }

    fn link_externs(
        _prog: &ir::Context,
        _write: &mut crate::utils::OutputFile,
    ) -> FutilResult<()> {
        Ok(())
    }

    fn emit(
        prog: &ir::Context,
        file: &mut crate::utils::OutputFile,
    ) -> FutilResult<()> {
        let toplevel = prog
            .components
            .iter()
            .find(|comp| comp.attributes.has("toplevel") || comp.name == "main")
            .ok_or_else(|| Error::Misc("no toplevel".to_string()))?;
        let memories = external_memories(toplevel);

        let mut modules = vec![
            top_level(12, 32, &memories),
            bram(32, 32, 5),
            axi::AxiInterface::control_module("Control_axi", 12, 32, &memories),
        ];

        for (i, _mem) in memories.iter().enumerate() {
            modules.push(axi::AxiInterface::memory_module(
                &format!("Memory_controller_axi_{}", i),
                512,
                64,
                32,
            ))
        }

        let module_string = modules
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
            .join("\n");

        write!(
            file.get_write(),
            r#"`default_nettype none
/* verilator lint_off DECLFILENAME */
{}`default_nettype wire"#,
            module_string,
        )?;

        Ok(())
    }
}

fn external_memories(comp: &ir::Component) -> Vec<String> {
    // find external memories
    comp.cells
        .iter()
        .filter(|cell_ref| {
            matches!(cell_ref.borrow().get_attribute("external"), Some(&1))
        })
        .map(|cell_ref| cell_ref.borrow().name.to_string())
        .collect()
}

fn top_level(
    address_width: u64,
    data_width: u64,
    memories: &[String],
) -> v::Module {
    let mut module = v::Module::new("Toplevel");

    // add system signals
    module.add_input("ap_clk", 1);
    module.add_input("ap_rst_n", 1);
    // module.add_output("ap_interrupt", 1);

    // axi control signals
    let axi4 = axi::AxiInterface::control_channels(
        address_width,
        data_width,
        "s_axi_control_",
    );
    axi4.add_ports_to(&mut module);

    // add an axi interface for each external memory
    for (idx, _mem) in memories.iter().enumerate() {
        axi::AxiInterface::memory_channels(64, 512, &format!("m{}_axi_", idx))
            .add_ports_to(&mut module);
    }

    // wires
    module.add_stmt(v::Decl::new_wire("ap_start", 1));
    module.add_stmt(v::Decl::new_wire("ap_done", 1));
    module.add_stmt(v::Decl::new_wire("timeout", 32));
    for mem in memories {
        module.add_stmt(v::Decl::new_wire(mem, 64));
    }

    // reset
    module.add_stmt(v::Decl::new_wire("reset", 1));
    module.add_stmt(v::Parallel::Assign(
        "reset".into(),
        v::Expr::new_not("ap_rst_n"),
    ));

    // instantiate control interface
    let base_control_axi_interface =
        axi::AxiInterface::control_channels(address_width, data_width, "");
    let mut control_instance =
        v::Instance::new("inst_control_axi", "Control_axi");
    control_instance.connect("ACLK", "ap_clk");
    control_instance.connect("ARESET", "reset");
    for mem in memories {
        control_instance.connect_ref(mem, mem);
    }
    control_instance.connect("ap_start", "ap_start");
    control_instance.connect("ap_done", "ap_done");
    control_instance.connect("timeout", "timeout");

    for port in base_control_axi_interface.ports() {
        control_instance.connect_ref(&port, &format!("s_axi_control_{}", port));
    }
    module.add_instance(control_instance);

    // and some wires for each memory
    for mem in memories {
        module.add_decl(v::Decl::new_wire(&format!("{}_copy", mem), 1));
        module.add_decl(v::Decl::new_wire(&format!("{}_copy_done", mem), 1));
        module.add_decl(v::Decl::new_wire(&format!("{}_send", mem), 1));
        module.add_decl(v::Decl::new_wire(&format!("{}_send_done", mem), 1));
    }
    host_transfer_fsm(&mut module, memories);

    // instantiate memory controllers
    let base_master_axi_interface =
        axi::AxiInterface::memory_channels(64, 32, "");
    for (idx, mem) in memories.iter().enumerate() {
        let write_data = format!("{}_write_data", mem);
        let read_data = format!("{}_read_data", mem);
        let addr0 = format!("{}_addr0", mem);
        let write_en = format!("{}_write_en", mem);
        let done = format!("{}_done", mem);
        module.add_decl(v::Decl::new_wire(&write_data, data_width));
        module.add_decl(v::Decl::new_wire(&read_data, data_width));
        module.add_decl(v::Decl::new_wire(&addr0, 5));
        module.add_decl(v::Decl::new_wire(&write_en, 1));
        module.add_decl(v::Decl::new_wire(&done, 1));

        let mut memory_instance = v::Instance::new(
            &format!("inst_mem_controller_axi_{}", idx),
            &format!("Memory_controller_axi_{}", idx),
        );
        memory_instance.connect("ACLK", "ap_clk");
        memory_instance.connect(
            "ARESET",
            v::Expr::new_logical_or("reset", "memories_sent"),
        );
        for port in base_master_axi_interface.ports() {
            memory_instance
                .connect_ref(&port, &format!("m{}_axi_{}", idx, port));
        }
        memory_instance.connect_ref("BASE_ADDRESS", mem);
        memory_instance.connect_ref("COPY_FROM_HOST", &format!("{}_copy", mem));
        memory_instance
            .connect_ref("COPY_FROM_HOST_DONE", &format!("{}_copy_done", mem));
        memory_instance.connect_ref("SEND_TO_HOST", &format!("{}_send", mem));
        memory_instance
            .connect_ref("SEND_TO_HOST_DONE", &format!("{}_send_done", mem));

        memory_instance.connect_ref("WRITE_DATA", &write_data);
        memory_instance.connect_ref("READ_DATA", &read_data);
        memory_instance.connect_ref("ADDR", &addr0);
        memory_instance.connect_ref("WE", &write_en);
        memory_instance.connect_ref("DONE", &done);
        module.add_instance(memory_instance);
    }

    // instantiate kernel
    let mut kernel_instance = v::Instance::new("kernel_inst", "main");
    module.add_decl(v::Decl::new_wire("kernel_start", 1));
    module.add_decl(v::Decl::new_wire("kernel_done", 1));
    kernel_instance.connect_ref("clk", "ap_clk");
    kernel_instance.connect_ref("go", "kernel_start");
    kernel_instance
        .connect("reset", v::Expr::new_logical_or("reset", "memories_sent"));
    kernel_instance.connect_ref("done", "kernel_done");
    for mem in memories {
        let read_data = format!("{}_read_data", mem);
        let done = format!("{}_done", mem);
        let addr0 = format!("{}_addr0", mem);
        let write_data = format!("{}_write_data", mem);
        let write_en = format!("{}_write_en", mem);
        let clk = format!("{}_clk", mem);
        kernel_instance.connect_ref(&read_data, &read_data);
        kernel_instance.connect_ref(&done, &done);
        kernel_instance.connect_ref(&addr0, &addr0);
        kernel_instance.connect_ref(&write_data, &write_data);
        kernel_instance.connect_ref(&write_en, &write_en);
        kernel_instance.connect_ref(&clk, "");
    }
    module.add_instance(kernel_instance);

    // add timeout counter
    module.add_decl(v::Decl::new_reg("counter", 32));
    module.add_stmt(utils::cond_non_blk_assign(
        "ap_clk",
        "counter",
        vec![
            (
                Some("ap_start".into()),
                v::Expr::new_add("counter", v::Expr::new_ulit_dec(32, "1")),
            ),
            (None, v::Expr::new_ulit_dec(32, "0")),
        ],
    ));

    // done signal
    module.add_stmt(v::Parallel::Assign(
        "ap_done".into(),
        v::Expr::new_logical_or(
            v::Expr::new_gt("counter", "timeout"),
            v::Expr::new_eq(
                "memories_sent",
                v::Expr::new_ulit_bin(memories.len() as u32, "1"),
            ),
        ),
    ));

    module
}

fn host_transfer_fsm(module: &mut v::Module, memories: &[String]) {
    module.add_decl(v::Decl::new_wire("memories_copied", 1));
    // module.add_decl(v::Decl::new_wire("memories_sent", 1));
    module.add_decl(v::Decl::new_reg("memories_sent", memories.len() as u64));
    module.add_stmt(v::Parallel::Assign(
        "memories_copied".into(),
        if memories.is_empty() {
            panic!("Need some memories")
        } else if memories.len() == 1 {
            format!("{}_copy_done", memories[0]).into()
        } else {
            memories[1..].iter().fold(
                format!("{}_copy_done", memories[0]).into(),
                |acc, elem| {
                    v::Expr::new_logical_and(acc, format!("{}_copy_done", elem))
                },
            )
        },
    ));
    let copy_start_assigns: Vec<v::Expr> = memories
        .iter()
        .map(|mem| format!("{}_copy", mem).into())
        .collect();
    let send_start_assigns: Vec<v::Expr> = memories
        .iter()
        .map(|mem| format!("{}_send", mem).into())
        .collect();
    let fsm = fsm::LinearFsm::new("host_txn_", "ap_clk", "reset")
        .state("idle", &[], "ap_start") // idle state
        .state("copy", &copy_start_assigns, "memories_copied") // copy memory state
        .state("run_kernel", &["kernel_start".into()], "kernel_done") // run kernel state
        .state("send", &send_start_assigns, "memories_sent"); // send memory to host state

    let mut parallel = v::ParallelProcess::new_always();
    parallel.set_event(v::Sequential::new_posedge("ap_clk"));
    let mut ifelse = v::SequentialIfElse::new(fsm.state_is("send"));
    if memories.len() == 1 {
        ifelse.add_seq(v::Sequential::new_nonblk_assign(
            "memories_sent",
            format!("{}_send_done", memories[0]),
        ));
    } else {
        for (idx, mem) in memories.iter().enumerate() {
            ifelse.add_seq(v::Sequential::new_nonblk_assign(
                v::Expr::new_index_bit("memories_sent", idx as i32),
                format!("{}_send_done", mem),
            ));
        }
    }
    ifelse.set_else(v::Sequential::new_nonblk_assign("memories_sent", 0));
    parallel.add_seq(ifelse);
    module.add_stmt(parallel);
    fsm.emit(module);
    // module.add_stmt(v::Parallel::Assign(
    //     "memories_sent".into(),
    //     if memories.is_empty() {
    //         panic!("Need some memories")
    //     } else if memories.len() == 1 {
    //         format!("{}_send_done", memories[0]).into()
    //     } else {
    //         memories[1..].iter().fold(
    //             format!("{}_send_done", memories[0]).into(),
    //             |acc, elem| {
    //                 v::Expr::new_logical_and(acc, format!("{}_send_done", elem))
    //             },
    //         )
    //     },
    // ));
}
