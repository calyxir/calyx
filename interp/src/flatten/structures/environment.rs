use super::indexed_map::IndexedMap;
use crate::{
    flatten::{
        flat_ir::{
            base::{ComponentRef, GlobalCellId, GlobalPortId, GlobalRefCellId},
            prelude::Identifier,
        },
        primitives::Primitive,
    },
    values::Value,
};

pub(crate) type PortMap = IndexedMap<GlobalPortId, Value>;
pub(crate) type CellMap = IndexedMap<GlobalCellId, CellLedger>;
pub(crate) type RefCellMap = IndexedMap<GlobalRefCellId, Option<GlobalCellId>>;

pub(crate) enum CellLedger {
    Primitive {
        name: Identifier,
        // wish there was a better option with this one
        cell_dyn: Box<dyn Primitive>,
    },
    Component {
        name: Identifier,
        port_base_offset: GlobalPortId,
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
