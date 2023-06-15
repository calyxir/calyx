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
    pcs: ProgramCounter,
    ports: PortMap,
    cells: CellMap,
    ref_cells: RefCellMap,
}
