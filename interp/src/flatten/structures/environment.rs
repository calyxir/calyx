use symbol_table::{Symbol, SymbolTable};

use super::indexed_map::IndexedMap;
use super::{
    bookkeeping::ComponentRef,
    index_trait::{impl_index, IndexRef},
};
use crate::{interpreter_ir::Component, primitives::Primitive, values::Value};

pub const INPUT_CELL_PORT_COUNT: usize = 8;
pub const OUTPUT_CELL_PORT_COUNT: usize = 4;

// making these all u32 for now, can give the macro an optional type as the
// second arg to contract or expand as needed

// Reference for a port assuming a zero base, ie local to the component
impl_index!(pub LocalPortRef);
// Global port reference, used for value mapping
impl_index!(pub GlobalPortRef);
// Global mapping for cell state
impl_index!(pub GlobalCellRef);
// cell reference local to a given component definition
impl_index!(pub LocalCellRef);
// A local reference
impl_index!(pub CellPortID);
// ref cell index
impl_index!(pub GlobalRCellRef);
impl_index!(pub LocalRCellRef);
impl_index!(pub LocalRPortRef);

#[derive(Debug)]
pub enum PortRef {
    Local(LocalPortRef),
    Ref(LocalRPortRef),
}

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
