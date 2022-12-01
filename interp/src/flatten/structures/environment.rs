use super::index_trait::{impl_index, IndexRef};
use super::indexed_map::IndexedMap;
use crate::{interpreter_ir::Component, values::Value};

impl_index!(PortRef);
impl_index!(ComponentRef, u16);

type PortMap = IndexedMap<Value, PortRef>;
type ComponentMap = IndexedMap<Component, ComponentRef>;
