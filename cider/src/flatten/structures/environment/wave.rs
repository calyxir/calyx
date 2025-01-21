// Copyright 2024 Cornell University
// released under MIT License
// author: Kevin Laeufer <laeufer@cornell.edu>

use crate::flatten::flat_ir::cell_prototype::CellPrototype;
use crate::flatten::flat_ir::prelude::*;
use crate::flatten::structures::context::Context;
use crate::flatten::structures::environment::{Environment, PortMap};

use baa::BitVecOps;
use cider_idx::maps::SecondaryMap;
use fst_writer::*;

#[derive(Debug, thiserror::Error)]
pub enum WaveError {
    #[error("FST write operation failed.")]
    Fst(#[from] fst_writer::FstWriteError),
}

pub type Result<T> = std::result::Result<T, WaveError>;

impl From<WaveError> for crate::errors::CiderError {
    fn from(value: WaveError) -> Self {
        Self::GenericError(value.to_string())
    }
}

pub struct WaveWriter {
    // `writer` will be `None` after this struct is dropped.
    writer: Option<FstBodyWriter<std::io::BufWriter<std::fs::File>>>,
    port_map: PortToSignalMap,
}

impl WaveWriter {
    pub fn open<C: AsRef<Context> + Clone>(
        file_path: &std::path::PathBuf,
        env: &Environment<C>,
    ) -> Result<Self> {
        let info = FstInfo {
            start_time: 0,
            timescale_exponent: 0,
            version: "Cider 2".to_string(),
            date: "today".to_string(),
            file_type: FstFileType::Verilog,
        };
        let mut writer = open_fst(file_path, &info)?;
        let port_map = declare_signals(&mut writer, env)?;
        let writer = writer.finish()?;
        Ok(Self {
            writer: Some(writer),
            port_map,
        })
    }

    pub fn write_values(&mut self, time: u64, values: &PortMap) -> Result<()> {
        let writer = self.writer.as_mut().unwrap();
        writer.time_change(time)?;
        for (port_id, maybe_signal_id) in self.port_map.iter() {
            if let Some(signal_id) = maybe_signal_id {
                match values[port_id].val() {
                    None => {
                        writer.signal_change(*signal_id, "x".as_bytes())?;
                    }
                    Some(value) => {
                        writer.signal_change(
                            *signal_id,
                            value.to_bit_str().as_bytes(),
                        )?;
                    }
                }
            }
        }
        Ok(())
    }
}

impl Drop for WaveWriter {
    fn drop(&mut self) {
        let writer = std::mem::take(&mut self.writer);
        if let Some(writer) = writer {
            writer.finish().unwrap();
        }
    }
}

enum Todo {
    /// instance name, instance id
    OpenScope(String, GlobalCellIdx),
    CloseScope,
}

type PortToSignalMap = SecondaryMap<GlobalPortIdx, Option<FstSignalId>>;

fn declare_signals<
    C: AsRef<Context> + Clone,
    W: std::io::Write + std::io::Seek,
>(
    out: &mut FstHeaderWriter<W>,
    env: &Environment<C>,
) -> Result<PortToSignalMap> {
    let ctx = env.ctx();
    let mut port_map: PortToSignalMap = PortToSignalMap::new();
    let root_idx = Environment::<C>::get_root();
    let root_comp = &ctx.secondary[ctx.entry_point];
    let root_name = ctx.lookup_name(root_comp.name).clone();
    let mut todo = vec![Todo::OpenScope(root_name, root_idx)];
    while let Some(component) = todo.pop() {
        match component {
            Todo::OpenScope(name, id) => {
                todo.push(Todo::CloseScope);
                declare_component(
                    out,
                    env,
                    &mut todo,
                    &mut port_map,
                    id,
                    name,
                )?;
            }
            Todo::CloseScope => {
                out.up_scope()?;
            }
        }
    }
    Ok(port_map)
}

fn declare_component<
    C: AsRef<Context> + Clone,
    W: std::io::Write + std::io::Seek,
>(
    out: &mut FstHeaderWriter<W>,
    env: &Environment<C>,
    todo: &mut Vec<Todo>,
    port_map: &mut PortToSignalMap,
    component_cell_idx: GlobalCellIdx,
    instance_name: String,
) -> Result<()> {
    let ctx = env.ctx();
    let instance = env.cells[component_cell_idx].as_comp().unwrap();
    let component = &ctx.secondary[instance.comp_id];

    let component_name = ctx.lookup_name(component.name);
    out.scope(instance_name, component_name, FstScopeType::Module)?;

    // component ports
    declare_ports(
        out,
        env,
        component.inputs().chain(component.outputs()).map(|local| {
            (
                component.port_offset_map[local],
                &instance.index_bases + local,
            )
        }),
        port_map,
    )?;

    // child components
    for (local_offset, cell) in component.cell_offset_map.iter() {
        let cell = &ctx.secondary.local_cell_defs[*cell];
        let cell_idx = &instance.index_bases + local_offset;
        if cell.prototype.is_component() {
            let name = ctx.lookup_name(cell.name).clone();
            todo.push(Todo::OpenScope(name, cell_idx));
        } else {
            if matches!(&cell.prototype, CellPrototype::Constant { .. }) {
                // skip constants
                continue;
            }
            let instance_name = ctx.lookup_name(cell.name);
            let primitive_name = ""; // TODO
            out.scope(instance_name, primitive_name, FstScopeType::Module)?;
            declare_ports(
                out,
                env,
                cell.ports.iter().map(|local| {
                    (
                        component.port_offset_map[local],
                        &instance.index_bases + local,
                    )
                }),
                port_map,
            )?;
            out.up_scope()?; // primitives do not have any children
        }
    }
    Ok(())
}

fn declare_ports<
    C: AsRef<Context> + Clone,
    I: Iterator<Item = (PortDefinitionIdx, GlobalPortIdx)>,
    W: std::io::Write + std::io::Seek,
>(
    out: &mut FstHeaderWriter<W>,
    env: &Environment<C>,
    ports: I,
    port_map: &mut PortToSignalMap,
) -> Result<()> {
    let ctx = env.ctx();
    for (port_id, global_idx) in ports {
        let port = ctx.secondary.local_port_defs.get(port_id).unwrap();
        let name = ctx.lookup_name(port.name);

        let alias = *port_map.get(global_idx);
        let signal_type = FstSignalType::bit_vec(port.width as u32);
        let id = out.var(
            name,
            signal_type,
            FstVarType::Logic,
            FstVarDirection::Implicit,
            alias,
        )?;
        if alias.is_none() {
            port_map.insert(global_idx, Some(id));
        }
    }
    Ok(())
}
