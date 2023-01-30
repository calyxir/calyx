use symbol_table::{Symbol, SymbolTable};

use super::index_trait::{impl_index, IndexRef};
use super::indexed_map::IndexedMap;
use crate::{
    flatten::{
        flat_ir::base::{
            ComponentRef, GlobalCellRef, GlobalPortRef, GlobalRCellRef,
        },
        primitives::Primitive,
    },
    interpreter_ir::Component,
    values::Value,
};

pub(crate) type PortMap = IndexedMap<Value, GlobalPortRef>;
pub(crate) type CellMap = IndexedMap<CellLedger, GlobalCellRef>;
pub(crate) type RefCellMap = IndexedMap<Option<GlobalCellRef>, GlobalRCellRef>;

pub(crate) enum CellLedger {
    Primitive {
        name: Symbol,
        // wish there was a better option with this one
        cell_dyn: Box<dyn Primitive>,
    },
    Component {
        name: Symbol,
        port_base_offset: GlobalPortRef,
        comp_id: ComponentRef,
    },
}

pub(crate) struct ProgramCounter {
    // TODO
}

pub struct Environment {
    pcs: ProgramCounter,
    ports: PortMap,
    cells: CellMap,
    ref_cells: RefCellMap,
}
