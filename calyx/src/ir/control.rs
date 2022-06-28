use std::rc::Rc;

use super::{Attributes, Cell, CombGroup, GetAttributes, Group, Id, Port, RRC};

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
    pub tbranch: Box<Control>,

    /// Control for the false branch.
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
    pub cond: Option<RRC<CombGroup>>,

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
type CellMap = Vec<(Id, RRC<Cell>)>;

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
    /// Optional combinational group that is active when the invoke is active.
    pub comb_group: Option<RRC<CombGroup>>,
    /// Mapping from name of external cell in 'comp' to the cell connected to it.
    pub ref_cells: CellMap,
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

impl GetAttributes for Control {
    fn get_mut_attributes(&mut self) -> Option<&mut Attributes> {
        match self {
            Self::Seq(Seq { attributes, .. })
            | Self::Par(Par { attributes, .. })
            | Self::If(If { attributes, .. })
            | Self::While(While { attributes, .. })
            | Self::Invoke(Invoke { attributes, .. })
            | Self::Enable(Enable { attributes, .. }) => Some(attributes),
            Self::Empty(..) => None,
        }
    }

    fn get_attributes(&self) -> Option<&Attributes> {
        match self {
            Self::Seq(Seq { attributes, .. })
            | Self::Par(Par { attributes, .. })
            | Self::If(If { attributes, .. })
            | Self::While(While { attributes, .. })
            | Self::Invoke(Invoke { attributes, .. })
            | Self::Enable(Enable { attributes, .. }) => Some(attributes),
            Self::Empty(..) => None,
        }
    }
}

impl Control {
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
            comb_group: None,
            ref_cells: Vec::new(),
        })
    }

    /// Convience constructor for if
    pub fn if_(
        port: RRC<Port>,
        cond: Option<RRC<CombGroup>>,
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
        cond: Option<RRC<CombGroup>>,
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

impl Control {
    /// Associated clone method the control program. We don't define this using the
    /// [Clone] trait because cloning control is not very common and clones
    /// should be explicit.
    #[allow(clippy::should_implement_trait)]
    pub fn clone(con: &Control) -> Control {
        match con {
            Control::Seq(Seq { stmts, attributes }) => Control::Seq(Seq {
                stmts: stmts.iter().map(Control::clone).collect(),
                attributes: attributes.clone(),
            }),
            Control::Par(Par { stmts, attributes }) => Control::Par(Par {
                stmts: stmts.iter().map(Control::clone).collect(),
                attributes: attributes.clone(),
            }),
            Control::If(If {
                port,
                cond,
                tbranch,
                fbranch,
                attributes,
            }) => Control::If(If {
                port: Rc::clone(port),
                cond: cond.clone().map(|cg| Rc::clone(&cg)),
                tbranch: Box::new(Control::clone(tbranch)),
                fbranch: Box::new(Control::clone(fbranch)),
                attributes: attributes.clone(),
            }),
            Control::While(While {
                port,
                cond,
                body,
                attributes,
            }) => Control::While(While {
                port: Rc::clone(port),
                cond: cond.clone().map(|cg| Rc::clone(&cg)),
                body: Box::new(Control::clone(body)),
                attributes: attributes.clone(),
            }),
            Control::Invoke(Invoke {
                comp,
                inputs,
                outputs,
                attributes,
                comb_group,
                ref_cells,
            }) => Control::Invoke(Invoke {
                comp: Rc::clone(comp),
                inputs: inputs
                    .iter()
                    .map(|(name, port)| (name.clone(), Rc::clone(port)))
                    .collect(),
                outputs: outputs
                    .iter()
                    .map(|(name, port)| (name.clone(), Rc::clone(port)))
                    .collect(),
                comb_group: comb_group.clone().map(|cg| Rc::clone(&cg)),
                attributes: attributes.clone(),
                ref_cells: ref_cells
                    .iter()
                    .map(|(outcell, incell)| {
                        (outcell.clone(), Rc::clone(incell))
                    })
                    .collect(),
            }),
            Control::Enable(Enable { group, attributes }) => {
                Control::Enable(Enable {
                    group: Rc::clone(group),
                    attributes: attributes.clone(),
                })
            }
            Control::Empty(_) => Control::empty(),
        }
    }
}
