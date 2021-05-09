use std::{collections::BTreeMap, ops::Range};
use vast::v05::ast as v;

/// from the perspective of the master
#[derive(Debug)]
pub(crate) enum Flags {
    Read(String),
    Write,
}

pub(crate) struct Meaning {
    internal_register: String,
    address_range: Range<usize>,
    register_range: Range<usize>,
    flags: Flags,
}

pub(crate) struct Address {
    address: usize,
    name: String,
    bit_meaning: Vec<Meaning>,
}

pub(crate) struct AddressSpace {
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
    pub fn new(address_width: u64, data_width: u64) -> Self {
        Self {
            space: Vec::new(),
            registers: BTreeMap::new(),
            address_width,
            data_width,
        }
    }

    pub fn address(
        mut self,
        address: usize,
        name: &str,
        bit_meaning: Vec<(Range<usize>, &str, Range<usize>, Flags)>,
    ) -> Self {
        self.add_address(address, name, bit_meaning);
        self
    }

    pub fn add_address(
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

    pub fn internal_registers(&self, module: &mut v::Module) {
        for (register, size) in &self.registers {
            module.add_decl(v::Decl::new_reg(&register, *size as u64));
        }
    }

    /// generate logic for outputting internal registers on the bus
    pub fn output_to_bus(
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
            let mut end = 0;
            for meaning in &addr.bit_meaning {
                branch.add_seq(v::Sequential::new_nonblk_assign(
                    slice(
                        &data_variable,
                        self.data_width as u64,
                        &meaning.address_range,
                    ),
                    self.slice(&meaning),
                ));
                end = meaning.address_range.end;
            }

            if end < 32 {
                branch.add_seq(v::Sequential::new_nonblk_assign(
                    slice(&data_variable, self.data_width as u64, &(end..32)),
                    v::Expr::new_int(0),
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

        let mut reset_if = v::SequentialIfElse::new("ARESET".into());
        reset_if.add_seq(v::Sequential::new_nonblk_assign(
            data_variable.into(),
            v::Expr::new_int(0),
        ));

        let mut if_hs = v::SequentialIfElse::new(handshake);
        if_hs.add_seq(v::Sequential::new_case(case));

        reset_if.set_else(if_hs.into());

        always.add_seq(reset_if.into());
        module.add_stmt(always);
    }

    pub fn register_logic(
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
            handshake,
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
    pub fn print_mapping(&self) {
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
