use itertools::Itertools;

use super::{context::Context, indexed_map::IndexedMap};
use crate::{
    flatten::{
        flat_ir::prelude::{
            BaseIndices, ComponentIdx, GlobalCellId, GlobalPortId,
            GlobalRefCellId, GlobalRefPortId,
        },
        primitives::{self, Primitive},
        structures::index_trait::IndexRef,
    },
    values::Value,
};
use std::fmt::Debug;

pub(crate) type PortMap = IndexedMap<GlobalPortId, Value>;
pub(crate) type CellMap = IndexedMap<GlobalCellId, CellLedger>;
pub(crate) type RefCellMap = IndexedMap<GlobalRefCellId, Option<GlobalCellId>>;
pub(crate) type RefPortMap = IndexedMap<GlobalRefPortId, Option<GlobalPortId>>;

pub(crate) struct ComponentLedger {
    pub(crate) index_bases: BaseIndices,
    pub(crate) comp_id: ComponentIdx,
}

pub(crate) enum CellLedger {
    Primitive {
        // wish there was a better option with this one
        cell_dyn: Box<dyn Primitive>,
    },
    Component(ComponentLedger),
}

impl CellLedger {
    fn comp(idx: ComponentIdx, env: &Environment) -> Self {
        Self::Component(ComponentLedger {
            index_bases: BaseIndices::new(
                env.ports.peek_next_idx(),
                (env.cells.peek_next_idx().index() + 1).into(),
                env.ref_cells.peek_next_idx(),
                env.ref_ports.peek_next_idx(),
            ),
            comp_id: idx,
        })
    }

    pub fn as_comp(&self) -> Option<&ComponentLedger> {
        match self {
            Self::Component(comp) => Some(comp),
            _ => None,
        }
    }
}

impl Debug for CellLedger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primitive { .. } => f.debug_struct("Primitive").finish(),
            Self::Component(ComponentLedger {
                index_bases,
                comp_id,
            }) => f
                .debug_struct("Component")
                .field("index_bases", index_bases)
                .field("comp_id", comp_id)
                .finish(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ProgramCounter {
    // TODO
}

#[derive(Debug)]
pub struct Environment<'a> {
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

    /// The immutable context. This is retained for ease of use.
    ctx: &'a Context,
}

impl<'a> Environment<'a> {
    pub fn new(ctx: &'a Context) -> Self {
        let root = ctx.entry_point;
        let aux = &ctx.secondary[root];

        let mut env = Self {
            ports: PortMap::with_capacity(aux.port_offset_map.count()),
            cells: CellMap::with_capacity(aux.cell_offset_map.count()),
            ref_cells: RefCellMap::with_capacity(
                aux.ref_cell_offset_map.count(),
            ),
            ref_ports: RefPortMap::with_capacity(
                aux.ref_port_offset_map.count(),
            ),
            pcs: ProgramCounter {},
            ctx,
        };

        let root_node = CellLedger::comp(root, &env);
        let root = env.cells.push(root_node);
        env.layout_component(root);

        env
    }

    fn layout_component(&mut self, comp: GlobalCellId) {
        let ComponentLedger {
            index_bases,
            comp_id,
        } = self.cells[comp]
            .as_comp()
            .expect("Called layout component with a non-component cell.");
        let comp_aux = &self.ctx.secondary[*comp_id];

        let comp_id = *comp_id;

        // first layout the signature
        for sig_port in comp_aux.signature.iter() {
            let width = self.ctx.lookup_port_def(&comp_id, sig_port).width;
            let idx = self.ports.push(Value::zeroes(width));
            debug_assert_eq!(index_bases + sig_port, idx);
        }
        // second group ports
        for group_idx in comp_aux.definitions.groups() {
            // TODO Griffin: The sanity checks here are assuming that go & done
            // are defined in that order. This could break if the IR changes the
            // order on hole ports in groups.

            //go
            let go = self.ports.push(Value::bit_low());
            debug_assert_eq!(go, index_bases + self.ctx.primary[group_idx].go);

            //done
            let done = self.ports.push(Value::bit_low());
            debug_assert_eq!(
                done,
                index_bases + self.ctx.primary[group_idx].done
            );
        }

        for (cell_off, def_idx) in comp_aux.cell_offset_map.iter() {
            let info = &self.ctx.secondary[*def_idx];
            if !info.prototype.is_component() {
                for port in info.ports.iter() {
                    let width = self.ctx.lookup_port_def(&comp_id, port).width;
                    let idx = self.ports.push(Value::zeroes(width));
                    debug_assert_eq!(
                        &self.cells[comp].as_comp().unwrap().index_bases + port,
                        idx
                    );
                }
                let cell_dyn = primitives::build_primitive(info, self);
                let cell = self.cells.push(CellLedger::Primitive { cell_dyn });

                debug_assert_eq!(
                    &self.cells[comp].as_comp().unwrap().index_bases + cell_off,
                    cell
                );
            } else {
                let child_comp = info.prototype.as_component().unwrap();
                let child_comp = CellLedger::comp(*child_comp, self);

                let cell = self.cells.push(child_comp);
                debug_assert_eq!(
                    &self.cells[comp].as_comp().unwrap().index_bases + cell_off,
                    cell
                );

                self.layout_component(cell);
            }
        }

        // ref cells and ports are initialized to None
        for (ref_cell, def_idx) in comp_aux.ref_cell_offset_map.iter() {
            let info = &self.ctx.secondary[*def_idx];
            for port_idx in info.ports.iter() {
                let port_actual = self.ref_ports.push(None);
                debug_assert_eq!(
                    &self.cells[comp].as_comp().unwrap().index_bases + port_idx,
                    port_actual
                )
            }
            let cell_actual = self.ref_cells.push(None);
            debug_assert_eq!(
                &self.cells[comp].as_comp().unwrap().index_bases + ref_cell,
                cell_actual
            )
        }
    }

    pub fn print_env(&self) {
        let root_idx = GlobalCellId::new(0);
        let mut hierarchy = Vec::new();
        self.print_component(root_idx, &mut hierarchy)
    }

    fn print_component(
        &self,
        target: GlobalCellId,
        hierarchy: &mut Vec<GlobalCellId>,
    ) {
        let info = self.cells[target].as_comp().unwrap();
        let comp = &self.ctx.secondary[info.comp_id];
        hierarchy.push(target);

        // This funky iterator chain first pulls the first element (the
        // entrypoint) and extracts its name. Subsequent element are pairs of
        // global offsets produced by a staggered iteration, yielding `(root,
        // child)` then `(child, grandchild)` and so on. All the strings are
        // finally collected and concatenated with a `.` separator to produce
        // the fully qualified name prefix for the given component instance.
        let name_prefix = hierarchy
            .first()
            .iter()
            .map(|x| {
                let info = self.cells[**x].as_comp().unwrap();
                let prior_comp = &self.ctx.secondary[info.comp_id];
                &self.ctx.secondary[prior_comp.name]
            })
            .chain(hierarchy.iter().zip(hierarchy.iter().skip(1)).map(
                |(l, r)| {
                    let info = self.cells[*l].as_comp().unwrap();
                    let prior_comp = &self.ctx.secondary[info.comp_id];
                    let local_target = r - (&info.index_bases);

                    let def_idx = &prior_comp.cell_offset_map[local_target];

                    let id = &self.ctx.secondary[*def_idx];
                    &self.ctx.secondary[id.name]
                },
            ))
            .join(".");

        for (cell_off, def_idx) in comp.cell_offset_map.iter() {
            let definition = &self.ctx.secondary[*def_idx];

            println!("{}.{}", name_prefix, self.ctx.secondary[definition.name]);
            for port in definition.ports.iter() {
                let definition =
                    &self.ctx.secondary[comp.port_offset_map[port]];
                println!(
                    "    {}: {}",
                    self.ctx.secondary[definition.name],
                    self.ports[&info.index_bases + port]
                );
            }

            if definition.prototype.is_component() {
                let child_target = &info.index_bases + cell_off;
                self.print_component(child_target, hierarchy);
            }
        }

        hierarchy.pop();
    }

    pub fn print_env_stats(&self) {
        println!("Environment Stats:");
        println!("  Ports: {}", self.ports.len());
        println!("  Cells: {}", self.cells.len());
        println!("  Ref Cells: {}", self.ref_cells.len());
        println!("  Ref Ports: {}", self.ref_ports.len());
    }
}
