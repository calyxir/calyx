use calyx_ir::Control as CalyxControl;
use calyx_ir::{self as ir, Attributes, CombGroup, Port, RRC};

use std::rc::Rc;

// These IR constructs are unchanged but are here re-exported for consistency
pub use calyx_ir::{Empty, Enable, Invoke};

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
    /// Optional combinational group attached using `with`.
    pub cond: Option<RRC<CombGroup>>,
    /// Control for the true branch.
    pub tbranch: Control,
    /// Control for the true branch.
    pub fbranch: Control,
    /// Attributes attached to this control statement.
    pub attributes: Attributes,
}

/// Data for the `if` control statement.
#[derive(Debug)]
pub struct While {
    /// Port that connects the conditional check.
    pub port: RRC<Port>,
    /// Group that makes the signal on the conditional port valid.
    pub cond: Option<RRC<CombGroup>>,
    /// Control for the loop body.
    pub body: Control,
    /// Attributes attached to this control statement.
    pub attributes: Attributes,
}

/// Control AST nodes.
#[derive(Debug, Clone)]
pub enum Control {
    /// Represents sequential composition of control statements.
    Seq(Rc<Seq>),
    /// Represents parallel composition of control statements.
    Par(Rc<Par>),
    /// Standard imperative if statement
    If(Rc<If>),
    /// Standard imperative while statement
    While(Rc<While>),
    /// Invoke a sub-component with the given port assignments
    Invoke(Rc<Invoke>),
    /// Runs the control for a list of subcomponents.
    Enable(Rc<Enable>),
    /// Control statement that does nothing.
    Empty(Rc<Empty>),
}

impl From<CalyxControl> for Control {
    fn from(cc: CalyxControl) -> Self {
        match cc {
            CalyxControl::Seq(s) => Control::Seq(Rc::new(s.into())),
            CalyxControl::Par(p) => Control::Par(Rc::new(p.into())),
            CalyxControl::If(i) => Control::If(Rc::new(i.into())),
            CalyxControl::While(wh) => Control::While(Rc::new(wh.into())),
            CalyxControl::Invoke(invoke) => Control::Invoke(Rc::new(invoke)),
            CalyxControl::Enable(enable) => Control::Enable(Rc::new(enable)),
            CalyxControl::Static(_) => {
                todo!("interpreter does not yet support static")
            }
            CalyxControl::Repeat(_) => {
                todo!("interpreter does not yet support repeat")
            }
            CalyxControl::Empty(empty) => Control::Empty(Rc::new(empty)),
        }
    }
}

impl From<ir::Seq> for Seq {
    fn from(seq: ir::Seq) -> Self {
        Self {
            stmts: seq.stmts.into_iter().map(|x| x.into()).collect(),
            attributes: seq.attributes,
        }
    }
}

impl From<ir::Par> for Par {
    fn from(par: ir::Par) -> Self {
        Self {
            stmts: par.stmts.into_iter().map(|x| x.into()).collect(),
            attributes: par.attributes,
        }
    }
}

impl From<ir::If> for If {
    fn from(i: ir::If) -> Self {
        Self {
            port: i.port,
            cond: i.cond,
            tbranch: (*i.tbranch).into(),
            fbranch: (*i.fbranch).into(),
            attributes: i.attributes,
        }
    }
}

impl From<ir::While> for While {
    fn from(wh: ir::While) -> Self {
        Self {
            port: wh.port,
            cond: wh.cond,
            body: (*wh.body).into(),
            attributes: wh.attributes,
        }
    }
}
