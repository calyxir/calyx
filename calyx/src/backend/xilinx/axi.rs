use super::fsm;
use vast::v05::ast as v;

pub(crate) enum ChannelDirection {
    Recv,
    Send,
}

pub(crate) struct AxiChannel {
    pub prefix: String,
    pub direction: ChannelDirection,
    pub state: Vec<v::Decl>,
    pub data_ports: Vec<(String, u64)>,
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
        vec![self.ready(), self.valid()].into_iter().chain(
            self.data_ports
                .iter()
                .map(move |(x, _)| format!("{}{}", self.prefix, x)),
        )
    }

    pub fn add_ports_to(&self, module: &mut v::Module) {
        // add valid/ready signals
        module.add_input(&self.valid(), 1);
        module.add_output(&self.ready(), 1);

        match &self.direction {
            ChannelDirection::Recv => {
                for (name, width) in &self.data_ports {
                    module
                        .add_input(&format!("{}{}", self.prefix, name), *width);
                }
            }
            ChannelDirection::Send => {
                for (name, width) in &self.data_ports {
                    module.add_output(
                        &format!("{}{}", self.prefix, name),
                        *width,
                    );
                }
            }
        }
    }

    pub fn assign<S, E>(&self, data_port: S, expr: E) -> v::Stmt
    where
        S: AsRef<str>,
        E: Into<v::Expr>,
    {
        if let ChannelDirection::Send = &self.direction {
            v::Parallel::Assign(self.get(data_port).into(), expr.into()).into()
        } else {
            panic!("Can't assign on a recv channel");
        }
    }

    pub fn get<S>(&self, data_port: S) -> String
    where
        S: AsRef<str>,
    {
        self.data_ports
            .iter()
            .find(|(name, _width)| name == data_port.as_ref())
            .map(|(name, _)| format!("{}{}", self.prefix, name))
            .expect("Data port didn't exist in channel")
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

        let mut fsm = fsm::LinearFsm::new(&self.prefix, "ACLK", "ARESET");

        if let Some(trigger) = &self.trigger {
            fsm.add_state("trigger", &[], trigger.clone());
        }

        for ch in &self.channels {
            fsm.add_state(&ch.prefix, &[ch.ready().into()], ch.valid());
            // if let (0, Some(trigger)) = (i, &self.trigger) {
            //     fsm.add_state(
            //         &ch.prefix,
            //         &[ch.ready().into()],
            //         v::Expr::new_logical_and(ch.valid(), trigger.clone()),
            //     );
            // } else {
            // }
        }
        fsm.emit(module);
    }
}
