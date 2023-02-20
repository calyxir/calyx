use super::indexed_map::IndexedMap;
use crate::{
    flatten::{
        flat_ir::{
            base::{
                ComponentRef, GlobalCellRef, GlobalPortRef, GlobalRCellRef,
            },
            prelude::Identifier,
        },
        primitives::Primitive,
    },
    values::Value,
};

pub(crate) type PortMap = IndexedMap<GlobalPortRef, Value>;
pub(crate) type CellMap = IndexedMap<GlobalCellRef, CellLedger>;
pub(crate) type RefCellMap = IndexedMap<GlobalRCellRef, Option<GlobalCellRef>>;

pub(crate) enum CellLedger {
    Primitive {
        name: Identifier,
        // wish there was a better option with this one
        cell_dyn: Box<dyn Primitive>,
    },
    Component {
        name: Identifier,
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
