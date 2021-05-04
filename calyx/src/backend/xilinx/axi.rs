use super::fsm;
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
            trigger: None,
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
    trigger: Option<v::Expr>,
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

    pub fn trigger(mut self, expr: v::Expr) -> Self {
        self.trigger = Some(expr);
        self
    }

    pub fn emit(&self, module: &mut v::Module) {
        // internal channel wires
        for ch in &self.channels {
            ch.state.iter().for_each(|d| module.add_decl(d.clone()));
        }

        let mut fsm = fsm::LinearFsm::new(&self.prefix);
        for ch in &self.channels {
            fsm.add_state(&ch.prefix, &[ch.ready().into()], ch.valid());
        }
        fsm.emit(module);
    }
}
