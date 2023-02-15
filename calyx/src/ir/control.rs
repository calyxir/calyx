use std::rc::Rc;

use serde::Serialize;
use serde_with::{serde_as, SerializeAs};

use super::{
    Attributes, Cell, CombGroup, GetAttributes, Group, Id, Port, SerCellRef,
    SerPortRef, RRC,
};

/// Data for the `seq` control statement.
#[derive(Debug, Serialize)]
pub struct Seq {
    /// List of `Control` statements to run in sequence.
    pub stmts: Vec<Control>,
    /// Attributes attached to this control statement.
    pub attributes: Attributes,
}
impl GetAttributes for Seq {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn get_mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

/// Data for the `par` control statement.
#[derive(Debug, Serialize)]
pub struct Par {
    /// List of `Control` statements to run in parallel.
    pub stmts: Vec<Control>,
    /// Attributes attached to this control statement.
    pub attributes: Attributes,
}
impl GetAttributes for Par {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn get_mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

struct SerCombGroupRef;
impl SerializeAs<RRC<CombGroup>> for SerCombGroupRef {
    fn serialize_as<S>(
        value: &RRC<CombGroup>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        value.borrow().name.serialize(serializer)
    }
}

/// Data for the `if` control statement.
#[serde_as]
#[derive(Debug, Serialize)]
pub struct If {
    /// Port that connects the conditional check.
    #[serde_as(as = "SerPortRef")]
    pub port: RRC<Port>,

    /// Optional combinational group attached using `with`.
    #[serde_as(as = "Option<SerCombGroupRef>")]
    pub cond: Option<RRC<CombGroup>>,

    /// Control for the true branch.
    pub tbranch: Box<Control>,

    /// Control for the false branch.
    pub fbranch: Box<Control>,

    /// Attributes attached to this control statement.
    pub attributes: Attributes,
}
impl GetAttributes for If {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn get_mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

/// Data for the `while` control statement.
#[serde_as]
#[derive(Debug, Serialize)]
pub struct While {
    /// Port that connects the conditional check.
    #[serde_as(as = "SerPortRef")]
    pub port: RRC<Port>,
    /// Group that makes the signal on the conditional port valid.
    #[serde_as(as = "Option<SerCombGroupRef>")]
    pub cond: Option<RRC<CombGroup>>,
    /// Control for the loop body.
    pub body: Box<Control>,
    /// Attributes attached to this control statement.
    pub attributes: Attributes,
}
impl GetAttributes for While {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn get_mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

struct SerGroupRef;
impl SerializeAs<RRC<Group>> for SerGroupRef {
    fn serialize_as<S>(
        value: &RRC<Group>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        value.borrow().name().serialize(serializer)
    }
}

/// Data for the `enable` control statement.
#[serde_as]
#[derive(Debug, Serialize)]
pub struct Enable {
    /// List of components to run.
    #[serde_as(as = "SerGroupRef")]
    pub group: RRC<Group>,
    /// Attributes attached to this control statement.
    pub attributes: Attributes,
}
impl GetAttributes for Enable {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn get_mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

type CellMap = Vec<(Id, RRC<Cell>)>;
type PortMap = Vec<(Id, RRC<Port>)>;

/// Data for an `invoke` control statement.
#[serde_as]
#[derive(Debug, Serialize)]
pub struct Invoke {
    /// Cell that is being invoked.
    #[serde_as(as = "SerCellRef")]
    pub comp: RRC<Cell>,
    /// Mapping from name of input ports in `comp` to the port connected to it.
    #[serde_as(as = "Vec<(_, SerPortRef)>")]
    pub inputs: PortMap,
    /// Mapping from name of output ports in `comp` to the port connected to it.
    #[serde_as(as = "Vec<(_, SerPortRef)>")]
    pub outputs: PortMap,
    /// Attributes attached to this control statement.
    pub attributes: Attributes,
    /// Optional combinational group that is active when the invoke is active.
    #[serde_as(as = "Option<SerCombGroupRef>")]
    pub comb_group: Option<RRC<CombGroup>>,
    /// Mapping from name of external cell in 'comp' to the cell connected to it.
    #[serde_as(as = "Vec<(_, SerCellRef)>")]
    pub ref_cells: CellMap,
}
impl GetAttributes for Invoke {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn get_mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

/// Data for the `empty` control statement.
#[derive(Debug, Default, Serialize)]
pub struct Empty {
    pub attributes: Attributes,
}

/// Control AST nodes.
#[derive(Debug, Serialize)]
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

impl From<Invoke> for Control {
    fn from(inv: Invoke) -> Self {
        Control::Invoke(inv)
    }
}

impl From<Enable> for Control {
    fn from(en: Enable) -> Self {
        Control::Enable(en)
    }
}

impl GetAttributes for Control {
    fn get_mut_attributes(&mut self) -> &mut Attributes {
        match self {
            Self::Seq(Seq { attributes, .. })
            | Self::Par(Par { attributes, .. })
            | Self::If(If { attributes, .. })
            | Self::While(While { attributes, .. })
            | Self::Invoke(Invoke { attributes, .. })
            | Self::Enable(Enable { attributes, .. })
            | Self::Empty(Empty { attributes }) => attributes,
        }
    }

    fn get_attributes(&self) -> &Attributes {
        match self {
            Self::Seq(Seq { attributes, .. })
            | Self::Par(Par { attributes, .. })
            | Self::If(If { attributes, .. })
            | Self::While(While { attributes, .. })
            | Self::Invoke(Invoke { attributes, .. })
            | Self::Enable(Enable { attributes, .. })
            | Self::Empty(Empty { attributes }) => attributes,
        }
    }
}

impl Control {
    // ================ Constructor methods ================
    /// Convience constructor for empty.
    pub fn empty() -> Self {
        Control::Empty(Empty::default())
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

    /// Returns the value of an attribute if present
    pub fn get_attribute<S>(&self, attr: S) -> Option<u64>
    where
        S: Into<Id>,
    {
        self.get_attributes().get(attr).cloned()
    }

    /// Returns true if the node has a specific attribute
    pub fn has_attribute<S>(&self, attr: S) -> bool
    where
        S: Into<Id>,
    {
        self.get_attributes().has(attr)
    }
}

/// Implement cloning operations on control statements.
/// We implement these separatily from the [Clone] trait because cloning trait
/// is not very common and clones should be explicit.
pub struct Cloner;

impl Cloner {
    pub fn enable(en: &Enable) -> Enable {
        Enable {
            group: Rc::clone(&en.group),
            attributes: en.attributes.clone(),
        }
    }

    pub fn invoke(inv: &Invoke) -> Invoke {
        Invoke {
            comp: Rc::clone(&inv.comp),
            inputs: inv.inputs.clone(),
            outputs: inv.outputs.clone(),
            attributes: inv.attributes.clone(),
            comb_group: inv.comb_group.clone(),
            ref_cells: inv.ref_cells.clone(),
        }
    }

    pub fn empty(en: &Empty) -> Empty {
        Empty {
            attributes: en.attributes.clone(),
        }
    }

    pub fn while_(wh: &While) -> While {
        While {
            port: Rc::clone(&wh.port),
            cond: wh.cond.clone(),
            body: Box::new(Self::control(&wh.body)),
            attributes: wh.attributes.clone(),
        }
    }

    pub fn if_(if_: &If) -> If {
        If {
            port: Rc::clone(&if_.port),
            cond: if_.cond.clone(),
            tbranch: Box::new(Self::control(&if_.tbranch)),
            fbranch: Box::new(Self::control(&if_.fbranch)),
            attributes: if_.attributes.clone(),
        }
    }

    pub fn par(par: &Par) -> Par {
        Par {
            stmts: par.stmts.iter().map(Self::control).collect(),
            attributes: par.attributes.clone(),
        }
    }

    pub fn seq(seq: &Seq) -> Seq {
        Seq {
            stmts: seq.stmts.iter().map(Self::control).collect(),
            attributes: seq.attributes.clone(),
        }
    }

    pub fn control(con: &Control) -> Control {
        match con {
            Control::Seq(seq) => Control::Seq(Cloner::seq(seq)),
            Control::Par(par) => Control::Par(Cloner::par(par)),
            Control::If(if_) => Control::If(Cloner::if_(if_)),
            Control::While(wh) => Control::While(Cloner::while_(wh)),
            Control::Invoke(inv) => Control::Invoke(Cloner::invoke(inv)),
            Control::Enable(en) => Control::Enable(Cloner::enable(en)),
            Control::Empty(en) => Control::Empty(Cloner::empty(en)),
        }
    }
}
