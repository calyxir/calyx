use super::{Group, Port, RRC, Id, Cell};

/// Data for the `seq` control statement.
#[derive(Debug)]
pub struct Seq {
    /// List of `Control` statements to run in sequence.
    pub stmts: Vec<Control>,
}

/// Data for the `par` control statement.
#[derive(Debug)]
pub struct Par {
    /// List of `Control` statements to run in parallel.
    pub stmts: Vec<Control>,
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
}

/// Data for the `enable` control statement.
#[derive(Debug)]
pub struct Enable {
    /// List of components to run.
    pub group: RRC<Group>,
}

type PortMap = Vec<(Id, RRC<Port>)>;

#[derive(Debug)]
pub struct Invoke {
    /// Cell that is being invoked.
    pub comp: RRC<Cell>,
    /// Mapping from name of input ports in `comp` to the port connected to it.
    pub inputs: PortMap,
    /// Mapping from name of output ports in `comp` to the port connected to it.
    pub outputs: PortMap,
}

impl From<RRC<Group>> for Enable {
    fn from(group: RRC<Group>) -> Self {
        Enable { group }
    }
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
    /// Convience constructor for empty.
    pub fn empty() -> Self {
        Control::Empty(Empty {})
    }

    /// Convience constructor for seq.
    pub fn seq(stmts: Vec<Control>) -> Self {
        Control::Seq(Seq { stmts })
    }

    /// Convience constructor for par.
    pub fn par(stmts: Vec<Control>) -> Self {
        Control::Par(Par { stmts })
    }

    /// Convience constructor for enable.
    pub fn enable(group: RRC<Group>) -> Self {
        Control::Enable(Enable { group })
    }

    /// Convience constructor for invoke.
    pub fn invoke(comp: RRC<Cell>, inputs: PortMap, outputs: PortMap) -> Self {
        Control::Invoke(Invoke { comp, inputs, outputs })
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
        })
    }

    /// Convience constructor for while
    pub fn while_(
        port: RRC<Port>,
        cond: RRC<Group>,
        body: Box<Control>,
    ) -> Self {
        Control::While(While { port, cond, body })
    }
}
