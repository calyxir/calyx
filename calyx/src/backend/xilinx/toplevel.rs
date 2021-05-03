use super::{
    axi, control_axi::ControlInterface, fsm, memory_axi::MemoryInterface,
};
use crate::{
    backend::traits::Backend,
    errors::{Error, FutilResult},
    ir,
};
use vast::v05::ast as v;

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
            .find(|comp| comp.attributes.has("toplevel"))
            .ok_or_else(|| Error::Misc("no toplevel".to_string()))?;
        let memories = external_memories(toplevel);

        let mut modules = vec![
            top_level(12, 32, &memories),
            axi::Axi4Lite::control_module("Control_axi", 12, 32, &memories),
        ];

        for (i, _mem) in memories.iter().enumerate() {
            modules.push(axi::Axi4Lite::memory_module(
                &format!("Memory_controller_axi_{}", i),
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
            "{}\n{}{}",
            "`default_nettype none",
            module_string,
            "`default_nettype wire"
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
    let axi4 = axi::Axi4Lite::control_channels(
        address_width,
        data_width,
        "s_axi_control_",
    );
    axi4.add_ports_to(&mut module);

    // TODO: axi master interfaces
    for (idx, _mem) in memories.iter().enumerate() {
        axi::Axi4Lite::memory_channels(64, 32, &format!("m{}_axi_", idx))
            .add_ports_to(&mut module);
    }

    // wires
    module.add_stmt(v::Decl::new_wire("ap_start", 1));
    module.add_stmt(v::Decl::new_wire("ap_done", 1));
    module.add_stmt(v::Decl::new_wire("timeout", 32));
    for mem in memories {
        module.add_stmt(v::Decl::new_wire(mem, 64));
    }

    // debugging
    module.add_output("DBG_ap_start", 1);
    module.add_output("DBG_ap_done", 1);
    module.add_output("DBG_timeout", 1);
    module.add_output("DBG_counter", 32);

    module.add_stmt(v::Parallel::Assign(
        "DBG_ap_start".into(),
        "ap_start".into(),
    ));
    module
        .add_stmt(v::Parallel::Assign("DBG_ap_done".into(), "ap_done".into()));
    module
        .add_stmt(v::Parallel::Assign("DBG_timeout".into(), "timeout".into()));
    module
        .add_stmt(v::Parallel::Assign("DBG_counter".into(), "counter".into()));

    // TODO: have real interrupt support
    // module.add_stmt(v::Parallel::Assign(
    //     "ap_interrupt".into(),
    //     v::Expr::new_ulit_bin(1, "0"),
    // ));

    // instantiate control interface
    let base_control_axi_interface =
        axi::Axi4Lite::control_channels(address_width, data_width, "");
    let mut control_instance =
        v::Instance::new("inst_control_axi", "Control_axi");
    control_instance.connect("ACLK", "ap_clk");
    control_instance.connect("ARESET", v::Expr::new_not("ap_rst_n"));
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

    for mem in memories {
        module.add_decl(v::Decl::new_wire(&format!("{}_copy_done", mem), 1));
        module.add_decl(v::Decl::new_wire(&format!("{}_send_done", mem), 1));
    }
    host_transfer_fsm(&mut module, memories);

    // instantiate memory controllers
    let base_master_axi_interface = axi::Axi4Lite::memory_channels(64, 32, "");
    for (idx, mem) in memories.iter().enumerate() {
        let write_data = format!("{}_write_data", mem);
        let read_data = format!("{}_read_data", mem);
        let addr0 = format!("{}_addr0", mem);
        let write_en = format!("{}_write_en", mem);
        let done = format!("{}_done", mem);
        module.add_decl(v::Decl::new_wire(&write_data, 1));
        module.add_decl(v::Decl::new_wire(&read_data, 1));
        module.add_decl(v::Decl::new_wire(&addr0, 1));
        module.add_decl(v::Decl::new_wire(&write_en, 1));
        module.add_decl(v::Decl::new_wire(&done, 1));

        let mut memory_instance = v::Instance::new(
            &format!("inst_mem_controller_axi_{}", idx),
            &format!("Memory_controller_axi_{}", idx),
        );
        memory_instance.connect("ACLK", "ap_clk");
        memory_instance.connect("ARESET", v::Expr::new_not("ap_rst_n"));
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
    let mut kernel_instance = v::Instance::new("kernel_inst", "Kernel");
    module.add_decl(v::Decl::new_wire("kernel_start", 1));
    module.add_decl(v::Decl::new_wire("kernel_done", 1));
    kernel_instance.connect_ref("clk", "ap_clk");
    kernel_instance.connect_ref("go", "kernel_start");
    kernel_instance.connect_ref("done", "kernel_done");
    for mem in memories {
        let read_data = format!("{}_read_data", mem);
        let done = format!("{}_done", mem);
        let addr0 = format!("{}_addr0", mem);
        let write_data = format!("{}_write_data", mem);
        let write_en = format!("{}_write_en", mem);
        kernel_instance.connect_ref(&read_data, &read_data);
        kernel_instance.connect_ref(&done, &done);
        kernel_instance.connect_ref(&addr0, &addr0);
        kernel_instance.connect_ref(&write_data, &write_data);
        kernel_instance.connect_ref(&write_en, &write_en);
    }
    module.add_instance(kernel_instance);

    // add timeout counter
    module.add_decl(v::Decl::new_reg("counter", 32));
    let mut always_counter = v::ParallelProcess::new_always();
    always_counter.set_event(v::Sequential::new_posedge("ap_clk"));
    let mut reset_if = v::SequentialIfElse::new("ap_start".into());
    reset_if.add_seq(v::Sequential::new_nonblk_assign(
        "counter".into(),
        v::Expr::new_add("counter".into(), v::Expr::new_ulit_dec(32, "1")),
    ));
    reset_if.set_else(v::Sequential::new_nonblk_assign(
        "counter".into(),
        v::Expr::new_ulit_dec(32, "0"),
    ));
    always_counter.add_seq(reset_if.into());
    module.add_stmt(always_counter);

    // done signal
    module.add_stmt(v::Parallel::Assign(
        "ap_done".into(),
        v::Expr::new_logical_or(
            v::Expr::new_gt("counter", "timeout"),
            "memories_sent".into(),
        ),
    ));

    module
}

fn host_transfer_fsm(module: &mut v::Module, memories: &[String]) {
    module.add_decl(v::Decl::new_wire("memories_copied", 1));
    module.add_decl(v::Decl::new_wire("memories_sent", 1));
    module.add_stmt(v::Parallel::Assign(
        "memories_copied".into(),
        if memories.len() == 0 {
            panic!("Need some memories")
        } else if memories.len() == 1 {
            format!("{}_copy_done", memories[0]).into()
        } else {
            memories[1..].iter().fold(
                format!("{}_copy_done", memories[0]).into(),
                |acc, elem| {
                    v::Expr::new_logical_and(
                        acc,
                        format!("{}_copy_done", elem).into(),
                    )
                },
            )
        },
    ));
    module.add_stmt(v::Parallel::Assign(
        "memories_sent".into(),
        if memories.len() == 0 {
            panic!("Need some memories")
        } else if memories.len() == 1 {
            format!("{}_send_done", memories[0]).into()
        } else {
            memories[1..].iter().fold(
                format!("{}_send_done", memories[0]).into(),
                |acc, elem| {
                    v::Expr::new_logical_and(
                        acc,
                        format!("{}_send_done", elem).into(),
                    )
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
    fsm::LinearFsm::new("host_txn_")
        .state(&[], "ap_start") // idle state
        .state(&copy_start_assigns, "memories_copied") // copy memory state
        .state(&["kernel_start".into()], "kernel_done") // run kernel state
        .state(&send_start_assigns, "memories_sent") // send memory to host state
        .emit(module);

    // let state = "host_txn_state";
    // let next = "host_txn_next";

    // // state mapping:
    // //  0: idle, waiting for ap_start
    // //  1: copy memory from host, waiting for copying to finish
    // //  2: run kernel, waiting for kernel to finish
    // //  3: send memory to host

    // // fsm that controls when to read / write to memories
    // module.add_decl(v::Decl::new_reg(state, 2));
    // module.add_decl(v::Decl::new_reg(next, 2));

    // // fsm update block
    // let mut parallel = v::ParallelProcess::new_always();
    // parallel.set_event(v::Sequential::new_posedge("ACLK"));

    // let mut ifelse = v::SequentialIfElse::new("ARESET".into());
    // ifelse.add_seq(v::Sequential::new_nonblk_assign(
    //     state.into(),
    //     v::Expr::new_int(0),
    // ));
    // ifelse
    //     .set_else(v::Sequential::new_nonblk_assign(state.into(), next.into()));

    // parallel.add_seq(ifelse.into());
    // module.add_stmt(parallel);

    // module.add_decl(v::Decl::new_wire("memories_copied", 1));
    // module.add_decl(v::Decl::new_wire("memories_sent", 1));
    // module.add_stmt(v::Parallel::Assign(
    //     "memories_copied".into(),
    //     if memories.len() == 0 {
    //         panic!("Need some memories")
    //     } else if memories.len() == 1 {
    //         format!("{}_copy_done", memories[0]).into()
    //     } else {
    //         memories[1..].iter().fold(
    //             format!("{}_copy_done", memories[0]).into(),
    //             |acc, elem| {
    //                 v::Expr::new_logical_and(
    //                     acc,
    //                     format!("{}_copy_done", elem).into(),
    //                 )
    //             },
    //         )
    //     },
    // ));
    // module.add_stmt(v::Parallel::Assign(
    //     "memories_sent".into(),
    //     if memories.len() == 0 {
    //         panic!("Need some memories")
    //     } else if memories.len() == 1 {
    //         format!("{}_send_done", memories[0]).into()
    //     } else {
    //         memories[1..].iter().fold(
    //             format!("{}_send_done", memories[0]).into(),
    //             |acc, elem| {
    //                 v::Expr::new_logical_and(
    //                     acc,
    //                     format!("{}_send_done", elem).into(),
    //                 )
    //             },
    //         )
    //     },
    // ));
    // for mem in memories {
    //     module.add_stmt(v::Parallel::Assign(
    //         format!("{}_copy", mem).into(),
    //         v::Expr::new_eq(state.into(), v::Expr::new_int(1)),
    //     ));
    //     module.add_stmt(v::Parallel::Assign(
    //         format!("{}_send", mem).into(),
    //         v::Expr::new_eq(state.into(), v::Expr::new_int(3)),
    //     ));
    // }
    // module.add_stmt(v::Parallel::Assign(
    //     "kernel_start".into(),
    //     v::Expr::new_eq(state.into(), v::Expr::new_int(2)),
    // ));

    // let mut parallel = v::ParallelProcess::new_always();
    // parallel.set_event(v::Sequential::Wildcard);

    // let mut case = v::Case::new(state.into());

    // // idle state
    // let mut idle_state = v::CaseBranch::new(v::Expr::new_int(0));
    // let mut idle_if = v::SequentialIfElse::new("ap_start".into());
    // idle_if.add_seq(v::Sequential::new_blk_assign(
    //     state.into(),
    //     v::Expr::new_int(1),
    // ));
    // idle_if.set_else(v::Sequential::new_blk_assign(
    //     state.into(),
    //     v::Expr::new_int(0),
    // ));
    // idle_state.add_seq(idle_if.into());
    // case.add_branch(idle_state);

    // // copy from host state
    // let mut copy_from_host_state = v::CaseBranch::new(v::Expr::new_int(1));
    // let mut copy_if = v::SequentialIfElse::new("memories_copied".into());
    // copy_if.add_seq(v::Sequential::new_blk_assign(
    //     state.into(),
    //     v::Expr::new_int(2),
    // ));
    // copy_if.set_else(v::Sequential::new_blk_assign(
    //     state.into(),
    //     v::Expr::new_int(1),
    // ));
    // copy_from_host_state.add_seq(copy_if.into());
    // case.add_branch(copy_from_host_state);

    // // run kernel state
    // let mut run_kernel_state = v::CaseBranch::new(v::Expr::new_int(2));
    // let mut kernel_if = v::SequentialIfElse::new("kernel_done".into());
    // kernel_if.add_seq(v::Sequential::new_blk_assign(
    //     state.into(),
    //     v::Expr::new_int(3),
    // ));
    // kernel_if.set_else(v::Sequential::new_blk_assign(
    //     state.into(),
    //     v::Expr::new_int(2),
    // ));
    // run_kernel_state.add_seq(kernel_if.into());
    // case.add_branch(run_kernel_state);

    // // send to host state
    // let mut send_to_host_state = v::CaseBranch::new(v::Expr::new_int(3));
    // let mut send_if = v::SequentialIfElse::new("memories_sent".into());
    // send_if.add_seq(v::Sequential::new_blk_assign(
    //     state.into(),
    //     v::Expr::new_int(0),
    // ));
    // send_if.set_else(v::Sequential::new_blk_assign(
    //     state.into(),
    //     v::Expr::new_int(3),
    // ));
    // send_to_host_state.add_seq(send_if.into());
    // case.add_branch(send_to_host_state);

    // // default case
    // let mut default = v::CaseDefault::default();
    // default.add_seq(v::Sequential::new_blk_assign(
    //     next.into(),
    //     v::Expr::new_int(0),
    // ));
    // case.set_default(default);

    // parallel.add_seq(v::Sequential::new_case(case));
    // module.add_stmt(parallel)
}
