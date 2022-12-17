use super::axi::AxiChannel;
use std::{collections::BTreeMap, ops::Range};
use vast::v05::ast as v;

/// Stores flags for different characteristics
/// of mmemory mapped registers.
#[derive(Debug, Default)]
pub(crate) struct Flags {
    /// This address is available to be read in the interface.
    /// The string holds the name of the internal register.
    read: Option<String>,
    /// Clear the value of the internal register when the given
    /// channel reads.
    clear_on_read: Option<(AxiChannel, String)>,
    /// Clear the internal register when there is a successful
    /// handshake on this channel.
    // XXX(nathanielnrn): might not be a good name? used for things that aren't just
    // handshakes
    clear_on_handshake: Option<String>,
    /// Clear the internal register when ap_start is asserted
    /// and invert `ARESET` logic (idle is high when reset)
    idle: bool,
    /// This register can be written to with the interface.
    write: bool,
}

impl Flags {
    /// Builder style function that sets the `read` flag.
    pub(crate) fn read<S>(mut self, name: S) -> Self
    where
        S: ToString,
    {
        self.read = Some(name.to_string());
        self
    }

    /// Builder style function for setting the `clear_on_read` flag.
    pub(crate) fn clear_on_read<S>(
        mut self,
        axi_channel: AxiChannel,
        int_addr: S,
    ) -> Self
    where
        S: ToString,
    {
        self.clear_on_read = Some((axi_channel, int_addr.to_string()));
        self
    }

    /// Builder style function for setting the `clear_on_handshake` flag.
    pub(crate) fn clear_on_handshake<S>(mut self, name: S) -> Self
    where
        S: ToString,
    {
        self.clear_on_handshake = Some(name.to_string());
        self
    }

    /// Builder style function for setting the `idle` flag.
    pub(crate) fn idle(mut self) -> Self {
        self.idle = true;
        self
    }
    /// Builder style function for setting the `write` flag.
    pub(crate) fn write(mut self) -> Self {
        self.write = true;
        self
    }
}

/// Stores what a range of bits mean for an AXI address.
pub(crate) struct Meaning {
    /// The name of the internal register that stores
    /// this part of the address space.
    internal_register: String,
    /// The range of the address that holds this meaning.
    address_range: Range<usize>,
    /// The range of the internal register this address maps to.
    register_range: Range<usize>,
    /// Flags describing how this register should be used.
    flags: Flags,
}

/// Stores the meanings for a particular address.
pub(crate) struct Address {
    address: usize,
    name: String,
    bit_meaning: Vec<Meaning>,
}

/// Stores a space of addresses.
pub(crate) struct AddressSpace {
    space: Vec<Address>,
    registers: BTreeMap<String, usize>,
    address_width: u64,
    data_width: u64,
}

/// Helper for generating slice expressions.
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
    /// Create an address space with a particular address width and data width.
    pub fn new(address_width: u64, data_width: u64) -> Self {
        Self {
            space: Vec::new(),
            registers: BTreeMap::new(),
            address_width,
            data_width,
        }
    }

    /// Builder style method for adding an address.
    pub fn address(
        mut self,
        address: usize,
        name: &str,
        bit_meaning: Vec<(Range<usize>, &str, Range<usize>, Flags)>,
    ) -> Self {
        self.add_address(address, name, bit_meaning);
        self
    }

    /// Add an address called `name` with meanings: `bit_meaning`.
    /// For example,
    /// ```
    /// space.address(
    ///   0x0,
    ///   "CTRL",
    ///   vec![
    ///     (0..1, "start", 0..1, Flags::Write)
    ///     (1..2, "done", 0..1, Flags::Write)
    ///   ]
    /// )
    /// ```
    /// adds an address called `CTRL` that maps the lowest bit (0) of `0x0` to
    /// a one bit register called `start` and maps bit 1 of `0x0` to a one bit register
    /// called `done`.
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
            *self
                .registers
                .entry(meaning.internal_register.to_string())
                .or_default() += meaning.register_range.len();
        }

        self.space.push(Address {
            address,
            name: name.to_string(),
            bit_meaning,
        });
    }

    /// Helper to create slices on meanings.
    fn slice(&self, meaning: &Meaning) -> v::Expr {
        slice(
            &meaning.internal_register,
            self.registers[&meaning.internal_register] as u64,
            &meaning.register_range,
        )
    }

    /// Add declarations for internal registers.
    pub fn internal_registers(&self, module: &mut v::Module) {
        for (register, size) in &self.registers {
            module.add_decl(v::Decl::new_reg(register, *size as u64));
        }
    }

    /// Generate logic for outputting internal registers on the bus
    pub fn output_to_bus(
        &self,
        module: &mut v::Module,
        handshake: v::Expr,
        address_variable: &str,
        data_variable: &str,
    ) {
        let mut case = v::Case::new(address_variable);

        for addr in &self.space {
            let mut branch = v::CaseBranch::new(v::Expr::new_ulit_hex(
                self.address_width as u32,
                &format!("{:02x}", addr.address),
            ));
            let mut end = 0;
            for meaning in &addr.bit_meaning {
                branch.add_seq(v::Sequential::new_nonblk_assign(
                    slice(
                        data_variable,
                        self.data_width,
                        &meaning.address_range,
                    ),
                    self.slice(meaning),
                ));
                end = meaning.address_range.end;
            }

            if end < 32 {
                branch.add_seq(v::Sequential::new_nonblk_assign(
                    slice(data_variable, self.data_width, &(end..32)),
                    v::Expr::new_int(0),
                ));
            }

            case.add_branch(branch);
        }
        let mut default = v::CaseDefault::default();
        default.add_seq(v::Sequential::new_nonblk_assign(data_variable, 0));
        case.set_default(default);

        let mut always = v::ParallelProcess::new_always();
        always.set_event(v::Sequential::new_posedge("ACLK"));

        let mut reset_if = v::SequentialIfElse::new("ARESET");
        reset_if.add_seq(v::Sequential::new_nonblk_assign(data_variable, 0));

        let mut if_hs = v::SequentialIfElse::new(handshake);
        if_hs.add_seq(v::Sequential::new_case(case));

        reset_if.set_else(if_hs);

        always.add_seq(reset_if);
        module.add_stmt(always);
    }

    /// Generate logic for writing / reading from internal registers
    /// holding state.
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

        let mut if_stmt = v::SequentialIfElse::new("ARESET");
        let mut else_br = v::SequentialIfElse::new(v::Expr::new_logical_and(
            handshake,
            v::Expr::new_eq(int_addr, addr.address as i32),
        ));

        // XXX(sam) this is a hack to avoid iterating through the bit meanings again
        // only happens for writes
        let mut writes_exist: bool = false;
        for meaning in addr.bit_meaning.iter().filter(|m| m.flags.write) {
            // this part is only the assignment itself?? underneath the ARESET of the if branch
            if_stmt.add_seq(v::Sequential::new_nonblk_assign(
                self.slice(meaning), //gets name of register
                v::Expr::new_int(0),
            ));
            else_br.add_seq(v::Sequential::new_nonblk_assign(
                self.slice(meaning),
                slice(data, self.data_width, &meaning.address_range),
            ));
            if let Some(name) = &meaning.flags.clear_on_handshake {
                let mut clear_if = v::SequentialIfElse::new(name.as_str());
                clear_if.add_seq(v::Sequential::new_nonblk_assign(
                    self.slice(meaning),
                    0,
                ));
                else_br.set_else(clear_if);
            }
            writes_exist = true;
        }
        if writes_exist {
            if_stmt.set_else(else_br);

            always.add_seq(if_stmt);
            module.add_stmt(always);
        }

        // port writes to internal register logic
        // reads only
        // XXX(nathanielnrn) Why does this need to be seperate from the above write logic?
        // Seems basically the same to me?
        for meaning in &addr.bit_meaning {
            if let Some(port) = &meaning.flags.read {
                //Takes in read string of Read option and assigns it to `port`
                let mut branches = vec![
                    (Some("ARESET".into()), 0.into()),
                    (Some(port.as_str().into()), 1.into()),
                ];
                if let Some((channel, addr_reg)) = &meaning.flags.clear_on_read
                {
                    let cond = v::Expr::new_logical_and(
                        channel.handshake(),
                        v::Expr::new_eq(addr_reg.as_str(), addr.address as i32),
                    );
                    branches.push((Some(cond), 0.into()));
                }
                if meaning.flags.idle {
                    branches[0] = (Some("ARESET".into()), 1.into());
                    let if_ap_start = v::Expr::new_ref("ap_start");
                    branches.push((Some(if_ap_start), 0.into()));
                }
                let always = super::utils::cond_non_blk_assign(
                    "ACLK",
                    self.slice(meaning),
                    branches,
                );
                module.add_stmt(always);
            }
        }
    }

    /// Human readable representation of the address space for debugging.
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
