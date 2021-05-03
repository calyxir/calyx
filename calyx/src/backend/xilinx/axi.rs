use crate::utils;
use vast::v05::ast as v;

pub(crate) enum ChannelDirection {
    Recv,
    Send,
}

pub(crate) struct AxiChannel {
    pub prefix: String,
    pub direction: ChannelDirection,
    pub state: Vec<v::Decl>,
    pub inputs: Vec<(String, u64)>,
    /// Vector of (PortName, expression)
    pub outputs: Vec<(String, u64)>,
}

impl AxiChannel {
    pub fn handshake(&self) -> v::Expr {
        v::Expr::new_bit_and(self.valid(), self.ready())
    }

    pub fn ready(&self) -> String {
        match self.direction {
            ChannelDirection::Recv => format!("{}READY", self.prefix),
            ChannelDirection::Send => format!("{}VALID", self.prefix),
        }
    }

    pub fn valid(&self) -> String {
        match self.direction {
            ChannelDirection::Recv => format!("{}VALID", self.prefix),
            ChannelDirection::Send => format!("{}READY", self.prefix),
        }
    }

    pub fn then<'a>(&'a self, channel: &'a AxiChannel) -> Synchronization<'a> {
        Synchronization {
            channels: vec![self, channel],
            prefix: String::new(),
        }
    }

    pub fn ports<'a>(&'a self) -> impl Iterator<Item = String> + 'a {
        vec![self.ready(), self.valid()]
            .into_iter()
            .chain(
                self.inputs
                    .iter()
                    .map(move |(x, _)| format!("{}{}", self.prefix, x)),
            )
            .chain(
                self.outputs
                    .iter()
                    .map(move |(x, _)| format!("{}{}", self.prefix, x)),
            )
    }

    pub fn add_ports_to(&self, module: &mut v::Module) {
        // add valid/ready signals
        module.add_input(&self.valid(), 1);
        module.add_output(&self.ready(), 1);

        let mod_inputs = match &self.direction {
            ChannelDirection::Recv => &self.inputs,
            ChannelDirection::Send => &self.outputs,
        };
        let mod_outputs = match &self.direction {
            ChannelDirection::Recv => &self.outputs,
            ChannelDirection::Send => &self.inputs,
        };

        for (name, width) in mod_inputs {
            module.add_input(&format!("{}{}", self.prefix, name), *width);
        }

        for (name, width) in mod_outputs {
            module.add_output(&format!("{}{}", self.prefix, name), *width);
        }
    }
}

pub(crate) struct Axi4Lite {
    pub read_address: AxiChannel,
    pub read_data: AxiChannel,
    pub write_address: AxiChannel,
    pub write_data: AxiChannel,
    pub write_response: AxiChannel,
}

impl Axi4Lite {
    pub fn add_ports_to(&self, module: &mut v::Module) {
        // add channel ports
        self.read_address.add_ports_to(module);
        self.read_data.add_ports_to(module);
        self.write_address.add_ports_to(module);
        self.write_data.add_ports_to(module);
        self.write_response.add_ports_to(module);
    }

    pub fn ports<'a>(&'a self) -> impl Iterator<Item = String> + 'a {
        self.read_address
            .ports()
            .chain(self.read_data.ports())
            .chain(self.write_address.ports())
            .chain(self.write_data.ports())
            .chain(self.write_response.ports())
    }
}

pub(crate) struct Synchronization<'a> {
    channels: Vec<&'a AxiChannel>,
    prefix: String,
}

impl<'a> Synchronization<'a> {
    pub fn then(mut self, channel: &'a AxiChannel) -> Self {
        self.channels.push(channel);
        self
    }

    pub fn prefix<S>(mut self, prefix: S) -> Self
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

    pub fn emit(&self, module: &mut v::Module) {
        self.decls(module);
        self.state_assignments(module);
        self.update(module);
        self.transition_block(module);
    }
}
