use super::{Group, Port, RRC};

/// Data for the `seq` control statement.
//#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Seq {
    /// List of `Control` statements to run in sequence.
    pub stmts: Vec<Control>,
}

/// Data for the `par` control statement.
//#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Par {
    /// List of `Control` statements to run in parallel.
    pub stmts: Vec<Control>,
}

/// Data for the `if` control statement.
//#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct If {
    /// Port that connects the conditional check.
    pub port: RRC<Port>,

    /// Group that makes the signal on the conditional port valid.
    pub group: RRC<Group>,

    /// Control for the true branch.
    pub tbranch: Box<Control>,

    /// Control for the true branch.
    pub fbranch: Box<Control>,
}

/// Data for the `if` control statement.
//#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct While {
    /// Port that connects the conditional check.
    pub port: RRC<Port>,

    /// Group that makes the signal on the conditional port valid.
    pub group: RRC<Group>,

    /// Control for the loop body.
    pub body: Box<Control>,
}

/// Data for the `enable` control statement.
//#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Enable {
    /// List of components to run.
    pub group: RRC<Group>,
}

/// Data for the `empty` control statement.
//#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Empty {}

/// Control AST nodes.
//#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Control {
    /// Represents sequential composition of control statements.
    Seq(Seq),
    /// Represents parallel composition of control statements.
    Par(Par),
    /// Standard imperative if statement
    If(If),
    /// Standard imperative while statement
    While(While),
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

    /// Convience constructor for par.
    pub fn enable(group: RRC<Group>) -> Self {
        Control::Enable(Enable { group })
    }

    /// Convience constructor for if
    pub fn if_(
        port: RRC<Port>,
        group: RRC<Group>,
        tbranch: Box<Control>,
        fbranch: Box<Control>,
    ) -> Self {
        Control::If(If {
            port,
            group,
            tbranch,
            fbranch,
        })
    }

    /// Convience constructor for while
    pub fn while_(
        port: RRC<Port>,
        group: RRC<Group>,
        body: Box<Control>,
    ) -> Self {
        Control::While(While { port, group, body })
    }
}
