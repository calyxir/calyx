use crate::{
    debugger::PrintCode,
    errors::InterpreterResult,
    flatten::{flat_ir::base::GlobalPortId, structures::environment::PortMap},
    primitives::Serializable,
    values::Value,
};

pub struct AssignResult {
    pub destination: GlobalPortId,
    pub value: Value,
}

impl AssignResult {
    pub fn new(destination: GlobalPortId, value: Value) -> Self {
        Self { destination, value }
    }
}

impl From<(GlobalPortId, Value)> for AssignResult {
    fn from(value: (GlobalPortId, Value)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl From<(Value, GlobalPortId)> for AssignResult {
    fn from(value: (Value, GlobalPortId)) -> Self {
        Self::new(value.1, value.0)
    }
}

/// The return value for evaluating the results of a primitive
pub type Results = InterpreterResult<Vec<AssignResult>>;

pub trait Primitive {
    fn exec_comb(&self, _port_map: &PortMap) -> Results {
        Ok(vec![])
    }

    fn exec_cycle(&mut self, _port_map: &PortMap) -> Results {
        Ok(vec![])
    }

    fn reset(&mut self) -> Results {
        Ok(vec![])
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
