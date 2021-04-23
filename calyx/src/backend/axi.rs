use itertools::Itertools;
use std::ops::Range;
use vast::v05::ast as v;

struct Address {
    name: String,
    width: usize,
    bit_meaning: Vec<(Range<usize>, String)>,
}

#[derive(Default)]
struct AddressSpace {
    space: Vec<Address>,
}

impl AddressSpace {
    fn address(
        mut self,
        name: &str,
        bit_meaning: Vec<(Range<usize>, &str)>,
    ) -> Self {
        self.add_address(name, bit_meaning);
        self
    }

    fn add_address(
        &mut self,
        name: &str,
        bit_meaning: Vec<(Range<usize>, &str)>,
    ) {
        let mut max: usize = 0;
        // check to make sure that ranges are increasing and disjoint
        if bit_meaning.len() >= 2 {
            for ((r1, _), (r2, _)) in bit_meaning.iter().tuple_windows() {
                if r1.end > r2.start {
                    panic!("{:?}, {:?}", r1, r2);
                }
                max = r2.end;
            }
        } else {
            max = bit_meaning[0].0.end;
        }

        // round max up to multiple of 32
        let width = (max + 32 - 1) - ((max + 32 - 1) % 32);

        self.space.push(Address {
            name: name.to_string(),
            width,
            bit_meaning: bit_meaning
                .into_iter()
                .map(|(r, name)| (r, name.to_string()))
                .collect(),
        });
    }

    fn read(&self, address_variable: &str, data_variable: &str) {
        let mut case = v::Case::new(address_variable.into());
        let mut mapping: usize = 0;

        for addr in &self.space {
            let mut branch = v::CaseBranch::new(v::Expr::new_ulit_hex(
                4,
                &format!("{:02x}", mapping),
            ));
            for (bit_range, name) in &addr.bit_meaning {
                if bit_range.len() == 1 {
                    branch.add_seq(v::Sequential::new_nonblk_assign(
                        v::Expr::new_index_bit(
                            data_variable,
                            bit_range.start as i32,
                        ),
                        v::Expr::new_ref(name),
                    ));
                } else {
                    branch.add_seq(v::Sequential::new_nonblk_assign(
                        v::Expr::new_slice(
                            data_variable,
                            v::Expr::new_int((bit_range.end - 1) as i32),
                            v::Expr::new_int(bit_range.start as i32),
                        ),
                        v::Expr::new_ref(name),
                    ));
                }
            }

            case.add_branch(branch);

            mapping += addr.width / 8;
        }

        // for (i, ch) in self.channels.iter().enumerate() {
        //     let this_state = i as i32;
        //     let next_state = ((i + 1) % self.channels.len()) as i32;

        //     let mut branch = v::CaseBranch::new(v::Expr::new_int(this_state));
        //     let mut ifelse =
        //         v::SequentialIfElse::new(v::Expr::new_ref(ch.valid()));
        //     ifelse.add_seq(v::Sequential::new_blk_assign(
        //         "next".into(),
        //         v::Expr::new_int(next_state),
        //     ));
        //     ifelse.set_else(v::Sequential::new_blk_assign(
        //         "next".into(),
        //         v::Expr::new_int(this_state),
        //     ));
        //     branch.add_seq(ifelse.into());
        //     case.add_branch(branch);
        // }

        // let mut default = v::CaseDefault::default();
        // default.add_seq(v::Sequential::new_blk_assign(
        //     "state".into(),
        //     v::Expr::new_int(0),
        // ));
        // case.set_default(default);

        println!("{}", v::Sequential::new_case(case));
    }

    fn print_mapping(&self) {
        let mut mapping: usize = 0;

        for addr in &self.space {
            println!("{:#04x} {}: {}", mapping, addr.name, addr.width);
            for (range, name) in &addr.bit_meaning {
                let slice = if range.len() == 1 {
                    format!("{}", range.start)
                } else {
                    format!("[{}:{}]", range.end - 1, range.start)
                };
                println!("     {} ({})", slice, name);
            }

            mapping += addr.width / 8;
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
    inputs: Vec<String>,
    outputs: Vec<(String, String)>,
}

impl AxiChannel {
    fn handshake(&self) -> String {
        format!("{}valid & {}ready", self.prefix, self.prefix)
    }

    fn ready(&self) -> String {
        match self.direction {
            ChannelDirection::Recv => format!("{}ready", self.prefix),
            ChannelDirection::Send => format!("{}valid", self.prefix),
        }
    }

    fn valid(&self) -> String {
        match self.direction {
            ChannelDirection::Recv => format!("{}valid", self.prefix),
            ChannelDirection::Send => format!("{}ready", self.prefix),
        }
    }

    fn then<'a>(&'a self, channel: &'a AxiChannel) -> Synchronization<'a> {
        Synchronization {
            channels: vec![self, channel],
        }
    }
}

struct Synchronization<'a> {
    channels: Vec<&'a AxiChannel>,
}

impl<'a> Synchronization<'a> {
    fn then(mut self, channel: &'a AxiChannel) -> Self {
        self.channels.push(channel);
        self
    }

    fn transition_block(&self) -> v::Stmt {
        let mut parallel = v::ParallelProcess::new_always();
        parallel.set_event(v::Sequential::Wildcard);

        let mut case = v::Case::new("state".into());

        for (i, ch) in self.channels.iter().enumerate() {
            let this_state = i as i32;
            let next_state = ((i + 1) % self.channels.len()) as i32;

            let mut branch = v::CaseBranch::new(v::Expr::new_int(this_state));
            let mut ifelse =
                v::SequentialIfElse::new(v::Expr::new_ref(ch.valid()));
            ifelse.add_seq(v::Sequential::new_blk_assign(
                "next".into(),
                v::Expr::new_int(next_state),
            ));
            ifelse.set_else(v::Sequential::new_blk_assign(
                "next".into(),
                v::Expr::new_int(this_state),
            ));
            branch.add_seq(ifelse.into());
            case.add_branch(branch);
        }

        let mut default = v::CaseDefault::default();
        default.add_seq(v::Sequential::new_blk_assign(
            "state".into(),
            v::Expr::new_int(0),
        ));
        case.set_default(default);

        parallel.add_seq(v::Sequential::new_case(case));
        parallel.into()
    }
}

pub fn axi() {
    let addr_space = AddressSpace::default()
        .address(
            "AP_CONTROL",
            vec![(0..1, "ap_start"), (1..2, "ap_done"), (2..3, "ap_idle")],
        )
        .address("GIE", vec![(0..1, "gie")])
        .address("IER", vec![(0..1, "ier_done"), (1..2, "ier_ready")])
        .address("ISR", vec![(0..1, "isr_done"), (1..2, "isr_ready")])
        .address("TIMEOUT", vec![(0..32, "timeout"), (33..64, "reserved")])
        .address("A", vec![(0..64, "addr_A")])
        .address("B", vec![(0..64, "addr_B")]);

    addr_space.print_mapping();
    addr_space.read("raddr", "rdata");

    // write channels
    let write_address = AxiChannel {
        prefix: "aw".to_string(),
        direction: ChannelDirection::Recv,
        inputs: vec!["addr".to_string()],
        outputs: vec![],
    };
    let write_data = AxiChannel {
        prefix: "w".to_string(),
        direction: ChannelDirection::Recv,
        inputs: vec!["data".to_string(), "strb".to_string()],
        outputs: vec![],
    };
    let write_response = AxiChannel {
        prefix: "b".to_string(),
        direction: ChannelDirection::Send,
        inputs: vec![],
        outputs: vec![("resp".to_string(), "0".to_string())],
    };

    let sync = write_address.then(&write_data).then(&write_response);

    //

    // write_address.on_handshake(/* waddr <= AWADDR[5:0]; */);
    // write_data.on_handshake();

    // if write_address.handshake()

    // read channels

    // println!("{}", sync.transition_block());
}

// struct AxiChannelWriteAddress {
//     valid: String,
//     ready: String,
//     addr: String,
// }

// struct AxiChannelWriteData {
//     valid: String,
//     ready: String,
//     address: String,
//     strobe: String,
// }

// struct AxiChannelWriteRespose {
//     valid: String,
//     ready: String,
//     resp: String,
// }

// struct AxiChannelReadAddress {
//     valid: String,
//     ready: String,
//     address: String,
// }

// struct AxiChannelReadData {
//     valid: String,
//     ready: String,
//     data: String,
//     resp: String,
// }
