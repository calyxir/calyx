use crate::{
    errors::InterpreterResult,
    flatten::{
        flat_ir::prelude::{
            CellDefinitionIdx, CellRef, GlobalCellIdx, GlobalPortIdx,
            GlobalRefCellIdx, GlobalRefPortIdx, PortRef, RefCellDefinitionIdx,
        },
        structures::context::Context,
    },
};
use smallvec::{smallvec, SmallVec};
use thiserror::Error;

use super::Environment;

#[derive(Error)]
pub enum TraversalError {
    #[error("unable to locate port: {0}")]
    Port(String),

    #[error("unable to locate cell: {0}")]
    Cell(String),

    #[error("unable to locate entity: {0}")]
    Unknown(String),
}

impl std::fmt::Debug for TraversalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
#[derive(Debug)]
pub struct Traverser {
    concrete_path: SmallVec<[GlobalCellIdx; 4]>,
    abstract_path: SmallVec<[CellRef; 2]>,
    first_ref: Option<GlobalRefCellIdx>,
}

impl Traverser {
    pub fn new() -> Self {
        Self {
            concrete_path: smallvec![Environment::<&Context>::get_root(),],
            abstract_path: smallvec![],
            first_ref: None,
        }
    }

    pub fn next_cell<C: AsRef<Context> + Clone, S: AsRef<str>>(
        &mut self,
        env: &Environment<C>,
        target: S,
    ) -> Result<(), TraversalError> {
        // case 1: we are a concrete path so far
        if self.first_ref.is_none() {
            // this unwrap is safe
            let current_comp = *self.concrete_path.last().unwrap();

            let current_comp_ledger =
                env.cells[current_comp].as_comp().unwrap();
            let comp_def = &env.ctx().secondary[current_comp_ledger.comp_id];

            // check cells
            for (offset, def_idx) in comp_def.cell_offset_map.iter() {
                let def = &env.ctx().secondary[*def_idx];
                if env.ctx().lookup_name(def.name) == target.as_ref() {
                    self.concrete_path
                        .push(&current_comp_ledger.index_bases + offset);
                    return Ok(());
                }
            }

            // check ref cells
            for (offset, def_idx) in comp_def.ref_cell_offset_map.iter() {
                let def = &env.ctx().secondary[*def_idx];
                if env.ctx().lookup_name(def.name) == target.as_ref() {
                    let global_offset =
                        &current_comp_ledger.index_bases + offset;

                    self.first_ref = Some(global_offset);
                    return Ok(());
                }
            }
            Err(TraversalError::Cell(target.as_ref().to_string()))
        }
        // case 2: we're in abstract territory
        else {
            let current_comp_idx = self.compute_current_comp_def(env);

            let current_comp_def = &env.ctx().secondary[current_comp_idx];
            for (offset, def_idx) in current_comp_def.cell_offset_map.iter() {
                if env.ctx().lookup_name(env.ctx().secondary[*def_idx].name)
                    == target.as_ref()
                {
                    self.abstract_path.push(offset.into());
                    return Ok(());
                }
            }

            for (offset, def_idx) in current_comp_def.ref_cell_offset_map.iter()
            {
                if env.ctx().lookup_name(env.ctx().secondary[*def_idx].name)
                    == target.as_ref()
                {
                    self.abstract_path.push(offset.into());
                    return Ok(());
                }
            }

            Err(TraversalError::Cell(target.as_ref().to_string()))
        }
    }

    fn compute_current_comp_def<C: AsRef<Context> + Clone>(
        &self,
        env: &Environment<C>,
    ) -> crate::flatten::flat_ir::prelude::ComponentIdx {
        let last_comp = *self.concrete_path.last().unwrap();
        let last_comp_ledger = env.cells[last_comp].as_comp().unwrap();
        let last_comp_def = &env.ctx().secondary[last_comp_ledger.comp_id];

        let first_ref_local =
            self.first_ref.unwrap() - &last_comp_ledger.index_bases;
        let first_ref_def = last_comp_def.ref_cell_offset_map[first_ref_local];
        let def = &env.ctx().secondary[first_ref_def];

        let mut current_comp_idx = *def
            .prototype
            .as_component()
            .expect("called next_cell on a primitive ref cell");

        for cell_ref in self.abstract_path.iter() {
            let def = &env.ctx().secondary[current_comp_idx];
            match cell_ref {
                CellRef::Local(l) => {
                    let local_def = def.cell_offset_map[*l];
                    current_comp_idx = *env.ctx().secondary[local_def]
                        .prototype
                        .as_component()
                        .unwrap();
                }
                CellRef::Ref(r) => {
                    let ref_def = def.ref_cell_offset_map[*r];
                    current_comp_idx = *env.ctx().secondary[ref_def]
                        .prototype
                        .as_component()
                        .unwrap();
                }
            }
        }
        current_comp_idx
    }

    pub fn last_step<C: AsRef<Context> + Clone, S: AsRef<str>>(
        mut self,
        env: &Environment<C>,
        target: S,
    ) -> Result<Path, TraversalError> {
        if let Some(first_ref) = self.first_ref {
            let current_comp_idx = if !self.abstract_path.is_empty() {
                self.compute_current_comp_def(env)
            } else {
                let last_comp = *self.concrete_path.last().unwrap();
                let last_comp_ledger = env.cells[last_comp].as_comp().unwrap();
                last_comp_ledger.comp_id
            };

            let current_comp_def = &env.ctx().secondary[current_comp_idx];
            for (offset, def_idx) in current_comp_def.cell_offset_map.iter() {
                if env.ctx().lookup_name(env.ctx().secondary[*def_idx].name)
                    == target.as_ref()
                {
                    self.abstract_path.push(offset.into());
                    return Ok(Path::AbstractCell(LazyCellPath {
                        concrete_prefix: self.concrete_path,
                        first_ref,
                        abstract_suffix: self.abstract_path,
                    }));
                }

                for port in env.ctx().secondary[*def_idx].ports.iter() {
                    let port_def_idx = current_comp_def.port_offset_map[port];
                    let port_name = env.ctx().secondary[port_def_idx].name;
                    if env.ctx().lookup_name(port_name) == target.as_ref() {
                        return Ok(Path::AbstractPort {
                            cell: LazyCellPath {
                                concrete_prefix: self.concrete_path,
                                first_ref,
                                abstract_suffix: self.abstract_path,
                            },
                            port: port.into(),
                        });
                    }
                }
            }

            for (offset, def_idx) in current_comp_def.ref_cell_offset_map.iter()
            {
                if env.ctx().lookup_name(env.ctx().secondary[*def_idx].name)
                    == target.as_ref()
                {
                    self.abstract_path.push(offset.into());
                    return Ok(Path::AbstractCell(LazyCellPath {
                        concrete_prefix: self.concrete_path,
                        first_ref,
                        abstract_suffix: self.abstract_path,
                    }));
                }

                for port in env.ctx().secondary[*def_idx].ports.iter() {
                    let port_def_idx =
                        current_comp_def.ref_port_offset_map[port];
                    let port_name = env.ctx().secondary[port_def_idx];
                    if env.ctx().lookup_name(port_name) == target.as_ref() {
                        return Ok(Path::AbstractPort {
                            cell: LazyCellPath {
                                concrete_prefix: self.concrete_path,
                                first_ref,
                                abstract_suffix: self.abstract_path,
                            },
                            port: port.into(),
                        });
                    }
                }
            }
        } else {
            let current_comp = *self.concrete_path.last().unwrap();

            let current_comp_ledger =
                env.cells[current_comp].as_comp().unwrap_or_else(|| {
                    // we are in a primitive component so need to go up a level
                    env.cells[self.concrete_path[self.concrete_path.len() - 2]]
                        .unwrap_comp()
                });
            let comp_def = &env.ctx().secondary[current_comp_ledger.comp_id];

            // check cells
            for (offset, def_idx) in comp_def.cell_offset_map.iter() {
                let def = &env.ctx().secondary[*def_idx];
                if env.ctx().lookup_name(def.name) == target.as_ref() {
                    return Ok(Path::Cell(
                        &current_comp_ledger.index_bases + offset,
                    ));
                }

                // check ports
                for port in def.ports.iter() {
                    let name = env.ctx().secondary
                        [comp_def.port_offset_map[port]]
                        .name;
                    if env.ctx().lookup_name(name) == target.as_ref() {
                        let port_idx = &current_comp_ledger.index_bases + port;
                        return Ok(Path::Port(port_idx));
                    }
                }
            }

            // check ref cells
            for (offset, def_idx) in comp_def.ref_cell_offset_map.iter() {
                let def = &env.ctx().secondary[*def_idx];
                if env.ctx().lookup_name(def.name) == target.as_ref() {
                    let global_offset =
                        &current_comp_ledger.index_bases + offset;

                    return Ok(Path::AbstractCell(LazyCellPath {
                        concrete_prefix: self.concrete_path,
                        first_ref: global_offset,
                        abstract_suffix: SmallVec::new(),
                    }));
                }

                // we don't check ports here since there can't be ref ports
                // without first having visited a ref cell
            }
        }

        Err(TraversalError::Unknown(target.as_ref().to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct LazyCellPath {
    pub concrete_prefix: SmallVec<[GlobalCellIdx; 4]>,
    pub first_ref: GlobalRefCellIdx,
    pub abstract_suffix: SmallVec<[CellRef; 2]>,
}

#[derive(Debug, Clone)]
pub enum Path {
    Cell(GlobalCellIdx),
    Port(GlobalPortIdx),
    AbstractCell(LazyCellPath),
    AbstractPort { cell: LazyCellPath, port: PortRef },
}

impl Path {
    fn walk_lazy_cell_path<C: AsRef<Context> + Clone>(
        env: &Environment<C>,
        path: &LazyCellPath,
    ) -> Result<GlobalCellIdx, ResolvePathError> {
        if let Some(cell_actual) = env.ref_cells[path.first_ref] {
            let mut current = cell_actual;
            for abstract_cell in path.abstract_suffix.iter() {
                let current_comp_ledger = env.cells[current].as_comp().unwrap();

                match abstract_cell {
                    CellRef::Local(l) => {
                        current = &current_comp_ledger.index_bases + l;
                    }
                    CellRef::Ref(r) => {
                        let ref_global_offset =
                            &current_comp_ledger.index_bases + r;
                        if let Some(ref_actual) =
                            env.ref_cells[ref_global_offset]
                        {
                            current = ref_actual;
                        } else {
                            // todo griffin: improve error message
                            return Err(ResolvePathError(format!(
                                "<PLACEHOLDER {:?}>",
                                ref_global_offset
                            )));
                        }
                    }
                }
            }
            Ok(current)
        } else {
            // todo griffin: improve error message
            Err(ResolvePathError(format!(
                "<PLACEHOLDER {:?}>",
                path.first_ref
            )))
        }
    }

    pub fn resolve_path<C: AsRef<Context> + Clone>(
        &self,
        env: &Environment<C>,
    ) -> Result<PathResolution, ResolvePathError> {
        Ok(match self {
            Path::Cell(c) => PathResolution::Cell(*c),
            Path::Port(p) => PathResolution::Port(*p),
            Path::AbstractCell(c) => {
                PathResolution::Cell(Self::walk_lazy_cell_path(env, c)?)
            }
            Path::AbstractPort { cell, port } => {
                let cell = Self::walk_lazy_cell_path(env, cell)?;
                if let Some(ledger) = env.cells[cell].as_comp() {
                    // we are looking at interface ports here
                    let port_idx = &ledger.index_bases
                        + port
                            .as_local()
                            .expect("path is malformed. This is an error please report it");

                    PathResolution::Port(port_idx)
                } else {
                    let parent = env
                        .get_parent_cell_from_cell(cell)
                        .expect("primitive cell has no parent. This is an error please report it");

                    let ledger = env.cells[parent].as_comp().unwrap();
                    match port {
                        PortRef::Local(l) => {
                            PathResolution::Port(&ledger.index_bases + l)
                        }
                        PortRef::Ref(r) => {
                            let ref_global_offset = &ledger.index_bases + r;
                            let ref_actual = env.ref_ports[ref_global_offset].expect("ref port is undefined but parent cell is not. This should never happen. This is an error please report it");

                            PathResolution::Port(ref_actual)
                        }
                    }
                }
            }
        })
    }

    pub fn as_string<C: AsRef<Context> + Clone>(
        &self,
        env: &Environment<C>,
    ) -> String {
        match self {
            Path::Cell(c) => env.get_full_name(c),
            Path::Port(p) => env.get_full_name(p),
            Path::AbstractCell(c) => todo!("ref cells not supported yet"),
            Path::AbstractPort { cell, port } => {
                todo!("ref ports not supported yet")
            }
        }
    }
}

pub enum PathResolution {
    Cell(GlobalCellIdx),
    Port(GlobalPortIdx),
}

impl PathResolution {
    #[must_use]
    pub fn as_cell(&self) -> Option<&GlobalCellIdx> {
        if let Self::Cell(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_port(&self) -> Option<&GlobalPortIdx> {
        if let Self::Port(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Debug, Error)]
#[error("Could not resolve path. Ref cell {0} is undefined")]
pub struct ResolvePathError(String);

#[derive(Debug, Error)]
pub enum PathError {
    #[error(transparent)]
    ResolvePathError(#[from] ResolvePathError),

    #[error(transparent)]
    TraversalError(#[from] TraversalError),
}
