use vast::v05::ast as v;

use super::{
    axi::{Axi4Lite, AxiChannel, ChannelDirection},
    fsm,
};
use crate::utils;
use std::rc::Rc;

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
            data_ports: vec![
                ("ID".to_string(), address_width / 8),
                ("ADDR".to_string(), address_width),
                ("LEN".to_string(), 8),
                ("SIZE".to_string(), 3),
            ],
        };
        let read_data = AxiChannel {
            prefix: format!("{}R", prefix),
            direction: ChannelDirection::Recv,
            state: vec![],
            data_ports: vec![
                ("ID".to_string(), address_width / 8),
                ("DATA".to_string(), data_width),
                ("RESP".to_string(), 2),
                ("LAST".to_string(), 1),
            ],
        };

        // write channels
        let write_address = AxiChannel {
            prefix: format!("{}AW", prefix),
            direction: ChannelDirection::Send,
            state: vec![],
            data_ports: vec![
                ("ID".to_string(), address_width / 8),
                ("ADDR".to_string(), address_width),
                ("LEN".to_string(), 8),
                ("SIZE".to_string(), 3),
            ],
        };
        let write_data = AxiChannel {
            prefix: format!("{}W", prefix),
            direction: ChannelDirection::Send,
            state: vec![],
            data_ports: vec![
                ("ID".to_string(), address_width / 8),
                ("DATA".to_string(), data_width),
                ("STRB".to_string(), data_width / 8),
                ("LAST".to_string(), 1),
            ],
        };
        let write_response = AxiChannel {
            prefix: format!("{}B", prefix),
            direction: ChannelDirection::Recv,
            state: vec![],
            data_ports: vec![
                ("ID".to_string(), address_width / 8),
                ("RESP".to_string(), 2),
            ],
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
        let memory_size = 32;
        let memory_size_bits: u64 = utils::math::bits_needed_for(memory_size); // TODO make memory size parametric
        let offset_size_bits = memory_size_bits + 1;

        module.add_input("ACLK", 1);
        module.add_input("ARESET", 1);

        // add axi interface ports
        let axi4 = Axi4Lite::memory_channels(address_width, data_width, "");
        axi4.add_ports_to(&mut module);

        module.add_input("BASE_ADDRESS", address_width);
        module.add_input("COPY_FROM_HOST", 1);
        module.add_output("COPY_FROM_HOST_DONE", 1);
        module.add_input("SEND_TO_HOST", 1);
        module.add_output("SEND_TO_HOST_DONE", 1);

        // BRAM interface
        module.add_input("WRITE_DATA", data_width);
        module.add_output("READ_DATA", data_width);
        module.add_input("ADDR", memory_size_bits);
        module.add_input("WE", 1);
        module.add_output("DONE", 1);

        // internal signals
        module.add_decl(v::Decl::new_wire("copy_done", 1));
        module.add_stmt(v::Parallel::Assign(
            "copy_done".into(),
            v::Expr::new_eq(
                "copy_addr_offset".into(),
                v::Expr::new_int(memory_size as i32),
            ),
        ));
        module.add_decl(v::Decl::new_wire("send_done", 1));
        module.add_stmt(v::Parallel::Assign(
            "send_done".into(),
            v::Expr::new_eq(
                "send_addr_offset".into(),
                v::Expr::new_int(memory_size as i32),
            ),
        ));

        // module mode fsm
        let mode_fsm = module_mode_fsm(&mut module);

        // bram reading / writing logic
        bram_logic(&axi4, &mut module, &mode_fsm);
        module.add_stmt(v::Parallel::Assign(
            "READ_DATA".into(),
            "bram_read_data".into(),
        ));

        // synchronise channels
        let read_controller = axi4
            .read_address
            .then(&axi4.read_data)
            .prefix("r")
            .trigger(mode_fsm.state_is("copy"));
        read_controller.emit(&mut module);

        // increment copy address offset
        module.add_decl(v::Decl::new_reg("copy_addr_offset", offset_size_bits));
        incr_addr(
            &mut module,
            mode_fsm.state_is("copy"),
            "copy_addr_offset",
            axi4.read_data.handshake(),
        );

        let shift_by = 2;
        let size = 2;

        module.add_stmt(axi4.read_address.assign("ID", 0));
        let mut concat = v::ExprConcat::default();
        concat.add_expr(v::Expr::new_shift_left("copy_addr_offset", shift_by));
        concat.add_expr(v::Expr::new_repeat(
            address_width - offset_size_bits,
            v::Expr::new_ulit_bin(1, "0"),
        ));
        module.add_stmt(
            axi4.read_address
                .assign("ADDR", v::Expr::new_add("BASE_ADDRESS", concat)),
        );
        module.add_stmt(axi4.read_address.assign("LEN", 0));
        module.add_stmt(axi4.read_address.assign("SIZE", size));

        let write_controller = axi4
            .write_address
            .then(&axi4.write_data)
            .then(&axi4.write_response)
            .prefix("w")
            .trigger(mode_fsm.state_is("send"));
        write_controller.emit(&mut module);

        // increment send address offset
        module.add_decl(v::Decl::new_reg("send_addr_offset", offset_size_bits));
        incr_addr(
            &mut module,
            mode_fsm.state_is("send"),
            "send_addr_offset",
            axi4.write_response.handshake(),
        );

        module.add_stmt(axi4.write_address.assign("ID", 0));
        let mut concat = v::ExprConcat::default();
        concat.add_expr(v::Expr::new_shift_left("send_addr_offset", shift_by));
        concat.add_expr(v::Expr::new_repeat(
            address_width - offset_size_bits,
            v::Expr::new_ulit_bin(1, "0"),
        ));
        module.add_stmt(
            axi4.write_address
                .assign("ADDR", v::Expr::new_add("BASE_ADDRESS", concat)),
        );
        module.add_stmt(axi4.write_address.assign("LEN", 0));
        module.add_stmt(axi4.write_address.assign("SIZE", size));

        // write data channel
        module.add_stmt(axi4.write_data.assign("ID", 0));
        module.add_stmt(axi4.write_data.assign("DATA", "bram_read_data"));
        module.add_stmt(axi4.write_data.assign(
            "STRB",
            v::Expr::new_repeat(data_width / 8, v::Expr::new_ulit_bin(1, "1")),
        ));
        module.add_stmt(axi4.write_data.assign("LAST", 1));

        module
    }
}

fn module_mode_fsm(module: &mut v::Module) -> fsm::LinearFsm {
    // states:
    //  0: idle, start when COPY_TO_HOST
    //  1: copy to host, trans when
    //  2: act as a bram
    //  3: send to host
    // TODO assign to done signals when counter reaches limit
    let fsm = fsm::LinearFsm::new("memory_mode_", "ACLK", "ARESET")
        .state("idle", &[], "COPY_FROM_HOST") // idle: wait for COPY_FROM_HOST
        .state("copy", &[], "copy_done") // copy data from host into local bram
        .state("bram", &["COPY_FROM_HOST_DONE".into()], "SEND_TO_HOST") // act as bram
        .state("send", &[], "send_done") // send data to host from local bram
        .state("done", &["SEND_TO_HOST_DONE".into()], "ARESET"); // send data to host from local bram
    fsm.emit(module);
    fsm
}

fn bram_logic(
    axi4: &Axi4Lite,
    module: &mut v::Module,
    mode_fsm: &fsm::LinearFsm,
) {
    module.add_decl(v::Decl::new_wire("bram_addr", 5));
    module.add_decl(v::Decl::new_wire("bram_write_data", 32));
    module.add_decl(v::Decl::new_wire("bram_we", 1));
    module.add_decl(v::Decl::new_wire("bram_read_data", 32));
    module.add_decl(v::Decl::new_wire("bram_done", 1));

    let mut ram_instance = v::Instance::new("bram", "SINGLE_PORT_BRAM");
    ram_instance.connect_ref("ACLK", "ACLK");
    ram_instance.connect_ref("ADDR", "bram_addr");
    ram_instance.connect_ref("Din", "bram_write_data");
    ram_instance.connect_ref("WE", "bram_we");
    ram_instance.connect_ref("Dout", "bram_read_data");
    ram_instance.connect_ref("Done", "bram_done");
    module.add_instance(ram_instance);
    module.add_stmt(v::Parallel::Assign("DONE".into(), "bram_done".into()));

    // bram address logic
    let copy_address = v::Expr::new_slice("copy_addr_offset", 4, 0);
    let bram_address: v::Expr = "ADDR".into();
    let send_address = v::Expr::new_slice("send_addr_offset", 4, 0);
    let mux_address = v::Expr::new_mux(
        v::Expr::new_logical_and(
            axi4.read_data.handshake(),
            mode_fsm.state_is("copy"),
        ),
        copy_address,
        v::Expr::new_mux(
            mode_fsm.state_is("bram"),
            bram_address,
            v::Expr::new_mux(mode_fsm.state_is("send"), send_address, 0.into()),
        ),
    );
    module.add_stmt(v::Parallel::Assign("bram_addr".into(), mux_address));

    // bram write enable
    let copy_we: v::Expr = 1.into();
    let bram_we: v::Expr = "WE".into();
    let mux_we = v::Expr::new_mux(
        v::Expr::new_logical_and(
            axi4.read_data.handshake(),
            mode_fsm.state_is("copy"),
        ),
        copy_we,
        v::Expr::new_mux(mode_fsm.state_is("bram"), bram_we, 0.into()),
    );
    module.add_stmt(v::Parallel::Assign("bram_we".into(), mux_we));

    // bram write data
    let copy_data: v::Expr = axi4.read_data.get("DATA").into();
    let bram_data: v::Expr = "WRITE_DATA".into();
    let mux_data = v::Expr::new_mux(
        v::Expr::new_logical_and(
            axi4.read_data.handshake(),
            mode_fsm.state_is("copy"),
        ),
        copy_data,
        v::Expr::new_mux(mode_fsm.state_is("bram"), bram_data, 0.into()),
    );
    module.add_stmt(v::Parallel::Assign("bram_write_data".into(), mux_data));
}

fn incr_addr(
    module: &mut v::Module,
    mode_condition: v::Expr,
    offset_reg: &str,
    condition: v::Expr,
) {
    let mut always = v::ParallelProcess::new_always();
    always.set_event(v::Sequential::new_posedge("ACLK"));

    let mut mode_if = v::SequentialIfElse::new(mode_condition);
    let mut ifelse = v::SequentialIfElse::new(condition.into());
    ifelse.add_seq(v::Sequential::new_nonblk_assign(
        offset_reg.into(),
        v::Expr::new_add(offset_reg, 1),
    ));
    ifelse.set_else(v::Sequential::new_nonblk_assign(
        offset_reg.into(),
        offset_reg.into(),
    ));

    mode_if.add_seq(ifelse.into());
    mode_if.set_else(v::Sequential::new_nonblk_assign(
        offset_reg.into(),
        v::Expr::new_int(0),
    ));
    always.add_seq(mode_if.into());

    module.add_stmt(always);
}

pub fn bram() -> v::Module {
    let mut module = v::Module::new("SINGLE_PORT_BRAM");
    // module.add_param_uint("DATA_WIDTH", 32);
    // module.add_param_uint("SIZE", 32);
    // module.add_param_uint("ADDR_WIDTH", 5);
    module.add_input("ACLK", 1);
    module.add_input("ADDR", 5);
    module.add_input("Din", 32);
    module.add_input("WE", 1);
    module.add_output("Dout", 32);
    module.add_output("Done", 1);

    let mut attr = v::Attribute::default();
    attr.add_stmt("ram_style", "block");
    module.add_decl(v::Decl::AttributeDecl(
        attr,
        Rc::new(v::Decl::new_array("ram_core", 32, 32)),
    ));
    // module.add_decl(v::Decl::new_array("ram_core", 32, 32));

    let mut write_block = v::ParallelProcess::new_always();
    write_block.set_event(v::Sequential::new_posedge("ACLK"));
    let mut if_we = v::SequentialIfElse::new("WE".into());
    if_we.add_seq(v::Sequential::new_nonblk_assign(
        v::Expr::new_index_expr("ram_core", "ADDR".into()),
        "Din".into(),
    ));
    write_block.add_seq(if_we.into());
    module.add_stmt(write_block);

    module.add_decl(v::Decl::new_reg("done_reg", 1));
    let mut we_block = v::ParallelProcess::new_always();
    we_block.set_event(v::Sequential::new_posedge("ACLK"));
    let mut if_we = v::SequentialIfElse::new("WE".into());
    if_we.add_seq(v::Sequential::new_nonblk_assign(
        "done_reg".into(),
        1.into(),
    ));
    if_we.set_else(v::Sequential::new_nonblk_assign(
        "done_reg".into(),
        0.into(),
    ));
    we_block.add_seq(if_we.into());
    module.add_stmt(we_block);
    module.add_stmt(v::Parallel::Assign("Done".into(), "done_reg".into()));

    module.add_stmt(v::Parallel::Assign(
        "Dout".into(),
        v::Expr::new_index_expr("ram_core", "ADDR".into()),
    ));

    module
}
