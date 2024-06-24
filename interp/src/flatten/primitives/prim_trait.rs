use crate::{
    debugger::PrintCode,
    errors::InterpreterResult,
    flatten::{flat_ir::base::GlobalPortIdx, structures::environment::PortMap},
    serialization::Serializable,
    values::Value,
};

pub struct AssignResult {
    pub destination: GlobalPortIdx,
    pub value: Value,
}

impl AssignResult {
    pub fn new(destination: GlobalPortIdx, value: Value) -> Self {
        Self { destination, value }
    }
}

impl From<(GlobalPortIdx, Value)> for AssignResult {
    fn from(value: (GlobalPortIdx, Value)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl From<(Value, GlobalPortIdx)> for AssignResult {
    fn from(value: (Value, GlobalPortIdx)) -> Self {
        Self::new(value.1, value.0)
    }
}

/// An enum used to denote whether or not committed updates changed the state
#[derive(Debug)]
pub enum UpdateStatus {
    Unchanged,
    Changed,
}

impl From<bool> for UpdateStatus {
    fn from(value: bool) -> Self {
        if value {
            Self::Changed
        } else {
            Self::Unchanged
        }
    }
}

impl UpdateStatus {
    #[inline]
    /// If the status is unchanged and other is changed, updates the status of
    /// self to changed, otherwise does nothing
    pub fn update(&mut self, other: Self) {
        if !self.as_bool() && other.as_bool() {
            *self = UpdateStatus::Changed;
        }
    }

    #[inline]
    /// Returns `true` if the update status is [`Changed`].
    ///
    /// [`Changed`]: UpdateStatus::Changed
    #[must_use]
    pub fn as_bool(&self) -> bool {
        matches!(self, Self::Changed)
    }
}

impl std::ops::BitOr for UpdateStatus {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        if self.as_bool() || rhs.as_bool() {
            UpdateStatus::Changed
        } else {
            UpdateStatus::Unchanged
        }
    }
}

impl std::ops::BitOr for &UpdateStatus {
    type Output = UpdateStatus;

    fn bitor(self, rhs: Self) -> Self::Output {
        if self.as_bool() || rhs.as_bool() {
            UpdateStatus::Changed
        } else {
            UpdateStatus::Unchanged
        }
    }
}

impl std::ops::BitOrAssign for UpdateStatus {
    fn bitor_assign(&mut self, rhs: Self) {
        self.update(rhs)
    }
}

pub type UpdateResult = InterpreterResult<UpdateStatus>;

pub trait Primitive {
    fn exec_comb(&self, _port_map: &mut PortMap) -> UpdateResult {
        Ok(UpdateStatus::Unchanged)
    }

    fn exec_cycle(&mut self, _port_map: &mut PortMap) -> UpdateResult {
        Ok(UpdateStatus::Unchanged)
    }

    fn has_comb(&self) -> bool {
        true
    }

    fn has_stateful(&self) -> bool {
        true
    }

    /// Serialize the state of this primitive, if any.
    fn serialize(&self, _code: Option<PrintCode>) -> Serializable {
        Serializable::Empty
    }

    // more efficient to override this with true in stateful cases
    fn has_serializable_state(&self) -> bool {
        self.serialize(None).has_state()
    }

    fn dump_memory_state(&self) -> Option<Vec<u8>> {
        None
    }
}

/// An empty primitive implementation used for testing. It does not do anything
/// and has no ports of any kind
pub struct DummyPrimitive;

impl DummyPrimitive {
    pub fn new_dyn() -> Box<dyn Primitive> {
        Box::new(Self)
    }
}

impl Primitive for DummyPrimitive {}
