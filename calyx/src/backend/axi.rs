use crate::utils;
use std::{collections::BTreeMap, ops::Range};
use vast::v05::ast as v;

/// from the perspective of the master
#[derive(Debug)]
enum Flags {
    Read(String),
    Write,
}

struct Meaning {
    internal_register: String,
    address_range: Range<usize>,
    register_range: Range<usize>,
    flags: Flags,
}

struct Address {
    address: usize,
    name: String,
    bit_meaning: Vec<Meaning>,
}

struct AddressSpace {
    space: Vec<Address>,
    registers: BTreeMap<String, usize>,
    address_width: u64,
    data_width: u64,
}

fn slice(name: &str, width: u64, range: &Range<usize>) -> v::Expr {
    if range.len() == 1 {
        if width == 1 {
            name.into()
        } else {
            v::Expr::new_index_bit(name, range.start as i32)
        }
    } else {
        v::Expr::new_slice(
            name,
            v::Expr::new_int((range.end - 1) as i32),
            v::Expr::new_int(range.start as i32),
        )
    }
}

impl AddressSpace {
    fn new(address_width: u64, data_width: u64) -> Self {
        Self {
            space: Vec::new(),
            registers: BTreeMap::new(),
            address_width,
            data_width,
        }
    }

    fn address(
        mut self,
        address: usize,
        name: &str,
        bit_meaning: Vec<(Range<usize>, &str, Range<usize>, Flags)>,
    ) -> Self {
        self.add_address(address, name, bit_meaning);
        self
    }

    fn add_address(
        &mut self,
        address: usize,
        name: &str,
        bit_meaning: Vec<(Range<usize>, &str, Range<usize>, Flags)>,
    ) {
        let bit_meaning: Vec<Meaning> = bit_meaning
            .into_iter()
            .map(|(r1, name, r2, flags)| Meaning {
                address_range: r1,
                internal_register: name.to_string(),
                register_range: r2,
                flags,
            })
            .collect();

        for meaning in &bit_meaning {
            self.registers
                .entry(meaning.internal_register.to_string())
                .and_modify(|size| *size += meaning.register_range.len())
                .or_insert(meaning.register_range.len());
        }

        self.space.push(Address {
            address,
            name: name.to_string(),
            bit_meaning,
        });
    }

    fn slice(&self, meaning: &Meaning) -> v::Expr {
        slice(
            &meaning.internal_register,
            self.registers[&meaning.internal_register] as u64,
            &meaning.register_range,
        )
    }

    fn internal_registers(&self, module: &mut v::Module) {
        for (register, size) in &self.registers {
            module.add_decl(v::Decl::new_reg(&register, *size as u64));
        }
    }

    /// generate logic for outputting internal registers on the bus
    fn output_to_bus(
        &self,
        module: &mut v::Module,
        handshake: v::Expr,
        address_variable: &str,
        data_variable: &str,
    ) {
        let mut case = v::Case::new(address_variable.into());

        for addr in &self.space {
            let mut branch = v::CaseBranch::new(v::Expr::new_ulit_hex(
                self.address_width as u32,
                &format!("{:02x}", addr.address),
            ));
            for meaning in &addr.bit_meaning {
                branch.add_seq(v::Sequential::new_nonblk_assign(
                    slice(
                        &data_variable,
                        self.data_width as u64,
                        &meaning.address_range,
                    ),
                    self.slice(&meaning),
                ));
            }

            case.add_branch(branch);
        }
        let mut default = v::CaseDefault::default();
        default.add_seq(v::Sequential::new_nonblk_assign(
            data_variable.into(),
            v::Expr::new_int(0),
        ));
        case.set_default(default);

        let mut always = v::ParallelProcess::new_always();
        always.set_event(v::Sequential::new_posedge("ACLK"));

        let mut if_hs = v::SequentialIfElse::new(handshake);
        if_hs.add_seq(v::Sequential::new_case(case));

        always.add_seq(if_hs.into());
        module.add_stmt(always);
    }

    fn register_logic(
        &self,
        module: &mut v::Module,
        handshake: v::Expr,
        name: &str,
        int_addr: &str,
        data: &str,
    ) {
        let addr: &Address =
            self.space.iter().find(|x| x.name == name).unwrap();

        // AXI writes to internal register logic
        let mut always = v::ParallelProcess::new_always();
        always.set_event(v::Sequential::new_posedge("ACLK"));

        let mut if_stmt = v::SequentialIfElse::new("ARESET".into());
        let mut else_br = v::SequentialIfElse::new(v::Expr::new_logical_and(
            handshake.into(),
            v::Expr::new_eq(
                int_addr.into(),
                v::Expr::new_int(addr.address as i32),
            ),
        ));

        // XXX(sam) this is a hack to avoid iterating through the bit meanings again
        let mut writes_exist: bool = false;
        for meaning in addr
            .bit_meaning
            .iter()
            .filter(|m| matches!(m.flags, Flags::Write))
        {
            if_stmt.add_seq(v::Sequential::new_nonblk_assign(
                self.slice(&meaning),
                v::Expr::new_int(0),
            ));
            else_br.add_seq(v::Sequential::new_nonblk_assign(
                self.slice(&meaning),
                slice(data, self.data_width as u64, &meaning.address_range),
            ));
            writes_exist = true;
        }
        if writes_exist {
            if_stmt.set_else(else_br.into());

            always.add_seq(if_stmt.into());
            module.add_stmt(always);
        }

        // port writes to internal register logic
        for meaning in &addr.bit_meaning {
            if let Flags::Read(port) = &meaning.flags {
                let mut always = v::ParallelProcess::new_always();
                always.set_event(v::Sequential::new_posedge("ACLK"));

                let mut if_stmt = v::SequentialIfElse::new("ARESET".into());
                if_stmt.add_seq(v::Sequential::new_nonblk_assign(
                    self.slice(&meaning),
                    v::Expr::new_int(0),
                ));

                let mut else_br =
                    v::SequentialIfElse::new(port.as_str().into());
                else_br.add_seq(v::Sequential::new_nonblk_assign(
                    self.slice(&meaning),
                    v::Expr::new_ulit_bin(1, "1"),
                ));
                if_stmt.set_else(else_br.into());

                always.add_seq(if_stmt.into());
                module.add_stmt(always);
            }
        }
    }

    #[allow(unused)]
    fn print_mapping(&self) {
        for addr in &self.space {
            println!("{:#04x} {}", addr.address, addr.name);
            for meaning in &addr.bit_meaning {
                let slice = if meaning.address_range.len() == 1 {
                    format!("{}", meaning.address_range.start)
                } else {
                    format!(
                        "[{}:{}]",
                        meaning.address_range.end - 1,
                        meaning.address_range.start
                    )
                };
                let var_slice = if meaning.register_range.len() == 1 {
                    String::new()
                } else {
                    format!(
                        "[{}:{}]",
                        meaning.register_range.end - 1,
                        meaning.register_range.start
                    )
                };
                println!(
                    "     {} ({}{} {:?})",
                    slice, meaning.internal_register, var_slice, meaning.flags
                );
            }
        }
    }
}

enum ChannelDirection {
    Recv,
    Send,
}

struct AxiChannel {
    prefix: String,
    direction: ChannelDirection,
    state: Vec<v::Decl>,
    inputs: Vec<(String, u64)>,
    /// Vector of (PortName, expression)
    outputs: Vec<(String, u64)>,
}

impl AxiChannel {
    fn handshake(&self) -> v::Expr {
        v::Expr::new_bit_and(self.valid(), self.ready())
    }

    fn ready(&self) -> String {
        match self.direction {
            ChannelDirection::Recv => format!("{}READY", self.prefix),
            ChannelDirection::Send => format!("{}VALID", self.prefix),
        }
    }

    fn valid(&self) -> String {
        match self.direction {
            ChannelDirection::Recv => format!("{}VALID", self.prefix),
            ChannelDirection::Send => format!("{}READY", self.prefix),
        }
    }

    fn then<'a>(&'a self, channel: &'a AxiChannel) -> Synchronization<'a> {
        Synchronization {
            channels: vec![self, channel],
            prefix: String::new(),
        }
    }

    fn add_ports_to(&self, module: &mut v::Module) {
        // add valid/ready signals
        module.add_input(&self.valid(), 1);
        module.add_output(&self.ready(), 1);

        for (name, width) in &self.inputs {
            module.add_input(&format!("{}{}", self.prefix, name), *width);
        }

        for (name, width) in &self.outputs {
            module.add_output(&format!("{}{}", self.prefix, name), *width);
        }
    }
}

pub(crate) struct Axi4Lite {
    read_address: AxiChannel,
    read_data: AxiChannel,
    write_address: AxiChannel,
    write_data: AxiChannel,
    write_response: AxiChannel,
}

impl Axi4Lite {
    pub fn new(address_width: u64, data_width: u64, prefix: &str) -> Self {
        // read channels
        let read_address = AxiChannel {
            prefix: format!("{}AR", prefix),
            direction: ChannelDirection::Recv,
            state: vec![v::Decl::new_wire("raddr", address_width)],
            inputs: vec![("ADDR".to_string(), address_width)],
            outputs: vec![],
        };
        let read_data = AxiChannel {
            prefix: format!("{}R", prefix),
            direction: ChannelDirection::Send,
            state: vec![v::Decl::new_reg("rdata", data_width)],
            inputs: vec![],
            outputs: vec![
                ("DATA".to_string(), data_width),
                ("RESP".to_string(), 2),
            ],
        };

        // write channels
        let write_address = AxiChannel {
            prefix: format!("{}AW", prefix),
            direction: ChannelDirection::Recv,
            state: vec![v::Decl::new_reg("waddr", address_width)],
            inputs: vec![("ADDR".to_string(), address_width)],
            outputs: vec![],
        };
        let write_data = AxiChannel {
            prefix: format!("{}W", prefix),
            direction: ChannelDirection::Recv,
            state: vec![v::Decl::new_wire("wdata", data_width)],
            inputs: vec![("DATA".to_string(), data_width)],
            outputs: vec![],
        };
        let write_response = AxiChannel {
            prefix: format!("{}B", prefix),
            direction: ChannelDirection::Send,
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

    pub fn add_ports_to(&self, module: &mut v::Module) {
        // add channel ports
        self.read_address.add_ports_to(module);
        self.read_data.add_ports_to(module);
        self.write_address.add_ports_to(module);
        self.write_data.add_ports_to(module);
        self.write_response.add_ports_to(module);
    }
}

struct Synchronization<'a> {
    channels: Vec<&'a AxiChannel>,
    prefix: String,
}

impl<'a> Synchronization<'a> {
    fn then(mut self, channel: &'a AxiChannel) -> Self {
        self.channels.push(channel);
        self
    }

    fn prefix<S>(mut self, prefix: S) -> Self
    where
        S: ToString,
    {
        self.prefix = prefix.to_string();
        self
    }

    fn state(&self) -> String {
        format!("{}state", self.prefix)
    }

    fn next(&self) -> String {
        format!("{}next", self.prefix)
    }

    fn decls(&self, module: &mut v::Module) {
        // fsm state registers
        let state_width =
            utils::math::bits_needed_for(self.channels.len() as u64);
        module.add_decl(v::Decl::new_reg(&self.state(), state_width));
        module.add_decl(v::Decl::new_reg(&self.next(), state_width));

        // internal channel wires
        for ch in &self.channels {
            ch.state.iter().for_each(|d| module.add_decl(d.clone()));
        }
    }

    fn state_assignments(&self, module: &mut v::Module) {
        for (i, ch) in self.channels.iter().enumerate() {
            let assign = v::Parallel::Assign(
                ch.ready().into(),
                v::Expr::new_eq(
                    self.state().into(),
                    v::Expr::new_int(i as i32),
                ),
            );
            module.add_stmt(assign);
        }
    }

    fn update(&self, module: &mut v::Module) {
        let mut parallel = v::ParallelProcess::new_always();
        parallel.set_event(v::Sequential::new_posedge("ACLK"));

        let mut ifelse = v::SequentialIfElse::new("ARESET".into());
        ifelse.add_seq(v::Sequential::new_nonblk_assign(
            self.state().into(),
            v::Expr::new_int(0),
        ));
        ifelse.set_else(v::Sequential::new_nonblk_assign(
            self.state().into(),
            self.next().into(),
        ));

        parallel.add_seq(ifelse.into());
        module.add_stmt(parallel)
    }

    fn transition_block(&self, module: &mut v::Module) {
        let mut parallel = v::ParallelProcess::new_always();
        parallel.set_event(v::Sequential::Wildcard);

        let mut case = v::Case::new(self.state().into());

        for (i, ch) in self.channels.iter().enumerate() {
            let this_state = i as i32;
            let next_state = ((i + 1) % self.channels.len()) as i32;

            let mut branch = v::CaseBranch::new(v::Expr::new_int(this_state));
            let mut ifelse =
                v::SequentialIfElse::new(v::Expr::new_ref(ch.valid()));
            ifelse.add_seq(v::Sequential::new_blk_assign(
                self.next().into(),
                v::Expr::new_int(next_state),
            ));
            ifelse.set_else(v::Sequential::new_blk_assign(
                self.next().into(),
                v::Expr::new_int(this_state),
            ));
            branch.add_seq(ifelse.into());
            case.add_branch(branch);
        }

        let mut default = v::CaseDefault::default();
        default.add_seq(v::Sequential::new_blk_assign(
            self.next().into(),
            v::Expr::new_int(0),
        ));
        case.set_default(default);

        parallel.add_seq(v::Sequential::new_case(case));
        module.add_stmt(parallel)
    }

    fn emit(&self, module: &mut v::Module) {
        self.decls(module);
        self.state_assignments(module);
        self.update(module);
        self.transition_block(module);
    }
}

fn axi_address_space(address_width: u64, data_width: u64) -> AddressSpace {
    AddressSpace::new(address_width, data_width)
        .address(
            0x0,
            "AP_CONTROL",
            vec![
                (0..1, "int_ap_start", 0..1, Flags::Write),
                (
                    1..2,
                    "int_ap_done",
                    0..1,
                    Flags::Read("ap_done".to_string()),
                ),
                // (2..3, "ap_idle", 0..1),,
            ],
        )
        .address(0x4, "GIE", vec![(0..1, "int_gie", 0..1, Flags::Write)])
        .address(0x8, "IER", vec![(0..2, "int_ier", 0..2, Flags::Write)])
        .address(
            0xc,
            "ISR",
            vec![
                (0..1, "int_isr_done", 0..1, Flags::Write), // XXX should be read
                (1..2, "int_isr_ready", 0..1, Flags::Write),
            ],
        )
}

pub fn axi(
    module: &mut v::Module,
    address_width: u64,
    data_width: u64,
    memories: &[String],
) {
    module.add_input("ACLK", 1);
    module.add_input("ARESET", 1);

    // define the address space of the control interface
    let mut addr_space = axi_address_space(address_width, data_width);
    addr_space.add_address(
        0x10,
        "TIMEOUT",
        vec![(0..32, "int_timeout", 0..32, Flags::Write)],
    );
    for (idx, memory_name) in memories.iter().enumerate() {
        let part0_name = format!("{}_0", memory_name);
        let part1_name = format!("{}_1", memory_name);
        let addr_name = format!("addr_{}", memory_name);
        addr_space.add_address(
            0x18 + (idx * 8),
            &part0_name,
            vec![(0..32, &addr_name, 0..32, Flags::Write)],
        );
        addr_space.add_address(
            0x1c + (idx * 8),
            &part1_name,
            vec![(0..32, &addr_name, 32..64, Flags::Write)],
        );

        module.add_output(memory_name, 64);
    }

    module.add_output("ap_start", 1);
    module.add_input("ap_done", 1);
    module.add_output("timeout", 32);

    let axi4 = Axi4Lite::new(address_width, data_width, "");
    axi4.add_ports_to(module);

    // synchronise channels
    let read_controller = axi4.read_address.then(&axi4.read_data).prefix("r");
    read_controller.emit(module);
    module.add_stmt(v::Parallel::Assign("raddr".into(), "ARADDR".into()));
    module.add_stmt(v::Parallel::Assign("RDATA".into(), "rdata".into()));
    module.add_stmt(v::Parallel::Assign("RRESP".into(), v::Expr::new_int(0)));

    let write_controller = axi4
        .write_address
        .then(&axi4.write_data)
        .then(&axi4.write_response)
        .prefix("w");
    write_controller.emit(module);
    module.add_stmt(v::Parallel::Assign("wdata".into(), "WDATA".into()));
    module.add_stmt(v::Parallel::Assign("BRESP".into(), v::Expr::new_int(0)));
    let mut always = v::ParallelProcess::new_always();
    always.set_event(v::Sequential::new_posedge("ACLK"));
    let mut ifelse = v::SequentialIfElse::new(axi4.write_address.handshake());
    ifelse.add_seq(v::Sequential::new_nonblk_assign(
        "waddr".into(),
        "AWADDR".into(),
    ));
    always.add_seq(ifelse.into());
    module.add_stmt(always);

    // addr_space.print_mapping();
    // println!("====\n");
    addr_space.output_to_bus(
        module,
        axi4.read_data.handshake(),
        "raddr",
        "rdata",
    );

    addr_space.internal_registers(module);

    // register logic
    module.add_stmt(v::Parallel::Assign(
        "ap_start".into(),
        "int_ap_start".into(),
    ));
    module
        .add_stmt(v::Parallel::Assign("timeout".into(), "int_timeout".into()));
    addr_space.register_logic(
        module,
        axi4.write_data.handshake(),
        "AP_CONTROL",
        "waddr",
        "wdata",
    );
    addr_space.register_logic(
        module,
        axi4.write_data.handshake(),
        "GIE",
        "waddr",
        "wdata",
    );
    addr_space.register_logic(
        module,
        axi4.write_data.handshake(),
        "IER",
        "waddr",
        "wdata",
    );
    addr_space.register_logic(
        module,
        axi4.write_data.handshake(),
        "ISR",
        "waddr",
        "wdata",
    );
    addr_space.register_logic(
        module,
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
            module,
            axi4.write_data.handshake(),
            &part0_name,
            "waddr",
            "wdata",
        );
        addr_space.register_logic(
            module,
            axi4.write_data.handshake(),
            &part1_name,
            "waddr",
            "wdata",
        );
    }
}
