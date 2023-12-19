use crate::{
    debugger::PrintCode,
    errors::InterpreterResult,
    flatten::{flat_ir::base::GlobalPortIdx, structures::environment::PortMap},
    primitives::Serializable,
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
pub enum UpdateStatus {
    Unchanged,
    Changed,
}

impl UpdateStatus {
    #[inline]
    /// If the status is unchanged and other is changed, updates the status of
    /// self to changed, otherwise does nothing
    pub fn update(&mut self, other: Self) {
        match self {
            UpdateStatus::Unchanged => {
                if let UpdateStatus::Changed = other {
                    *self = UpdateStatus::Changed
                }
            }
            UpdateStatus::Changed => {}
        }
    }

    /// Returns [UpdateStatus::Changed] if either input is Changed otherwise
    /// returns Unchanged
    pub fn either_changed(first: Self, second: Self) -> Self {
        match (first, second) {
            (UpdateStatus::Unchanged, UpdateStatus::Unchanged) => {
                UpdateStatus::Unchanged
            }
            (UpdateStatus::Unchanged, UpdateStatus::Changed)
            | (UpdateStatus::Changed, UpdateStatus::Unchanged)
            | (UpdateStatus::Changed, UpdateStatus::Changed) => {
                UpdateStatus::Changed
            }
        }
    }

    /// Returns `true` if the update status is [`Changed`].
    ///
    /// [`Changed`]: UpdateStatus::Changed
    #[must_use]
    pub fn is_changed(&self) -> bool {
        matches!(self, Self::Changed)
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

    fn reset(&mut self, _port_map: &mut PortMap) -> InterpreterResult<()> {
        Ok(())
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
