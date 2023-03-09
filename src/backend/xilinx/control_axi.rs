use vast::v05::ast as v;

use super::axi::{AxiChannel, AxiInterface, ChannelDirection};
use super::axi_address_space::{AddressSpace, Flags};

/// Represents the AXI control interface that Xilinx expects
/// kernels to have.
pub trait ControlInterface {
    fn control_channels(
        address_width: u64,
        data_width: u64,
        prefix: &str,
    ) -> Self;
    fn control_module(
        name: &str,
        address_width: u64,
        data_width: u64,
        memories: &[String],
    ) -> v::Module;
}

/// Generate the base address space for the Xilinx control interface.
fn axi_address_space(
    axi: &AxiInterface,
    address_width: u64,
    data_width: u64,
) -> AddressSpace {
    AddressSpace::new(address_width, data_width)
        .address(
            0x0,
            "AP_CONTROL", //TODO: where does this disapear to?
            vec![
                (
                    0..1,
                    "int_ap_start",
                    0..1,
                    Flags::default().write().clear_on_handshake("ap_done"),
                ),
                (
                    1..2,
                    "int_ap_done",
                    0..1,
                    Flags::default()
                        .read("ap_done")
                        .clear_on_read(axi.read_data.clone(), "raddr"),
                ),
                (
                    2..3,
                    "int_ap_idle",
                    0..1,
                    Flags::default().read("ap_done").idle(),
                ),
            ],
        )
        .address(
            0x4,
            "GIE",
            vec![(0..1, "int_gie", 0..1, Flags::default().write())],
        )
        .address(
            0x8,
            "IER",
            vec![(0..2, "int_ier", 0..2, Flags::default().write())],
        )
        .address(
            0xc,
            "ISR",
            vec![
                (0..1, "int_isr_done", 0..1, Flags::default().write()), // XXX should be read
                (1..2, "int_isr_ready", 0..1, Flags::default().write()),
            ],
        )
}

impl ControlInterface for AxiInterface {
    fn control_channels(
        address_width: u64,
        data_width: u64,
        prefix: &str,
    ) -> Self {
        // read channels
        let read_address = AxiChannel {
            prefix: format!("{}AR", prefix),
            direction: ChannelDirection::Recv,
            state: vec![v::Decl::new_wire("raddr", address_width)],
            data_ports: vec![("ADDR".to_string(), address_width)],
        };
        let read_data = AxiChannel {
            prefix: format!("{}R", prefix),
            direction: ChannelDirection::Send,
            state: vec![v::Decl::new_reg("rdata", data_width)],
            data_ports: vec![
                ("DATA".to_string(), data_width),
                ("RESP".to_string(), 2),
            ],
        };

        // write channels
        let write_address = AxiChannel {
            prefix: format!("{}AW", prefix),
            direction: ChannelDirection::Recv,
            state: vec![v::Decl::new_reg("waddr", address_width)],
            data_ports: vec![("ADDR".to_string(), address_width)],
        };
        let write_data = AxiChannel {
            prefix: format!("{}W", prefix),
            direction: ChannelDirection::Recv,
            state: vec![v::Decl::new_wire("wdata", data_width)],
            data_ports: vec![("DATA".to_string(), data_width)],
        };
        let write_response = AxiChannel {
            prefix: format!("{}B", prefix),
            direction: ChannelDirection::Send,
            state: vec![],
            data_ports: vec![("RESP".to_string(), 2)],
        };
        Self {
            read_address,
            read_data,
            write_address,
            write_data,
            write_response,
        }
    }

    fn control_module(
        name: &str,
        address_width: u64,
        data_width: u64,
        memories: &[String],
    ) -> v::Module {
        let mut module = v::Module::new(name);

        module.add_input("ACLK", 1);
        module.add_input("ARESET", 1);

        let axi4 =
            AxiInterface::control_channels(address_width, data_width, "");

        // define the address space of the control interface
        let mut addr_space =
            axi_address_space(&axi4, address_width, data_width);
        addr_space.add_address(
            0x10,
            "TIMEOUT",
            vec![(0..32, "int_timeout", 0..32, Flags::default().write())],
        );
        for (idx, memory_name) in memories.iter().enumerate() {
            let part0_name = format!("{}_0", memory_name);
            let part1_name = format!("{}_1", memory_name);
            let addr_name = format!("addr_{}", memory_name);
            addr_space.add_address(
                0x18 + (idx * 8),
                &part0_name,
                vec![(0..32, &addr_name, 0..32, Flags::default().write())],
            );
            addr_space.add_address(
                0x1c + (idx * 8),
                &part1_name,
                vec![(0..32, &addr_name, 32..64, Flags::default().write())],
            );

            module.add_output(memory_name, 64);
        }

        module.add_output("ap_start", 1);
        module.add_input("ap_done", 1);
        module.add_output("timeout", 32);

        axi4.add_ports_to(&mut module);

        // synchronise channels
        let read_controller =
            axi4.read_address.then(&axi4.read_data).prefix("r");
        read_controller.emit(&mut module);
        module.add_stmt(v::Parallel::Assign("raddr".into(), "ARADDR".into()));
        module.add_stmt(v::Parallel::Assign("RDATA".into(), "rdata".into()));
        module
            .add_stmt(v::Parallel::Assign("RRESP".into(), v::Expr::new_int(0)));

        let write_controller = axi4
            .write_address
            .then(&axi4.write_data)
            .then(&axi4.write_response)
            .prefix("w");
        write_controller.emit(&mut module);
        module.add_stmt(v::Parallel::Assign("wdata".into(), "WDATA".into()));
        module
            .add_stmt(v::Parallel::Assign("BRESP".into(), v::Expr::new_int(0)));
        let mut always = v::ParallelProcess::new_always();
        always.set_event(v::Sequential::new_posedge("ACLK"));
        let mut reset_if = v::SequentialIfElse::new("ARESET");
        reset_if.add_seq(v::Sequential::new_nonblk_assign(
            "waddr",
            v::Expr::new_int(0),
        ));
        let mut waddr_write =
            v::SequentialIfElse::new(axi4.write_address.handshake());
        waddr_write
            .add_seq(v::Sequential::new_nonblk_assign("waddr", "AWADDR"));
        reset_if.set_else(waddr_write);
        always.add_seq(reset_if);
        module.add_stmt(always);

        addr_space.output_to_bus(
            &mut module,
            axi4.read_address.handshake(),
            "raddr",
            "rdata",
        );

        addr_space.internal_registers(&mut module);

        // register logic
        module.add_stmt(v::Parallel::Assign(
            "ap_start".into(),
            "int_ap_start".into(),
        ));
        module.add_stmt(v::Parallel::Assign(
            "timeout".into(),
            "int_timeout".into(),
        ));
        addr_space.register_logic(
            &mut module,
            axi4.write_data.handshake(),
            "AP_CONTROL",
            "waddr",
            "wdata",
        );
        addr_space.register_logic(
            &mut module,
            axi4.write_data.handshake(),
            "GIE",
            "waddr",
            "wdata",
        );
        addr_space.register_logic(
            &mut module,
            axi4.write_data.handshake(),
            "IER",
            "waddr",
            "wdata",
        );
        addr_space.register_logic(
            &mut module,
            axi4.write_data.handshake(),
            "ISR",
            "waddr",
            "wdata",
        );
        addr_space.register_logic(
            &mut module,
            axi4.write_data.handshake(),
            "TIMEOUT",
            "waddr",
            "wdata",
        );

        for memory in memories {
            let part0_name = format!("{}_0", memory);
            let part1_name = format!("{}_1", memory);
            let addr_name = format!("addr_{}", memory);
            module.add_stmt(v::Parallel::Assign(
                memory.as_str().into(),
                addr_name.into(),
            ));
            addr_space.register_logic(
                &mut module,
                axi4.write_data.handshake(),
                &part0_name,
                "waddr",
                "wdata",
            );
            addr_space.register_logic(
                &mut module,
                axi4.write_data.handshake(),
                &part1_name,
                "waddr",
                "wdata",
            );
        }

        module
    }
}
