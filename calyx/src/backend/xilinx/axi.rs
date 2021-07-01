//! Code generation for the AXI interface.
use super::fsm;
use vast::v05::ast as v;

/// Represents the Direction of a channel
/// from the perspective of the master interface.
#[derive(Clone, Debug)]
pub(crate) enum ChannelDirection {
    Recv,
    Send,
}

/// Represents a single AXI channel.
#[derive(Clone, Debug)]
pub(crate) struct AxiChannel {
    /// The string to prefix all ports with.
    pub prefix: String,
    /// The direction of the channel.
    pub direction: ChannelDirection,
    /// Internal stage registers used by this channel.
    pub state: Vec<v::Decl>,
    /// Data ports for this channel.
    pub data_ports: Vec<(String, u64)>,
}

impl AxiChannel {
    /// Generates an expression that is true when this
    /// channel has a successful handshake.
    pub fn handshake(&self) -> v::Expr {
        v::Expr::new_bit_and(self.valid(), self.ready())
    }

    /// Returns the name of signal representing when this channel
    /// is ready to receive data.
    pub fn ready(&self) -> String {
        match self.direction {
            ChannelDirection::Recv => format!("{}READY", self.prefix),
            ChannelDirection::Send => format!("{}VALID", self.prefix),
        }
    }

    /// Returns the name of signal representing when the data sent
    /// by the channel is valid.
    pub fn valid(&self) -> String {
        match self.direction {
            ChannelDirection::Recv => format!("{}VALID", self.prefix),
            ChannelDirection::Send => format!("{}READY", self.prefix),
        }
    }

    /// Synchronize this channel with another channel.
    pub fn then<'a>(&'a self, channel: &'a AxiChannel) -> Synchronization<'a> {
        Synchronization {
            trigger: None,
            channels: vec![self, channel],
            prefix: String::new(),
        }
    }

    /// Return an iterator over all the ports defined in this channel
    /// (including valid/ready).
    pub fn ports(&self) -> impl Iterator<Item = String> + '_ {
        vec![self.ready(), self.valid()].into_iter().chain(
            self.data_ports
                .iter()
                .map(move |(x, _)| format!("{}{}", self.prefix, x)),
        )
    }

    /// Add the ports defined in this channel to the interface
    /// of a `v::Module`
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

    /// Produce an assignment to a particular data port on this channel.
    /// This panics if the channel is a `Recv`.
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

    /// Get the name of a data_port on this channel.
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

/// A complete AxiInterface. Contains two channels for coordinating reads:
///
///  - `read_address`: The channel for coordinating the address to start reading from
///  - `read_data`: The channel for sending the data that is read.
///
/// and three channels for coordinating writes:
///
///  - `write_address`: Channel for coordinating the address to start writing to.
///  - `write_data`: Channel for communicating the data to be written.
///  - `write_response`: Channel for communicating the success of a write.
///
pub(crate) struct AxiInterface {
    pub read_address: AxiChannel,
    pub read_data: AxiChannel,
    pub write_address: AxiChannel,
    pub write_data: AxiChannel,
    pub write_response: AxiChannel,
}

impl AxiInterface {
    /// Add the ports of this AXI interface to a module.
    pub fn add_ports_to(&self, module: &mut v::Module) {
        // add channel ports
        self.read_address.add_ports_to(module);
        self.read_data.add_ports_to(module);
        self.write_address.add_ports_to(module);
        self.write_data.add_ports_to(module);
        self.write_response.add_ports_to(module);
    }

    /// Returns an iterator over all the ports in an AxiInterface.
    pub fn ports(&self) -> impl Iterator<Item = String> + '_ {
        self.read_address
            .ports()
            .chain(self.read_data.ports())
            .chain(self.write_address.ports())
            .chain(self.write_data.ports())
            .chain(self.write_response.ports())
    }
}

/// Represents a synchronization of channels. For example, in Axi,
/// the read address needs to be sent before any data can be sent.
/// This struct represents that relationship.
pub(crate) struct Synchronization<'a> {
    /// An external condition that must be true before
    /// we can start handling AXI transactions.
    trigger: Option<v::Expr>,
    /// The channels that are synchronized.
    channels: Vec<&'a AxiChannel>,
    /// The string prefixed to any internal stage registers generated.
    prefix: String,
}

impl<'a> Synchronization<'a> {
    /// Add `channel` to a synchronization.
    pub fn then(mut self, channel: &'a AxiChannel) -> Self {
        self.channels.push(channel);
        self
    }

    /// Rust builder for setting the prefix of a synchronization.
    pub fn prefix<S>(mut self, prefix: S) -> Self
    where
        S: ToString,
    {
        self.prefix = prefix.to_string();
        self
    }

    /// Rust builder style method for setting the trigger.
    pub fn trigger(mut self, expr: v::Expr) -> Self {
        self.trigger = Some(expr);
        self
    }

    /// Add an fsm implementing this synchronization to
    /// the given module.
    pub fn emit(&self, module: &mut v::Module) {
        // internal channel wires
        for ch in &self.channels {
            ch.state.iter().for_each(|d| module.add_decl(d.clone()));
        }

        // create a new fsm
        let mut fsm = fsm::LinearFsm::new(&self.prefix, "ACLK", "ARESET");

        // if there is a trigger, add a state for it
        if let Some(trigger) = &self.trigger {
            fsm.add_state("trigger", &[], trigger.clone());
        }

        // add a state in the fsm for all the channels
        for ch in &self.channels {
            fsm.add_state(&ch.prefix, &[ch.ready().into()], ch.valid());
        }
        fsm.emit(module);
    }
}
