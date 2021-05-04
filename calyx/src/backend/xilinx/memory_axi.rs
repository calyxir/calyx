use vast::v05::ast as v;

use super::{
    axi::{Axi4Lite, AxiChannel, ChannelDirection},
    fsm,
};

pub trait MemoryInterface {
    fn memory_channels(
        address_width: u64,
        data_width: u64,
        prefix: &str,
    ) -> Self;
    fn memory_module(
        name: &str,
        address_width: u64,
        data_width: u64,
    ) -> v::Module;
}

impl MemoryInterface for Axi4Lite {
    fn memory_channels(
        address_width: u64,
        data_width: u64,
        prefix: &str,
    ) -> Self {
        // read channels
        let read_address = AxiChannel {
            prefix: format!("{}AR", prefix),
            direction: ChannelDirection::Send,
            state: vec![],
            inputs: vec![("ADDR".to_string(), address_width)],
            outputs: vec![],
        };
        let read_data = AxiChannel {
            prefix: format!("{}R", prefix),
            direction: ChannelDirection::Recv,
            state: vec![],
            inputs: vec![],
            outputs: vec![
                ("DATA".to_string(), data_width),
                ("RESP".to_string(), 2),
            ],
        };

        // write channels
        let write_address = AxiChannel {
            prefix: format!("{}AW", prefix),
            direction: ChannelDirection::Send,
            state: vec![v::Decl::new_reg("waddr", address_width)],
            inputs: vec![("ADDR".to_string(), address_width)],
            outputs: vec![],
        };
        let write_data = AxiChannel {
            prefix: format!("{}W", prefix),
            direction: ChannelDirection::Send,
            state: vec![v::Decl::new_wire("wdata", data_width)],
            inputs: vec![("DATA".to_string(), data_width)],
            outputs: vec![],
        };
        let write_response = AxiChannel {
            prefix: format!("{}B", prefix),
            direction: ChannelDirection::Recv,
            state: vec![],
            inputs: vec![],
            outputs: vec![("RESP".to_string(), 2)],
        };
        Self {
            read_address,
            read_data,
            write_address,
            write_data,
            write_response,
        }
    }

    fn memory_module(
        name: &str,
        address_width: u64,
        data_width: u64,
    ) -> v::Module {
        let mut module = v::Module::new(name);

        module.add_input("ACLK", 1);
        module.add_input("ARESET", 1);

        // add axi interface ports
        let axi4 = Axi4Lite::memory_channels(address_width, data_width, "");
        axi4.add_ports_to(&mut module);

        module.add_input("BASE_ADDRESS", 1);
        module.add_input("COPY_FROM_HOST", 1);
        module.add_output("COPY_FROM_HOST_DONE", 1);
        module.add_input("SEND_TO_HOST", 1);
        module.add_output("SEND_TO_HOST_DONE", 1);

        // BRAM interface
        module.add_input("WRITE_DATA", data_width);
        module.add_output("READ_DATA", data_width);
        module.add_input("ADDR", address_width);
        module.add_input("WE", 1);
        module.add_output("DONE", 1);

        // module mode fsm
        let mode_fsm = module_mode_fsm(&mut module);

        // Instantiate BRAM
        module.add_decl(v::Decl::new_array("bram", data_width, 32));
        // TODO: try making this a wire? but then I think a BRAM won't be inferred
        module.add_decl(v::Decl::new_wire("bram_data", data_width));
        module.add_decl(v::Decl::new_reg("write_done", 1));

        // bram reading / writing logic
        bram_logic(&mut module, &mode_fsm);

        module.add_decl(v::Decl::new_reg("int_addr_offset", address_width));

        // synchronise channels
        let read_controller = axi4
            .read_address
            .then(&axi4.read_data)
            .prefix("r")
            .trigger(mode_fsm.state_is("copy"));
        read_controller.emit(&mut module);
        module.add_stmt(v::Parallel::Assign("raddr".into(), "ARADDR".into()));
        module.add_stmt(v::Parallel::Assign("RDATA".into(), "rdata".into()));
        module
            .add_stmt(v::Parallel::Assign("RRESP".into(), v::Expr::new_int(0)));

        // let write_controller = axi4
        //     .write_address
        //     .then(&axi4.write_data)
        //     .then(&axi4.write_response)
        //     .prefix("w");
        // write_controller.emit(&mut module);

        // let mut always = v::ParallelProcess::new_always();
        // always.set_event(v::Sequential::new_posedge("ACLK"));
        // let mut reset_if = v::SequentialIfElse::new("ARESET".into());
        // reset_if.add_seq(v::Sequential::new_nonblk_assign(
        //     "int_addr_offset".into(),
        //     v::Expr::new_int(0),
        // ));
        // let mut incr_if =
        //     v::SequentialIfElse::new(axi4.write_response.handshake());
        // incr_if.add_seq(v::Sequential::new_nonblk_assign(
        //     "int_addr_offset".into(),
        //     v::Expr::new_add("int_addr_offset".into(), v::Expr::new_int(4)),
        // ));
        // incr_if.set_else(v::Sequential::new_nonblk_assign(
        //     "int_addr_offset".into(),
        //     "int_addr_offset".into(),
        // ));
        // reset_if.set_else(incr_if.into());
        // always.add_seq(reset_if.into());
        // module.add_stmt(always);

        // module.add_stmt(v::Parallel::Assign(
        //     "AWADDR".into(),
        //     v::Expr::new_add("BASE_ADDRESS", "int_addr_offset"),
        // ));

        module
    }
}

fn module_mode_fsm(module: &mut v::Module) -> fsm::LinearFsm {
    // states:
    //  0: idle, start when COPY_TO_HOST
    //  1: copy to host, trans when
    //  2: act as a bram
    //  3: send to host
    module.add_decl(v::Decl::new_wire("copy_done", 1));
    module.add_decl(v::Decl::new_wire("send_done", 1));
    // TODO assign to done signals when counter reaches limit
    let fsm = fsm::LinearFsm::new("memory_mode_")
        .state("idle", &[], "COPY_FROM_HOST") // idle: wait for COPY_FROM_HOST
        .state("copy", &[], "copy_done") // copy data from host into local bram
        .state("bram", &["COPY_FROM_HOST_DONE".into()], "SEND_TO_HOST") // act as bram
        .state("send", &[], "send_done") // send data to host from local bram
        .state("done", &["SEND_TO_HOST_DONE".into()], "ARESET"); // send data to host from local bram
    fsm.emit(module);
    fsm
}

fn bram_logic(module: &mut v::Module, mode_fsm: &fsm::LinearFsm) {
    let mut bram_always = v::ParallelProcess::new_always();
    bram_always.set_event(v::Sequential::new_posedge("ACLK"));
    let mut if_mode_bram = v::SequentialIfElse::new(mode_fsm.state_is("bram"));
    let mut if_we_bram = v::SequentialIfElse::new("WE".into());
    if_we_bram.add_seq(v::Sequential::new_nonblk_assign(
        v::Expr::new_index_expr("bram", "ADDR".into()),
        "WRITE_DATA".into(),
    ));
    if_we_bram.add_seq(v::Sequential::new_nonblk_assign(
        "write_done".into(),
        v::Expr::new_int(1),
    ));
    let mut if_we_bram_else = v::SequentialIfElse::default();
    if_we_bram_else.add_seq(v::Sequential::new_nonblk_assign(
        "bram_data".into(),
        v::Expr::new_index_expr("bram", "ADDR".into()),
    ));
    if_we_bram_else.add_seq(v::Sequential::new_nonblk_assign(
        "write_done".into(),
        v::Expr::new_int(0),
    ));
    if_we_bram.set_else(if_we_bram_else.into());
    if_mode_bram.add_seq(if_we_bram.into());

    bram_always.add_seq(if_mode_bram.into());
    module.add_stmt(bram_always);
    module
        .add_stmt(v::Parallel::Assign("READ_DATA".into(), "bram_data".into()));
}
