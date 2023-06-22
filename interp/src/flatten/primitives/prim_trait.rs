use crate::{
    flatten::{flat_ir::base::GlobalPortId, structures::environment::PortMap},
    values::Value,
};

// Placeholder
pub type PortResults = Vec<(GlobalPortId, Value)>;

pub trait Primitive {
    fn exec_comb_paths(&self, portmap: &PortMap) -> PortResults;
    fn exec_stateful_paths(&mut self, portmap: &PortMap) -> PortResults;
}

pub struct DummyPrimitive;

impl DummyPrimitive {
    pub fn new_dyn() -> Box<dyn Primitive> {
        Box::new(Self)
    }
}

impl Primitive for DummyPrimitive {
    fn exec_comb_paths(&self, portmap: &PortMap) -> PortResults {
        todo!()
    }

    fn exec_stateful_paths(&mut self, portmap: &PortMap) -> PortResults {
        todo!()
    }
}
