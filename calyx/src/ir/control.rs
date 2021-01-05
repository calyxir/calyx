use super::{Attributes, Cell, Group, Id, Port, RRC};

/// Data for the `seq` control statement.
#[derive(Debug)]
pub struct Seq {
    /// List of `Control` statements to run in sequence.
    pub stmts: Vec<Control>,
    /// Attributes attached to this control statement.
    pub attributes: Attributes,
}

/// Data for the `par` control statement.
#[derive(Debug)]
pub struct Par {
    /// List of `Control` statements to run in parallel.
    pub stmts: Vec<Control>,
    /// Attributes attached to this control statement.
    pub attributes: Attributes,
}

/// Data for the `if` control statement.
#[derive(Debug)]
pub struct If {
    /// Port that connects the conditional check.
    pub port: RRC<Port>,

    /// Group that makes the signal on the conditional port valid.
    pub cond: RRC<Group>,

    /// Control for the true branch.
    pub tbranch: Box<Control>,

    /// Control for the true branch.
    pub fbranch: Box<Control>,

    /// Attributes attached to this control statement.
    pub attributes: Attributes,
}

/// Data for the `if` control statement.
#[derive(Debug)]
pub struct While {
    /// Port that connects the conditional check.
    pub port: RRC<Port>,

    /// Group that makes the signal on the conditional port valid.
    pub cond: RRC<Group>,

    /// Control for the loop body.
    pub body: Box<Control>,
    /// Attributes attached to this control statement.
    pub attributes: Attributes,
}

/// Data for the `enable` control statement.
#[derive(Debug)]
pub struct Enable {
    /// List of components to run.
    pub group: RRC<Group>,
    /// Attributes attached to this control statement.
    pub attributes: Attributes,
}

type PortMap = Vec<(Id, RRC<Port>)>;

/// Data for an `invoke` control statement.
#[derive(Debug)]
pub struct Invoke {
    /// Cell that is being invoked.
    pub comp: RRC<Cell>,
    /// Mapping from name of input ports in `comp` to the port connected to it.
    pub inputs: PortMap,
    /// Mapping from name of output ports in `comp` to the port connected to it.
    pub outputs: PortMap,
    /// Attributes attached to this control statement.
    pub attributes: Attributes,
}

/// Data for the `empty` control statement.
#[derive(Debug)]
pub struct Empty {}

/// Control AST nodes.
#[derive(Debug)]
pub enum Control {
    /// Represents sequential composition of control statements.
    Seq(Seq),
    /// Represents parallel composition of control statements.
    Par(Par),
    /// Standard imperative if statement
    If(If),
    /// Standard imperative while statement
    While(While),
    /// Invoke a sub-component with the given port assignments
    Invoke(Invoke),
    /// Runs the control for a list of subcomponents.
    Enable(Enable),
    /// Control statement that does nothing.
    Empty(Empty),
}

impl Control {
    pub fn attributes(&mut self) -> &mut Attributes {
        match self {
            Self::Seq(Seq { attributes, .. })
            | Self::Par(Par { attributes, .. })
            | Self::If(If { attributes, .. })
            | Self::While(While { attributes, .. })
            | Self::Invoke(Invoke { attributes, .. })
            | Self::Enable(Enable { attributes, .. }) => attributes,
            Self::Empty(..) => {
                panic!("No attributes for Control::Empty statements")
            }
        }
    }

    // ================ Constructor methods ================
    /// Convience constructor for empty.
    pub fn empty() -> Self {
        Control::Empty(Empty {})
    }

    /// Convience constructor for seq.
    pub fn seq(stmts: Vec<Control>) -> Self {
        Control::Seq(Seq {
            stmts,
            attributes: Attributes::default(),
        })
    }

    /// Convience constructor for par.
    pub fn par(stmts: Vec<Control>) -> Self {
        Control::Par(Par {
            stmts,
            attributes: Attributes::default(),
        })
    }

    /// Convience constructor for enable.
    pub fn enable(group: RRC<Group>) -> Self {
        Control::Enable(Enable {
            group,
            attributes: Attributes::default(),
        })
    }

    /// Convience constructor for invoke.
    pub fn invoke(comp: RRC<Cell>, inputs: PortMap, outputs: PortMap) -> Self {
        Control::Invoke(Invoke {
            comp,
            inputs,
            outputs,
            attributes: Attributes::default(),
        })
    }

    /// Convience constructor for if
    pub fn if_(
        port: RRC<Port>,
        cond: RRC<Group>,
        tbranch: Box<Control>,
        fbranch: Box<Control>,
    ) -> Self {
        Control::If(If {
            port,
            cond,
            tbranch,
            fbranch,
            attributes: Attributes::default(),
        })
    }

    /// Convience constructor for while
    pub fn while_(
        port: RRC<Port>,
        cond: RRC<Group>,
        body: Box<Control>,
    ) -> Self {
        Control::While(While {
            port,
            cond,
            body,
            attributes: Attributes::default(),
        })
    }
}
