use crate::lang::ast::Id;
use super::{
    component::{Group, Port},
};

/// Data for the `seq` control statement.
//#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Seq<'a> {
    /// List of `Control` statements to run in sequence.
    pub stmts: Vec<Control<'a>>,
}

/// Data for the `par` control statement.
//#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Par<'a> {
    /// List of `Control` statements to run in parallel.
    pub stmts: Vec<Control<'a>>,
}

/// Data for the `if` control statement.
//#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct If<'a> {
    /// Port that connects the conditional check.
    pub port: &'a Port,

    /// Modules that need to be enabled to send signal on `port`.
    pub cond: Id,

    /// Control for the true branch.
    pub tbranch: Box<Control<'a>>,

    /// Control for the true branch.
    pub fbranch: Box<Control<'a>>,
}

/// Data for the `if` control statement.
//#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct While<'a> {
    /// Port that connects the conditional check.
    pub port: &'a Port,

    /// Modules that need to be enabled to send signal on `port`.
    pub cond: Id,

    /// Control for the loop body.
    pub body: Box<Control<'a>>,
}

/// Data for the `enable` control statement.
//#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Enable<'a> {
    /// List of components to run.
    pub comp: &'a Group<'a>,
}

/// Data for the `empty` control statement.
//#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Empty {}

/// Control AST nodes.
//#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Control<'a> {
    /// Represents sequential composition of control statements.
    Seq { data: Seq<'a> },
    /// Represents parallel composition of control statements.
    Par { data: Par<'a> },
    /// Standard imperative if statement
    If { data: If<'a> },
    /// Standard imperative while statement
    While { data: While<'a> },
    /// Runs the control for a list of subcomponents.
    Enable { data: Enable<'a> },
    /// Control statement that does nothing.
    Empty { data: Empty },
}
