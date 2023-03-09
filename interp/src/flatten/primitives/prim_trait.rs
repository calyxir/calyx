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
