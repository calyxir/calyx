use crate::{
    errors::InterpreterResult,
    flatten::{
        flat_ir::prelude::{
            CellDefinitionIdx, GlobalCellIdx, GlobalPortIdx, GlobalRefCellIdx,
            GlobalRefPortIdx, RefCellDefinitionIdx,
        },
        structures::context::Context,
    },
};
use thiserror::Error;

use super::Environment;

#[derive(Error)]
pub enum TraversalError {
    #[error("unable to locate entity: {0}")]
    Target(String),

    #[error("unable to locate cell: {0}")]
    Cell(String),

    #[error("ref cell is not instantiated")]
    UninstantiatedRef(GlobalRefCellIdx),
}

impl std::fmt::Debug for TraversalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

pub struct CellTargetInfo {
    pub cell: GlobalCellIdx,
    pub parent: GlobalCellIdx,
}

pub struct RefCellTargetInfo {
    pub cell: GlobalRefCellIdx,
    pub parent: GlobalCellIdx,
}

pub struct PortTargetInfo {
    pub port: GlobalPortIdx,
    pub parent: GlobalCellIdx,
}

pub enum TraversalEnd {
    Root,
    Cell(CellTargetInfo),
    Port(PortTargetInfo),
    RefCell(RefCellTargetInfo),
    RefPort(GlobalRefPortIdx),
}

pub struct Traverser {
    pub current_component: GlobalCellIdx,
}

impl Traverser {
    pub fn new() -> Self {
        Self {
            current_component: Environment::<&Context>::get_root(),
        }
    }

    pub fn next_cell<C: AsRef<Context> + Clone, S: AsRef<str>>(
        &mut self,
        env: &Environment<C>,
        target: S,
    ) -> Result<(), TraversalError> {
        let current_comp_ledger =
            env.cells[self.current_component].as_comp().unwrap();
        let comp_def = &env.ctx.as_ref().secondary[current_comp_ledger.comp_id];

        // check cells
        for (offset, def_idx) in comp_def.cell_offset_map.iter() {
            let def = &env.ctx.as_ref().secondary[*def_idx];
            if env.ctx.as_ref().lookup_name(def.name) == target.as_ref() {
                self.current_component =
                    &current_comp_ledger.index_bases + offset;
                return Ok(());
            }
        }

        // check ref cells
        for (offset, def_idx) in comp_def.ref_cell_offset_map.iter() {
            let def = &env.ctx.as_ref().secondary[*def_idx];
            if env.ctx.as_ref().lookup_name(def.name) == target.as_ref() {
                let global_offset = &current_comp_ledger.index_bases + offset;
                if let Some(cell) = &env.ref_cells[global_offset] {
                    self.current_component = *cell;
                    return Ok(());
                } else {
                    return Err(TraversalError::UninstantiatedRef(
                        global_offset,
                    ));
                }
            }
        }

        Err(TraversalError::Cell(target.as_ref().to_string()))
    }

    pub fn last_step<C: AsRef<Context> + Clone, S: AsRef<str>>(
        &self,
        env: &Environment<C>,
        target: S,
    ) -> Result<TraversalEnd, TraversalError> {
        let current_comp_ledger =
            env.cells[self.current_component].as_comp().unwrap();
        let comp_def = &env.ctx.as_ref().secondary[current_comp_ledger.comp_id];

        // check cells
        for (offset, def_idx) in comp_def.cell_offset_map.iter() {
            let def = &env.ctx.as_ref().secondary[*def_idx];
            if env.ctx.as_ref().lookup_name(def.name) == target.as_ref() {
                return Ok(TraversalEnd::Cell(CellTargetInfo {
                    cell: &current_comp_ledger.index_bases + offset,
                    parent: self.current_component,
                }));
            }

            // check ports
            for port in def.ports.iter() {
                if env.ctx.as_ref().lookup_name(
                    env.ctx.as_ref().secondary[comp_def.port_offset_map[port]]
                        .name,
                ) == target.as_ref()
                {
                    let port_idx = &current_comp_ledger.index_bases + port;
                    return Ok(TraversalEnd::Port(PortTargetInfo {
                        port: port_idx,
                        parent: &current_comp_ledger.index_bases + offset,
                    }));
                }
            }
        }

        // check ref cells
        for (offset, def_idx) in comp_def.ref_cell_offset_map.iter() {
            let def = &env.ctx.as_ref().secondary[*def_idx];
            if env.ctx.as_ref().lookup_name(def.name) == target.as_ref() {
                let global_offset = &current_comp_ledger.index_bases + offset;

                return Ok(TraversalEnd::RefCell(RefCellTargetInfo {
                    cell: global_offset,
                    parent: self.current_component,
                }));
            }

            // check ports
            for port in def.ports.iter() {
                if env.ctx.as_ref().lookup_name(
                    env.ctx.as_ref().secondary
                        [comp_def.ref_port_offset_map[port]],
                ) == target.as_ref()
                {
                    let port_idx = &current_comp_ledger.index_bases + port;
                    return Ok(TraversalEnd::RefPort(port_idx));
                }
            }
        }

        Err(TraversalError::Target(target.as_ref().to_string()))
    }
}
