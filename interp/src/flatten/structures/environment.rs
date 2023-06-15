use super::indexed_map::IndexedMap;
use crate::{
    flatten::{
        flat_ir::prelude::{
            BaseIndices, ComponentIdx, GlobalCellId, GlobalPortId,
            GlobalRefCellId, GlobalRefPortId,
        },
        primitives::Primitive,
    },
    values::Value,
};

pub(crate) type PortMap = IndexedMap<GlobalPortId, Value>;
pub(crate) type CellMap = IndexedMap<GlobalCellId, CellLedger>;
pub(crate) type RefCellMap = IndexedMap<GlobalRefCellId, Option<GlobalCellId>>;
pub(crate) type RefPortMap = IndexedMap<GlobalRefPortId, Option<GlobalPortId>>;

pub(crate) enum CellLedger {
    Primitive {
        // wish there was a better option with this one
        cell_dyn: Box<dyn Primitive>,
    },
    Component {
        index_bases: BaseIndices,
        comp_id: ComponentIdx,
    },
}

pub(crate) struct ProgramCounter {
    // TODO
}

pub struct Environment {
    /// A map from global port IDs to their current values.
    ports: PortMap,
    /// A map from global cell IDs to their current state and execution info.
    cells: CellMap,
    /// A map from global ref cell IDs to the cell they reference, if any.
    ref_cells: RefCellMap,
    /// A map from global ref port IDs to the port they reference, if any.
    ref_ports: RefPortMap,

    /// The program counter for the whole program execution.
    pcs: ProgramCounter,
}
