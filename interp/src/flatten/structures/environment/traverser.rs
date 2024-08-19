use crate::flatten::{
    flat_ir::{
        base::ComponentIdx,
        cell_prototype::CellPrototype,
        prelude::{
            CellRef, GlobalCellIdx, GlobalPortIdx, GlobalRefCellIdx, PortRef,
        },
    },
    structures::context::Context,
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
            .expect("called compute_current_comp_def on a primitive ref cell");

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
            let (outer_comp, last_cell): (ComponentIdx, CellRef) = if !self
                .abstract_path
                .is_empty()
            {
                let current_comp_ledger = env.cells
                    [*self.concrete_path.last().unwrap()]
                .as_comp()
                .unwrap();

                let ref_offset = first_ref - &current_comp_ledger.index_bases;
                let mut current_comp_id = *env
                    .get_def_info_ref(current_comp_ledger.comp_id, ref_offset)
                    .prototype
                    .as_component()
                    .unwrap();

                // iterate over the cells which must be components
                for abstract_cell in
                    self.abstract_path.iter().take(self.abstract_path.len() - 1)
                {
                    match abstract_cell {
                        CellRef::Local(l) => {
                            let info = env.get_def_info(current_comp_id, *l);
                            current_comp_id =
                                *info.prototype.as_component().unwrap();
                        }
                        CellRef::Ref(r) => {
                            let info =
                                env.get_def_info_ref(current_comp_id, *r);
                            current_comp_id =
                                *info.prototype.as_component().unwrap();
                        }
                    }
                }

                let last_ref = self.abstract_path.last().unwrap();
                (current_comp_id, *last_ref)
            } else {
                let last_comp_idx = *self.concrete_path.last().unwrap();
                let last_comp_ledger =
                    env.cells[last_comp_idx].as_comp().unwrap();
                let local_offset = first_ref - &last_comp_ledger.index_bases;
                (last_comp_ledger.comp_id, local_offset.into())
            };

            let current_comp_idx = match last_cell {
                CellRef::Local(l) => {
                    let info = env.get_def_info(outer_comp, l);

                    // first check component ports for the target
                    for offset in info.ports.iter() {
                        let def_info =
                            env.get_port_def_info(outer_comp, offset);
                        if env.ctx().lookup_name(def_info.name)
                            == target.as_ref()
                        {
                            return Ok(Path::AbstractPort {
                                cell: LazyCellPath {
                                    concrete_prefix: self.concrete_path,
                                    first_ref,
                                    abstract_suffix: self.abstract_path,
                                },
                                port: offset.into(),
                            });
                        }
                    }

                    // it's not a port
                    if let Some(cell) = info.prototype.as_component() {
                        *cell
                    } else {
                        return Err(TraversalError::Unknown(
                            target.as_ref().to_string(),
                        ));
                    }
                }
                CellRef::Ref(r) => {
                    let info = env.get_def_info_ref(outer_comp, r);

                    // first check component ports for the target
                    for offset in info.ports.iter() {
                        let name =
                            env.get_port_def_info_ref(outer_comp, offset);
                        if env.ctx().lookup_name(name) == target.as_ref()
                            && info.prototype.is_component()
                        {
                            let comp = info.prototype.as_component().unwrap();
                            let sig = env.ctx().secondary[*comp].signature();
                            for port in sig {
                                let port_def =
                                    env.get_port_def_info(*comp, port);
                                if env.ctx().lookup_name(port_def.name)
                                    == target.as_ref()
                                {
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
                            unreachable!("port exists on cell but not in signature. Internal structure is in an inconsistent state. This is an error please report it");
                        } else if env.ctx().lookup_name(name) == target.as_ref()
                        {
                            return Ok(Path::AbstractPort {
                                cell: LazyCellPath {
                                    concrete_prefix: self.concrete_path,
                                    first_ref,
                                    abstract_suffix: self.abstract_path,
                                },
                                port: offset.into(),
                            });
                        }
                    }

                    // it's not a port
                    if let Some(cell) = info.prototype.as_component() {
                        *cell
                    } else {
                        return Err(TraversalError::Unknown(
                            target.as_ref().to_string(),
                        ));
                    }
                }
            };

            // Check cells
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
            }

            // check ref_cells
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

            // check ports
            if current_comp == *self.concrete_path.last().unwrap() {
                for port in
                    env.ctx().secondary[current_comp_ledger.comp_id].signature()
                {
                    let def_idx = env.ctx().secondary
                        [current_comp_ledger.comp_id]
                        .port_offset_map[port];
                    let port_name = env.ctx().secondary[def_idx].name;
                    if env.ctx().lookup_name(port_name) == target.as_ref() {
                        let port = &current_comp_ledger.index_bases + port;
                        return Ok(Path::Port(port));
                    }
                }
            } else {
                let local_offset = *self.concrete_path.last().unwrap()
                    - &current_comp_ledger.index_bases;
                let cell_def_idx = &env.ctx().secondary
                    [current_comp_ledger.comp_id]
                    .cell_offset_map[local_offset];
                let cell_def = &env.ctx().secondary[*cell_def_idx];
                for port in cell_def.ports.iter() {
                    let port_def_idx = env.ctx().secondary
                        [current_comp_ledger.comp_id]
                        .port_offset_map[port];
                    let port_name = env.ctx().secondary[port_def_idx].name;

                    if env.ctx().lookup_name(port_name) == target.as_ref() {
                        return Ok(Path::Port(
                            &current_comp_ledger.index_bases + port,
                        ));
                    }
                }
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

impl LazyCellPath {
    pub fn terminal_comp<C: AsRef<Context> + Clone>(
        &self,
        env: &Environment<C>,
    ) -> ComponentIdx {
        let last = self.concrete_prefix.last().unwrap();
        let ledger = env.cells[*last].as_comp().unwrap();
        let offset = self.first_ref - &ledger.index_bases;
        let cell_idx =
            env.ctx().secondary[ledger.comp_id].ref_cell_offset_map[offset];
        let mut current_comp = *env.ctx().secondary[cell_idx]
            .prototype
            .as_component()
            .unwrap_or(&ledger.comp_id);

        for cell_ref in self.abstract_suffix.iter() {
            match cell_ref {
                CellRef::Local(l) => {
                    let local_def =
                        env.ctx().secondary[current_comp].cell_offset_map[*l];

                    if let CellPrototype::Component(c) =
                        env.ctx().secondary[local_def].prototype
                    {
                        current_comp = c;
                    }
                }
                CellRef::Ref(r) => {
                    let ref_def = env.ctx().secondary[current_comp]
                        .ref_cell_offset_map[*r];
                    if let CellPrototype::Component(c) =
                        env.ctx().secondary[ref_def].prototype
                    {
                        current_comp = c;
                    }
                }
            }
        }
        current_comp
    }

    pub fn as_string<C: AsRef<Context> + Clone>(
        &self,
        env: &Environment<C>,
    ) -> String {
        let last = self.concrete_prefix.last().unwrap();
        let mut path = env.get_full_name(last);
        let ledger = env.cells[*last].as_comp().unwrap();
        let offset = self.first_ref - &ledger.index_bases;
        let cell_idx =
            env.ctx().secondary[ledger.comp_id].ref_cell_offset_map[offset];
        path.push('.');
        path.push_str(
            env.ctx().lookup_name(env.ctx().secondary[cell_idx].name),
        );

        let mut current_comp = *env.ctx().secondary[cell_idx]
            .prototype
            .as_component()
            .unwrap_or(&ledger.comp_id);

        for cell_ref in self.abstract_suffix.iter() {
            match cell_ref {
                CellRef::Local(l) => {
                    let local_def =
                        env.ctx().secondary[current_comp].cell_offset_map[*l];
                    path.push('.');
                    path.push_str(
                        env.ctx()
                            .lookup_name(env.ctx().secondary[local_def].name),
                    );
                    if let CellPrototype::Component(c) =
                        env.ctx().secondary[local_def].prototype
                    {
                        current_comp = c;
                    }
                }
                CellRef::Ref(r) => {
                    let ref_def = env.ctx().secondary[current_comp]
                        .ref_cell_offset_map[*r];
                    path.push('.');
                    path.push_str(
                        env.ctx()
                            .lookup_name(env.ctx().secondary[ref_def].name),
                    );
                    if let CellPrototype::Component(c) =
                        env.ctx().secondary[ref_def].prototype
                    {
                        current_comp = c;
                    }
                }
            }
        }

        path
    }
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
                            return Err(ResolvePathError(
                                env.get_full_name(ref_global_offset),
                            ));
                        }
                    }
                }
            }
            Ok(current)
        } else {
            // todo griffin: improve error message
            Err(ResolvePathError(env.get_full_name(path.first_ref)))
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
            Path::AbstractCell(path) => path.as_string(env),
            Path::AbstractPort { cell, port } => {
                let mut path = cell.as_string(env);
                let comp = cell.terminal_comp(env);
                match port {
                    PortRef::Local(l) => {
                        let idx = env.ctx().secondary[comp].port_offset_map[*l];
                        path.push('.');
                        path.push_str(
                            env.ctx()
                                .lookup_name(env.ctx().secondary[idx].name),
                        );
                    }
                    PortRef::Ref(r) => {
                        let idx =
                            env.ctx().secondary[comp].ref_port_offset_map[*r];
                        path.push('.');
                        path.push_str(
                            env.ctx().lookup_name(env.ctx().secondary[idx]),
                        );
                    }
                }

                path
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
