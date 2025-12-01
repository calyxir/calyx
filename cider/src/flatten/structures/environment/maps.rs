use baa::BitVecValue;
use cider_idx::{
    IndexRef,
    iter::IndexRange,
    maps::{IndexedMap, SemiContiguousSecondaryMap},
};
use std::fmt::Debug;
use std::{collections::HashMap, ops::Index};

use crate::{
    errors::{ConflictingAssignments, RuntimeError, RuntimeResult},
    flatten::{
        flat_ir::indexes::{
            AssignedValue, AssignmentIdx, AssignmentWinner, BaseIndices,
            CellRef, ComponentIdx, GlobalCellIdx, GlobalCellRef, GlobalPortIdx,
            GlobalPortRef, GlobalRefCellIdx, GlobalRefPortIdx, MemoryLocation,
            MemoryRegion, PortRef, PortValue,
        },
        primitives::{
            Primitive,
            prim_trait::{RaceDetectionPrimitive, UpdateStatus},
        },
        structures::context::Context,
    },
};

use super::{
    Environment,
    clock::{ClockMap, ClockPair, new_clock_pair},
};

#[derive(Debug, Clone)]
pub struct PortMap(IndexedMap<GlobalPortIdx, PortValue>);

impl PortMap {
    pub fn with_capacity(size: usize) -> Self {
        Self(IndexedMap::with_capacity(size))
    }
}

impl std::ops::DerefMut for PortMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for PortMap {
    type Target = IndexedMap<GlobalPortIdx, PortValue>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PortMap {
    /// Essentially asserts that the port given is undefined, it errors out if
    /// the port is defined and otherwise does nothing
    pub fn write_undef(&mut self, target: GlobalPortIdx) -> RuntimeResult<()> {
        if self[target].is_def() {
            Err(RuntimeError::UndefiningDefinedPort(target).into())
        } else {
            Ok(())
        }
    }

    /// Sets the given index to the given value without checking whether or not
    /// the assignment would conflict with an existing assignment. Should only
    /// be used by cells to set values that may be undefined
    #[must_use]
    pub fn write_exact_unchecked(
        &mut self,
        target: GlobalPortIdx,
        val: PortValue,
    ) -> UpdateStatus {
        if self[target].is_undef() && val.is_undef()
            || self[target]
                .as_option()
                .zip(val.as_option())
                .map(|(a, b)| a.eq_no_transitive_clocks(b))
                .unwrap_or_default()
        {
            UpdateStatus::Unchanged
        } else {
            self[target] = val;
            UpdateStatus::Changed
        }
    }

    #[inline]
    pub(crate) fn insert_val_unchecked(
        &mut self,
        idx: GlobalPortIdx,
        val: AssignedValue,
    ) {
        self[idx] = PortValue::new(val)
    }

    /// Sets the given index to undefined without checking whether or not it was
    /// already defined
    #[inline]
    pub fn write_undef_unchecked(&mut self, target: GlobalPortIdx) {
        self[target] = PortValue::new_undef();
    }

    #[inline(always)]
    pub fn insert_val(
        &mut self,
        target: GlobalPortIdx,
        val: AssignedValue,
    ) -> Result<UpdateStatus, Box<ConflictingAssignments>> {
        match self[target].as_option() {
            Some(t) => {
                if *t.winner() != AssignmentWinner::Implicit
                    && t.has_conflict_with(&val)
                {
                    Err(ConflictingAssignments {
                        target,
                        a1: t.clone(),
                        a2: val,
                    }
                    .into())
                } else if t.eq_no_transitive_clocks(&val) {
                    Ok(UpdateStatus::Unchanged)
                } else {
                    self[target] = PortValue::new(val);
                    Ok(UpdateStatus::Changed)
                }
            }
            // changed
            None => {
                self[target] = PortValue::new(val);
                Ok(UpdateStatus::Changed)
            }
        }
    }

    /// Identical to `insert_val` but returns a `RuntimeError` instead of a
    /// `ConflictingAssignments` error. This should be used inside of primitives
    /// while the latter is used in the general simulation flow.
    #[inline]
    pub fn insert_val_general(
        &mut self,
        target: GlobalPortIdx,
        val: AssignedValue,
    ) -> RuntimeResult<UpdateStatus> {
        self.insert_val(target, val)
            .map_err(|e| RuntimeError::ConflictingAssignments(e).into())
    }

    pub fn set_done(
        &mut self,
        target: GlobalPortIdx,
        done_bool: bool,
    ) -> RuntimeResult<UpdateStatus> {
        self.insert_val(
            target,
            AssignedValue::cell_value(if done_bool {
                BitVecValue::new_true()
            } else {
                BitVecValue::new_false()
            }),
        )
        .map_err(|e| RuntimeError::ConflictingAssignments(e).into())
    }
}

pub(crate) type CellMap = IndexedMap<GlobalCellIdx, CellLedger>;
pub(crate) type RefCellMap =
    IndexedMap<GlobalRefCellIdx, Option<GlobalCellIdx>>;
pub(crate) type RefPortMap =
    IndexedMap<GlobalRefPortIdx, Option<GlobalPortIdx>>;
pub(crate) type AssignmentRange = IndexRange<AssignmentIdx>;

#[derive(Clone)]
pub(crate) struct ComponentLedger {
    pub(crate) index_bases: BaseIndices,
    pub(crate) comp_id: ComponentIdx,
}

impl ComponentLedger {
    /// Convert a relative offset to a global one. Perhaps should take an owned
    /// value rather than a pointer
    pub fn convert_to_global_port(&self, port: &PortRef) -> GlobalPortRef {
        match port {
            PortRef::Local(l) => (&self.index_bases + l).into(),
            PortRef::Ref(r) => (&self.index_bases + r).into(),
        }
    }

    pub fn convert_to_global_cell(&self, cell: &CellRef) -> GlobalCellRef {
        match cell {
            CellRef::Local(l) => (&self.index_bases + l).into(),
            CellRef::Ref(r) => (&self.index_bases + r).into(),
        }
    }

    pub fn signature_ports(&self, ctx: &Context) -> IndexRange<GlobalPortIdx> {
        let sig = ctx.secondary[self.comp_id].signature();
        let beginning = &self.index_bases + sig.start();
        let end = &self.index_bases + sig.end();
        IndexRange::new(beginning, end)
    }
}

/// An enum encapsulating cell functionality. It is either a pointer to a
/// primitive or information about a calyx component instance
pub(crate) enum CellLedger {
    Primitive {
        // wish there was a better option with this one
        cell_dyn: Box<dyn Primitive>,
    },
    RaceDetectionPrimitive {
        cell_dyn: Box<dyn RaceDetectionPrimitive>,
    },
    Component(ComponentLedger),
}

impl Clone for CellLedger {
    fn clone(&self) -> Self {
        match self {
            Self::Primitive { cell_dyn } => Self::Primitive {
                cell_dyn: cell_dyn.clone_boxed(),
            },
            Self::RaceDetectionPrimitive { cell_dyn } => {
                Self::RaceDetectionPrimitive {
                    cell_dyn: cell_dyn.clone_boxed_rd(),
                }
            }
            Self::Component(component_ledger) => {
                Self::Component(component_ledger.clone())
            }
        }
    }
}

impl From<ComponentLedger> for CellLedger {
    fn from(v: ComponentLedger) -> Self {
        Self::Component(v)
    }
}

impl From<Box<dyn RaceDetectionPrimitive>> for CellLedger {
    fn from(cell_dyn: Box<dyn RaceDetectionPrimitive>) -> Self {
        Self::RaceDetectionPrimitive { cell_dyn }
    }
}

impl From<Box<dyn Primitive>> for CellLedger {
    fn from(cell_dyn: Box<dyn Primitive>) -> Self {
        Self::Primitive { cell_dyn }
    }
}

impl CellLedger {
    pub fn as_comp(&self) -> Option<&ComponentLedger> {
        match self {
            Self::Component(comp) => Some(comp),
            _ => None,
        }
    }

    #[inline]
    pub fn unwrap_comp(&self) -> &ComponentLedger {
        self.as_comp()
            .expect("Unwrapped cell ledger as component but received primitive")
    }

    #[must_use]
    pub fn as_primitive(&self) -> Option<&dyn Primitive> {
        match self {
            Self::Primitive { cell_dyn } => Some(&**cell_dyn),
            Self::RaceDetectionPrimitive { cell_dyn } => {
                Some(cell_dyn.as_primitive())
            }
            _ => None,
        }
    }

    pub fn unwrap_primitive(&self) -> &dyn Primitive {
        self.as_primitive()
            .expect("Unwrapped cell ledger as primitive but received component")
    }
}

impl Debug for CellLedger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primitive { .. } => f.debug_struct("Primitive").finish(),
            Self::RaceDetectionPrimitive { .. } => {
                f.debug_struct("RaceDetectionPrimitive").finish()
            }
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
impl CellLedger {
    pub(super) fn new_comp<C: AsRef<Context> + Clone>(
        idx: ComponentIdx,
        env: &Environment<C>,
    ) -> Self {
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
}

#[derive(Debug, Clone)]
pub(crate) struct PinnedPorts {
    map: HashMap<GlobalPortIdx, BitVecValue>,
}

impl PinnedPorts {
    pub fn iter(&self) -> impl Iterator<Item = (&GlobalPortIdx, &BitVecValue)> {
        self.map.iter()
    }

    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, port: GlobalPortIdx, val: BitVecValue) {
        self.map.insert(port, val);
    }

    pub fn remove(&mut self, port: GlobalPortIdx) {
        self.map.remove(&port);
    }
}

#[derive(Debug, Clone)]
pub struct MemoryMap {
    data: IndexedMap<MemoryLocation, BitVecValue>,
    clocks: SemiContiguousSecondaryMap<MemoryLocation, ClockPair>,
}

impl Index<MemoryLocation> for MemoryMap {
    type Output = BitVecValue;

    fn index(&self, index: MemoryLocation) -> &Self::Output {
        &self.data[index]
    }
}

impl Default for MemoryMap {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryMap {
    pub fn new() -> Self {
        Self {
            data: IndexedMap::new(),
            clocks: SemiContiguousSecondaryMap::new(),
        }
    }

    #[must_use]
    pub fn set_location(
        &mut self,
        idx: MemoryLocation,
        value: BitVecValue,
    ) -> UpdateStatus {
        let changed = self.data[idx] != value;
        self.data[idx] = value;
        changed.into()
    }

    pub fn push_clockless(&mut self, val: BitVecValue) -> MemoryLocation {
        self.data.push(val)
    }

    pub fn push_clocked(
        &mut self,
        val: BitVecValue,
        clock: ClockPair,
    ) -> MemoryLocation {
        let mem_loc = self.data.push(val);
        self.clocks.monotonic_insert(mem_loc, clock);
        mem_loc
    }

    pub fn get_clock(&self, key: MemoryLocation) -> Option<ClockPair> {
        self.clocks.get(key).copied()
    }

    pub fn get_clock_or_default(&self, key: MemoryLocation) -> ClockPair {
        self.clocks.get(key).copied().unwrap_or_default()
    }

    pub fn allocate_memory_location(
        &mut self,
        val: BitVecValue,
        cell: GlobalCellIdx,
        entry: Option<u32>,
        clock_map: &mut Option<&mut ClockMap>,
    ) -> MemoryLocation {
        if let Some(clock) = clock_map {
            self.push_clocked(val, new_clock_pair(clock, cell, entry))
        } else {
            self.push_clockless(val)
        }
    }

    pub fn peek_next_memory_location(&self) -> MemoryLocation {
        self.data.peek_next_idx()
    }

    /// Allocates a region of memory values.
    ///
    /// The iterator is assumed to cover the full range of memory values
    /// allocated for the given cell.
    pub fn allocate_region<I>(
        &mut self,
        iterator: I,
        cell: GlobalCellIdx,
        clock_map: &mut Option<&mut ClockMap>,
    ) -> MemoryRegion
    where
        I: Iterator<Item = BitVecValue>,
    {
        let mem_region_start = self.peek_next_memory_location();
        for (index, item) in iterator.enumerate() {
            self.allocate_memory_location(
                item,
                cell,
                Some(index.try_into().unwrap()),
                clock_map,
            );
        }
        let mem_region_end = self.peek_next_memory_location();
        MemoryRegion::new(mem_region_start, mem_region_end)
    }

    pub fn map_region<F, D>(
        &self,
        region: MemoryRegion,
        func: F,
    ) -> impl Iterator<Item = D>
    where
        F: Fn(&BitVecValue) -> D,
    {
        region.into_iter().map(move |i| func(&self.data[i]))
    }

    pub fn get_region_slice(&self, region: MemoryRegion) -> &[BitVecValue] {
        self.data.get_region_slice(region)
    }
}
