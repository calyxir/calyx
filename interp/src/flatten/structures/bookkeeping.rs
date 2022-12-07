use symbol_table::SymbolTable;

use crate::interpreter_ir::Component;

use super::{index_trait::impl_index, indexed_map::IndexedMap};

// Identifier for component definition
impl_index!(pub(crate) ComponentRef);
pub(crate) type ComponentMap = IndexedMap<Component, ComponentRef>;
pub struct InterpretationContext {
    components: ComponentMap,
    // maybe this is overkill for our purposes
    string_table: SymbolTable,
    // something else should be here but I don't remember what
}
