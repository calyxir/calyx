use super::{
    axi::{AxiChannel, AxiInterface, ChannelDirection},
    fsm,
};
use calyx_utils as utils;
use std::rc::Rc;
use vast::v05::ast as v;

/// Represents the interface for a AXI master that controls
/// a memory with an AXI slave interface.
pub trait MemoryInterface {
    fn memory_channels(
        address_width: u64,
        data_width: u64,
        prefix: &str,
    ) -> Self;
    fn memory_module(
        name: &str,
        bus_data_width: u64,
        bus_addr_width: u64,
        data_width: u64,
        memory_size: u64,
        addr_width: u64,
    ) -> v::Module;
}

impl MemoryInterface for AxiInterface {
    fn memory_channels(
        address_width: u64,
        data_width: u64,
        prefix: &str,
    ) -> Self {
        let addr_data_ports = vec![
            ("ID".to_string(), address_width / 8),
            ("ADDR".to_string(), address_width),
            ("LEN".to_string(), 8),
            ("SIZE".to_string(), 3),
            ("BURST".to_string(), 2),
        ];
        // read channels
        let read_address = AxiChannel {
            prefix: format!("{}AR", prefix),
            direction: ChannelDirection::Send,
            state: vec![],
            data_ports: addr_data_ports.clone(),
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
            data_ports: addr_data_ports,
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
        bus_data_width: u64,
        bus_addr_width: u64,
        data_width: u64,
        memory_size: u64,
        addr_width: u64,
    ) -> v::Module {
        let mut module = v::Module::new(name);

        module.add_input("ACLK", 1);
        module.add_input("ARESET", 1);

        // add axi interface ports
        let axi4 = AxiInterface::memory_channels(bus_addr_width, 512, "");
        axi4.add_ports_to(&mut module);

        module.add_input("BASE_ADDRESS", bus_addr_width);
        module.add_input("COPY_FROM_HOST", 1);
        module.add_output("COPY_FROM_HOST_DONE", 1);
        module.add_input("SEND_TO_HOST", 1);
        module.add_output("SEND_TO_HOST_DONE", 1);

        // BRAM interface
        module.add_input("WRITE_DATA", data_width);
        module.add_output("READ_DATA", data_width);
        module.add_input("ADDR", addr_width);
        module.add_input("WE", 1);
        module.add_output("DONE", 1);

        // internal signals
        module.add_decl(v::Decl::new_wire("copy_done", 1));
        module.add_stmt(v::Parallel::Assign(
            "copy_done".into(),
            v::Expr::new_eq("copy_addr_offset", memory_size as i32),
        ));
        module.add_decl(v::Decl::new_wire("send_done", 1));
        module.add_stmt(v::Parallel::Assign(
            "send_done".into(),
            v::Expr::new_eq("send_addr_offset", memory_size as i32),
        ));

        // module mode fsm
        let mode_fsm = module_mode_fsm(&mut module);
        // module.add_stmt(v::Parallel::Assign(
        //     "SEND_TO_HOST_DONE".into(),
        //     v::Expr::new_mux(mode_fsm.state_is("send"), "send_done", 0),
        // ));

        // count the number of read transactions we've received
        module.add_decl(v::Decl::new_reg(
            "read_txn_count",
            utils::bits_needed_for(bus_data_width / data_width),
        ));
        module.add_stmt(super::utils::cond_non_blk_assign(
            "ACLK",
            "read_txn_count",
            vec![
                (Some("ARESET".into()), 0.into()),
                (
                    Some(axi4.read_data.handshake()),
                    v::Expr::new_add("read_txn_count", 1),
                ),
            ],
        ));

        // bram reading / writing logic
        bram_logic(
            name,
            &axi4,
            &mut module,
            &mode_fsm,
            "read_txn_count".into(),
            data_width,
            addr_width,
        );
        module.add_stmt(v::Parallel::Assign(
            "READ_DATA".into(),
            "bram_read_data".into(),
        ));

        // add 1 so offset can count up to memory size inclusively
        let offset_width = utils::bits_needed_for(memory_size) + 1;

        // synchronise channels
        let read_controller = axi4
            .read_address
            .then(&axi4.read_data)
            .prefix("r")
            .trigger(mode_fsm.next_state_is("copy"));
        read_controller.emit(&mut module);

        // increment copy address offset
        module.add_decl(v::Decl::new_reg("copy_addr_offset", offset_width));
        incr_addr(
            &mut module,
            mode_fsm.state_is("copy"),
            "copy_addr_offset",
            axi4.read_data.handshake(),
        );

        // addresses are byte addressed which means addresses are computed as
        // base + (offset << shift_by)
        // TODO(nathanielnrn): Fix the burst size and shifting values based on
        // pynq(?) input? Or memory size? unclear.
        let shift_by = 2;
        let burst_size = v::Expr::new_ulit_dec(
            3,
            &utils::bits_needed_for(data_width / 8).to_string(),
        );
        //AxBURST corresponds to type of burst as follows:
        // 0b00: Fixed
        // 0b01 (default): Incr
        // 0b10: Wrap
        // 0b11: Reserved
        let burst_type = v::Expr::new_ulit_bin(2, "01");

        module.add_stmt(axi4.read_address.assign("ID", 0));

        //add reg to store value of shift to circumvent order of operations
        //issue on bit shift
        let copy_shift = "copy_shift";
        module.add_decl(v::Decl::new_wire(copy_shift, bus_addr_width));
        let mut concat = v::ExprConcat::default();
        concat.add_expr("copy_addr_offset");
        concat.add_expr(v::Expr::new_repeat(
            bus_addr_width - offset_width,
            v::Expr::new_ulit_bin(1, "0"),
        ));
        module.add_stmt(v::Parallel::Assign(
            copy_shift.into(),
            v::Expr::new_shift_left(concat, shift_by),
        ));
        module.add_stmt(
            axi4.read_address
                .assign("ADDR", v::Expr::new_add("BASE_ADDRESS", copy_shift)),
        );
        // Currently we do not utilize burst capabilities, therefore AxLEN is 0
        module.add_stmt(axi4.read_address.assign("LEN", 0));
        module.add_stmt(axi4.read_address.assign("SIZE", burst_size.clone()));
        module.add_stmt(axi4.read_address.assign("BURST", burst_type.clone()));

        let write_controller = axi4
            .write_address
            .then(&axi4.write_data)
            .then(&axi4.write_response)
            .prefix("w")
            .trigger(mode_fsm.next_state_is("send"));
        write_controller.emit(&mut module);

        // increment send address offset
        module.add_decl(v::Decl::new_reg("send_addr_offset", offset_width));
        incr_addr(
            &mut module,
            mode_fsm.state_is("send"),
            "send_addr_offset",
            axi4.write_response.handshake(),
        );

        module.add_stmt(axi4.write_address.assign("ID", 0));
        // assign shift to a wire to circumvent `vast` order of operations issues
        //we shift to convert offset to byte length
        let send_shift = "send_shift";
        module.add_decl(v::Decl::new_wire(send_shift, bus_addr_width));
        let mut concat = v::ExprConcat::default();
        concat.add_expr("send_addr_offset");
        concat.add_expr(v::Expr::new_repeat(
            bus_addr_width - offset_width,
            v::Expr::new_ulit_bin(1, "0"),
        ));
        module.add_stmt(v::Parallel::Assign(
            send_shift.into(),
            v::Expr::new_shift_left(concat, shift_by),
        ));

        module.add_stmt(
            axi4.write_address
                .assign("ADDR", v::Expr::new_add("BASE_ADDRESS", send_shift)),
        );
        // Currently we do not utilize burst capabilities, therefore AxLEN is 0
        module.add_stmt(axi4.write_address.assign("LEN", 0));
        module.add_stmt(axi4.write_address.assign("SIZE", burst_size));
        module.add_stmt(axi4.write_address.assign("BURST", burst_type));

        // write data channel
        module.add_stmt(axi4.write_data.assign("ID", 0));
        let mut concat = v::ExprConcat::default();
        concat.add_expr("bram_read_data");
        concat.add_expr(v::Expr::new_repeat(
            (bus_data_width / data_width) - 1,
            v::Expr::new_ulit_bin(32, "0"),
        ));
        // used for alignment issues where host reads from WDATA at offset if
        // AWADDR is not alligned with data-bus-width
        let wdata = v::Expr::new_shift_left(
            concat,
            v::Expr::new_mul("send_addr_offset", data_width as i32),
        );
        module.add_stmt(axi4.write_data.assign("DATA", wdata));
        let mut concat = v::ExprConcat::default();
        concat.add_expr(v::Expr::new_ulit_hex(4, "F"));
        concat.add_expr(v::Expr::new_repeat(
            (bus_data_width / (8 * 4)) - 1,
            v::Expr::new_ulit_hex(4, "0"),
        ));
        let wstrb = v::Expr::new_shift_left(
            concat,
            //divided by 8 because WSTRB bits refer to entire bytes
            v::Expr::new_mul("send_addr_offset", (data_width / 8) as i32),
        );
        module.add_stmt(axi4.write_data.assign("STRB", wstrb));
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
    name: &str, //assumed to be of form [Memory_controller_axi_<suffix>]
    axi4: &AxiInterface,
    module: &mut v::Module,
    mode_fsm: &fsm::LinearFsm,
    txn_count: v::Expr,
    data_width: u64,
    addr_width: u64,
) {
    module.add_decl(v::Decl::new_wire("bram_addr", addr_width));
    module.add_decl(v::Decl::new_wire("bram_write_data", data_width));
    module.add_decl(v::Decl::new_wire("bram_we", 1));
    module.add_decl(v::Decl::new_wire("bram_read_data", data_width));
    module.add_decl(v::Decl::new_wire("bram_done", 1));
    let suffix_idx = "Memory_controller_axi_".len();
    let suffix = &name[suffix_idx..];
    let mut ram_instance =
        v::Instance::new("bram", &format!("SINGLE_PORT_BRAM_{}", suffix));
    ram_instance.connect_ref("ACLK", "ACLK");
    ram_instance.connect_ref("ADDR", "bram_addr");
    ram_instance.connect_ref("Din", "bram_write_data");
    ram_instance.connect_ref("WE", "bram_we");
    ram_instance.connect_ref("Dout", "bram_read_data");
    ram_instance.connect_ref("Done", "bram_done");
    module.add_instance(ram_instance);
    module.add_stmt(v::Parallel::Assign("DONE".into(), "bram_done".into()));

    // bram address logic
    let copy_address =
        v::Expr::new_slice("copy_addr_offset", (addr_width - 1) as i32, 0);
    let bram_address: v::Expr = "ADDR".into();
    let send_address =
        v::Expr::new_slice("send_addr_offset", (addr_width - 1) as i32, 0);
    let mux_address = v::Expr::new_mux(
        v::Expr::new_logical_and(
            axi4.read_data.handshake(),
            mode_fsm.state_is("copy"),
        ),
        copy_address,
        v::Expr::new_mux(
            mode_fsm.state_is("bram"),
            bram_address,
            v::Expr::new_mux(mode_fsm.state_is("send"), send_address, 0),
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
        v::Expr::new_mux(mode_fsm.state_is("bram"), bram_we, 0),
    );
    module.add_stmt(v::Parallel::Assign("bram_we".into(), mux_we));

    // bram write data
    let copy_data: v::Expr = v::Expr::new_index_slice(
        &axi4.read_data.get("DATA"),
        v::Expr::new_mul(txn_count, 32),
        data_width as u32, // bram data width
    );
    let bram_data: v::Expr = "WRITE_DATA".into();
    let mux_data = v::Expr::new_mux(
        v::Expr::new_logical_and(
            axi4.read_data.handshake(),
            mode_fsm.state_is("copy"),
        ),
        copy_data,
        v::Expr::new_mux(mode_fsm.state_is("bram"), bram_data, 0),
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
    let mut ifelse = v::SequentialIfElse::new(condition);
    ifelse.add_seq(v::Sequential::new_nonblk_assign(
        offset_reg,
        v::Expr::new_add(offset_reg, 1),
    ));
    ifelse.set_else(v::Sequential::new_nonblk_assign(offset_reg, offset_reg));

    mode_if.add_seq(ifelse);
    mode_if.set_else(v::Sequential::new_nonblk_assign(offset_reg, 0));
    always.add_seq(mode_if);

    module.add_stmt(always);
}

pub fn bram(
    name: &str,
    data_width: u64,
    memory_size: u64,
    addr_width: u64,
) -> v::Module {
    let mut module = v::Module::new(name);
    module.add_input("ACLK", 1);
    module.add_input("ADDR", addr_width);
    module.add_input("Din", data_width);
    module.add_input("WE", 1);
    module.add_output("Dout", data_width);
    module.add_output("Done", 1);

    let mut attr = v::Attribute::default();
    attr.add_stmt("ram_style", "block");
    module.add_decl(v::Decl::AttributeDecl(
        attr,
        Rc::new(v::Decl::new_array("ram_core", data_width, memory_size)),
    ));

    module.add_stmt(super::utils::cond_non_blk_assign(
        "ACLK",
        match memory_size {
            1 => "ram_core".into(),
            _ => v::Expr::new_index_expr("ram_core", "ADDR"),
        },
        vec![(Some("WE".into()), "Din".into())],
    ));

    module.add_decl(v::Decl::new_reg("done_reg", 1));
    module.add_stmt(super::utils::cond_non_blk_assign(
        "ACLK",
        "done_reg",
        vec![(Some("WE".into()), 1.into()), (None, 0.into())],
    ));

    module.add_stmt(v::Parallel::Assign(
        "Dout".into(),
        match memory_size {
            1 => "ram_core".into(),
            _ => v::Expr::new_index_expr("ram_core", "ADDR"),
        },
    ));
    //add a simple assign Done = done_reg
    module.add_stmt(v::Parallel::Assign("Done".into(), "done_reg".into()));
    module
}
