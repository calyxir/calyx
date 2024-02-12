use calyx_frontend::Attribute;

use super::StaticGroup;
use std::rc::Rc;

use super::{Attributes, Cell, CombGroup, GetAttributes, Group, Id, Port, RRC};

type StaticLatency = u64;

/// Data for the `seq` control statement.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
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

/// Data for the `static seq` control statement.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct StaticSeq {
    /// List of `StaticControl` statements to run in sequence.
    pub stmts: Vec<StaticControl>,
    /// Attributes attached to this control statement.
    pub attributes: Attributes,
    /// Latency, in cycles
    pub latency: StaticLatency,
}
impl GetAttributes for StaticSeq {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn get_mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

/// Data for the `par` control statement.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
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

// Data for the `static par` control statement.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct StaticPar {
    /// List of `StaticControl` statements to run in parallel.
    pub stmts: Vec<StaticControl>,
    /// Attributes attached to this control statement.
    pub attributes: Attributes,
    /// Latency, in cycles
    pub latency: StaticLatency,
}
impl GetAttributes for StaticPar {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn get_mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

/// Data for the `if` control statement.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
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
impl GetAttributes for If {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn get_mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

/// Data for the `static if` control statement.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct StaticIf {
    /// Port that connects the conditional check.
    pub port: RRC<Port>,

    /// latency field
    /// currrently, if two if branches take different amounts of time,
    /// the latency to the length of the longer branch
    pub latency: StaticLatency,

    /// Control for the true branch.
    pub tbranch: Box<StaticControl>,

    /// Control for the false branch.
    pub fbranch: Box<StaticControl>,

    /// Attributes attached to this control statement.
    pub attributes: Attributes,
}
impl GetAttributes for StaticIf {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn get_mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

/// Data for the `while` control statement.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
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
impl GetAttributes for While {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn get_mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

/// Data for the Dynamic `Repeat` control statement. Repeats the body of the loop
/// a given number times.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Repeat {
    /// Attributes
    pub attributes: Attributes,
    /// Body to repeat
    pub body: Box<Control>,
    /// Number of times to repeat the body
    pub num_repeats: u64,
}
impl GetAttributes for Repeat {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn get_mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

/// Data for the `StaticRepeat` control statement. Essentially a static while loop.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct StaticRepeat {
    /// Attributes
    pub attributes: Attributes,
    /// Body to repeat
    pub body: Box<StaticControl>,
    /// Number of times to repeat the body
    pub num_repeats: u64,
    /// latency = num_repeats * (body latency)
    pub latency: StaticLatency,
}
impl GetAttributes for StaticRepeat {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn get_mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

/// Data for the `enable` control statement.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Enable {
    /// List of components to run.
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

/// Data for the `enable` control for a static group.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct StaticEnable {
    /// List of components to run.
    pub group: RRC<StaticGroup>,
    /// Attributes attached to this control statement.
    pub attributes: Attributes,
}
impl GetAttributes for StaticEnable {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn get_mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl StaticEnable {
    /// Returns the value of an attribute if present
    pub fn get_attribute(&self, attr: Attribute) -> Option<u64> {
        self.get_attributes().get(attr)
    }
}

type PortMap = Vec<(Id, RRC<Port>)>;
type CellMap = Vec<(Id, RRC<Cell>)>;

/// Data for an `invoke` control statement.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
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
impl GetAttributes for Invoke {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn get_mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

/// Data for a `StaticInvoke` control statement
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct StaticInvoke {
    /// Cell that is being invoked.
    pub comp: RRC<Cell>,
    /// StaticLatency
    pub latency: StaticLatency,
    /// Mapping from name of input ports in `comp` to the port connected to it.
    pub inputs: PortMap,
    /// Mapping from name of output ports in `comp` to the port connected to it.
    pub outputs: PortMap,
    /// Attributes attached to this control statement.
    pub attributes: Attributes,
    /// Mapping from name of external cell in 'comp' to the cell connected to it.
    pub ref_cells: CellMap,
    /// Optional combinational group that is active when the invoke is active.
    pub comb_group: Option<RRC<CombGroup>>,
}
impl GetAttributes for StaticInvoke {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn get_mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

/// Data for the `empty` control statement.
#[derive(Debug, Default)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Empty {
    pub attributes: Attributes,
}

/// Control AST nodes.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub enum Control {
    /// Represents sequential composition of control statements.
    Seq(Seq),
    /// Represents parallel composition of control statements.
    Par(Par),
    /// Standard imperative if statement
    If(If),
    /// Standard imperative while statement
    While(While),
    /// Standard repeat control statement
    Repeat(Repeat),
    /// Invoke a sub-component with the given port assignments
    Invoke(Invoke),
    /// Runs the control for a list of subcomponents.
    Enable(Enable),
    /// Control statement that does nothing.
    Empty(Empty),
    /// Static Control
    Static(StaticControl),
}

/// Control AST nodes.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub enum StaticControl {
    Repeat(StaticRepeat),
    Enable(StaticEnable),
    Par(StaticPar),
    Seq(StaticSeq),
    If(StaticIf),
    Empty(Empty),
    Invoke(StaticInvoke),
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

impl From<StaticControl> for Control {
    fn from(sc: StaticControl) -> Self {
        Control::Static(sc)
    }
}

impl From<StaticEnable> for StaticControl {
    fn from(se: StaticEnable) -> Self {
        StaticControl::Enable(se)
    }
}

impl From<RRC<StaticGroup>> for StaticControl {
    fn from(sgroup: RRC<StaticGroup>) -> Self {
        StaticControl::Enable(StaticEnable {
            group: sgroup,
            attributes: Attributes::default(),
        })
    }
}

impl<'a> From<&'a Control> for GenericControl<'a> {
    fn from(c: &'a Control) -> Self {
        match c {
            Control::Static(sc) => GenericControl::Static(sc),
            _ => GenericControl::Dynamic(c),
        }
    }
}

impl<'a> From<&'a StaticControl> for GenericControl<'a> {
    fn from(sc: &'a StaticControl) -> Self {
        GenericControl::Static(sc)
    }
}

impl GetAttributes for Control {
    fn get_mut_attributes(&mut self) -> &mut Attributes {
        match self {
            Self::Seq(Seq { attributes, .. })
            | Self::Par(Par { attributes, .. })
            | Self::If(If { attributes, .. })
            | Self::While(While { attributes, .. })
            | Self::Repeat(Repeat { attributes, .. })
            | Self::Invoke(Invoke { attributes, .. })
            | Self::Enable(Enable { attributes, .. })
            | Self::Empty(Empty { attributes }) => attributes,
            Self::Static(s) => s.get_mut_attributes(),
        }
    }

    fn get_attributes(&self) -> &Attributes {
        match self {
            Self::Seq(Seq { attributes, .. })
            | Self::Par(Par { attributes, .. })
            | Self::If(If { attributes, .. })
            | Self::While(While { attributes, .. })
            | Self::Repeat(Repeat { attributes, .. })
            | Self::Invoke(Invoke { attributes, .. })
            | Self::Enable(Enable { attributes, .. })
            | Self::Empty(Empty { attributes }) => attributes,
            Self::Static(s) => s.get_attributes(),
        }
    }
}

impl GetAttributes for StaticControl {
    fn get_mut_attributes(&mut self) -> &mut Attributes {
        match self {
            Self::Enable(StaticEnable { attributes, .. }) => attributes,
            Self::Repeat(StaticRepeat { attributes, .. }) => attributes,
            Self::Par(StaticPar { attributes, .. }) => attributes,
            Self::Seq(StaticSeq { attributes, .. }) => attributes,
            Self::If(StaticIf { attributes, .. }) => attributes,
            Self::Empty(Empty { attributes, .. }) => attributes,
            Self::Invoke(StaticInvoke { attributes, .. }) => attributes,
        }
    }
    fn get_attributes(&self) -> &Attributes {
        match self {
            Self::Enable(StaticEnable { attributes, .. }) => attributes,
            Self::Repeat(StaticRepeat { attributes, .. }) => attributes,
            Self::Par(StaticPar { attributes, .. }) => attributes,
            Self::Seq(StaticSeq { attributes, .. }) => attributes,
            Self::If(StaticIf { attributes, .. }) => attributes,
            Self::Empty(Empty { attributes, .. }) => attributes,
            Self::Invoke(StaticInvoke { attributes, .. }) => attributes,
        }
    }
}

impl calyx_utils::WithPos for Control {
    fn copy_span(&self) -> calyx_utils::GPosIdx {
        self.get_attributes().copy_span()
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

    /// Convience constructor for enable.
    pub fn static_enable(group: RRC<StaticGroup>) -> Self {
        Control::Static(StaticControl::Enable(StaticEnable {
            group,
            attributes: Attributes::default(),
        }))
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

    /// Convience constructor for dynamic repeat
    pub fn repeat(num_repeats: u64, body: Box<Control>) -> Self {
        Control::Repeat(Repeat {
            body,
            num_repeats,
            attributes: Attributes::default(),
        })
    }

    /// Returns the value of an attribute if present
    pub fn get_attribute<A>(&self, attr: A) -> Option<u64>
    where
        A: Into<Attribute>,
    {
        self.get_attributes().get(attr)
    }

    /// Returns true if the node has a specific attribute
    pub fn has_attribute<A>(&self, attr: A) -> bool
    where
        A: Into<Attribute>,
    {
        self.get_attributes().has(attr)
    }

    pub fn is_static(&self) -> bool {
        matches!(self, Control::Static(_))
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Control::Static(StaticControl::Empty(_)))
            || matches!(self, Control::Empty(_))
    }

    pub fn get_latency(&self) -> Option<StaticLatency> {
        match self {
            Control::Static(sc) => Some(sc.get_latency()),
            _ => None,
        }
    }

    /// Replaces &mut self with an empty control statement, and returns self
    pub fn take_control(&mut self) -> Control {
        let empty = Control::empty();
        std::mem::replace(self, empty)
    }

    /// Replaces &mut self with an empty control statement, and returns StaticControl
    /// of self. Note that this only works on Control that is static
    pub fn take_static_control(&mut self) -> StaticControl {
        let empty = Control::empty();
        let control = std::mem::replace(self, empty);
        let Control::Static(static_control) = control else {
            unreachable!("Called take_static_control on non-static control")
        };
        static_control
    }
}

impl StaticControl {
    /// Convience constructor for empty.
    pub fn empty() -> Self {
        StaticControl::Empty(Empty::default())
    }

    /// Convience constructor for static enable.
    pub fn seq(stmts: Vec<StaticControl>, latency: u64) -> Self {
        StaticControl::Seq(StaticSeq {
            stmts,
            attributes: Attributes::default(),
            latency,
        })
    }

    /// Convience constructor for static enable.
    pub fn par(stmts: Vec<StaticControl>, latency: u64) -> Self {
        StaticControl::Par(StaticPar {
            stmts,
            attributes: Attributes::default(),
            latency,
        })
    }

    /// Convience constructor for static if
    pub fn static_if(
        port: RRC<Port>,
        tbranch: Box<StaticControl>,
        fbranch: Box<StaticControl>,
        latency: u64,
    ) -> Self {
        StaticControl::If(StaticIf {
            port,
            tbranch,
            fbranch,
            attributes: Attributes::default(),
            latency,
        })
    }

    /// Convience constructor for static if
    pub fn repeat(
        num_repeats: u64,
        latency: u64,
        body: Box<StaticControl>,
    ) -> Self {
        StaticControl::Repeat(StaticRepeat {
            body,
            num_repeats,
            latency,
            attributes: Attributes::default(),
        })
    }

    /// Returns the value of an attribute if present
    pub fn get_attribute(&self, attr: Attribute) -> Option<u64> {
        self.get_attributes().get(attr)
    }

    /// Returns the value of an attribute if present
    pub fn get_latency(&self) -> StaticLatency {
        match self {
            StaticControl::Enable(StaticEnable { group, .. }) => {
                group.borrow().get_latency()
            }
            StaticControl::Seq(StaticSeq { latency, .. })
            | StaticControl::Par(StaticPar { latency, .. })
            | StaticControl::Repeat(StaticRepeat { latency, .. })
            | StaticControl::If(StaticIf { latency, .. })
            | StaticControl::Invoke(StaticInvoke { latency, .. }) => *latency,
            &StaticControl::Empty(_) => 0,
        }
    }

    /// Replaces &mut self with an empty static control statement, and returns self
    pub fn take_static_control(&mut self) -> StaticControl {
        let empty = StaticControl::empty();
        std::mem::replace(self, empty)
    }
}

#[derive(Debug)]
/// Either holds a reference to a StaticControl, reference to a Control, or None
/// Helpful when we want to be able get get any specific control statement within a
/// control program. For example, suppose we assign an id to each enable (static or dynamic)
/// in the control program. A function that takes in an id and returns the appropriate
/// enable would have to return a GenericControl.
/// Has the weird affect that GenericControl::Dynamic(Control::Static(_)) can be
/// a bit redundant with GenericControl::Static(_) but the latter gives us more precise access
/// to every enum in the static control, instead of just the big wrapper.
pub enum GenericControl<'a> {
    Static(&'a StaticControl),
    Dynamic(&'a Control),
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

    pub fn static_enable(en: &StaticEnable) -> StaticEnable {
        StaticEnable {
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

    pub fn static_repeat(rep: &StaticRepeat) -> StaticRepeat {
        StaticRepeat {
            attributes: rep.attributes.clone(),
            body: Box::new(Self::static_control(&rep.body)),
            num_repeats: rep.num_repeats,
            latency: rep.latency,
        }
    }

    pub fn repeat(rep: &Repeat) -> Repeat {
        Repeat {
            attributes: rep.attributes.clone(),
            body: Box::new(Self::control(&rep.body)),
            num_repeats: rep.num_repeats,
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

    pub fn static_if(sif: &StaticIf) -> StaticIf {
        StaticIf {
            port: Rc::clone(&sif.port),
            latency: sif.latency,
            tbranch: Box::new(Self::static_control(&sif.tbranch)),
            fbranch: Box::new(Self::static_control(&sif.fbranch)),
            attributes: sif.attributes.clone(),
        }
    }

    pub fn par(par: &Par) -> Par {
        Par {
            stmts: par.stmts.iter().map(Self::control).collect(),
            attributes: par.attributes.clone(),
        }
    }

    pub fn static_par(par: &StaticPar) -> StaticPar {
        StaticPar {
            stmts: par.stmts.iter().map(Self::static_control).collect(),
            attributes: par.attributes.clone(),
            latency: par.latency,
        }
    }

    pub fn seq(seq: &Seq) -> Seq {
        Seq {
            stmts: seq.stmts.iter().map(Self::control).collect(),
            attributes: seq.attributes.clone(),
        }
    }

    pub fn static_seq(seq: &StaticSeq) -> StaticSeq {
        StaticSeq {
            stmts: seq.stmts.iter().map(Self::static_control).collect(),
            attributes: seq.attributes.clone(),
            latency: seq.latency,
        }
    }

    pub fn static_invoke(i: &StaticInvoke) -> StaticInvoke {
        StaticInvoke {
            comp: Rc::clone(&i.comp),
            latency: i.latency,
            inputs: i.inputs.clone(),
            outputs: i.outputs.clone(),
            attributes: i.attributes.clone(),
            ref_cells: i.ref_cells.clone(),
            comb_group: i.comb_group.clone(),
        }
    }

    pub fn static_control(s: &StaticControl) -> StaticControl {
        match s {
            StaticControl::Enable(sen) => {
                StaticControl::Enable(Cloner::static_enable(sen))
            }
            StaticControl::Repeat(rep) => {
                StaticControl::Repeat(Cloner::static_repeat(rep))
            }
            StaticControl::Seq(sseq) => {
                StaticControl::Seq(Cloner::static_seq(sseq))
            }
            StaticControl::Par(spar) => {
                StaticControl::Par(Cloner::static_par(spar))
            }
            StaticControl::If(sif) => StaticControl::If(Cloner::static_if(sif)),
            StaticControl::Empty(e) => StaticControl::Empty(Self::empty(e)),
            StaticControl::Invoke(si) => {
                StaticControl::Invoke(Self::static_invoke(si))
            }
        }
    }

    pub fn control(con: &Control) -> Control {
        match con {
            Control::Seq(seq) => Control::Seq(Cloner::seq(seq)),
            Control::Par(par) => Control::Par(Cloner::par(par)),
            Control::If(if_) => Control::If(Cloner::if_(if_)),
            Control::While(wh) => Control::While(Cloner::while_(wh)),
            Control::Repeat(repeat) => Control::Repeat(Cloner::repeat(repeat)),
            Control::Invoke(inv) => Control::Invoke(Cloner::invoke(inv)),
            Control::Enable(en) => Control::Enable(Cloner::enable(en)),
            Control::Empty(en) => Control::Empty(Cloner::empty(en)),
            Control::Static(s) => Control::Static(Cloner::static_control(s)),
        }
    }
}
