use crate::{
    errors::RuntimeResult,
    flatten::{
        flat_ir::base::GlobalPortIdx,
        structures::{
            environment::{clock::ClockMap, PortMap},
            index_trait::SplitIndexRange,
            thread::ThreadMap,
        },
    },
    serialization::{PrintCode, Serializable},
};

use baa::BitVecValue;

pub struct AssignResult {
    pub destination: GlobalPortIdx,
    pub value: BitVecValue,
}

impl AssignResult {
    pub fn new(destination: GlobalPortIdx, value: BitVecValue) -> Self {
        Self { destination, value }
    }
}

impl From<(GlobalPortIdx, BitVecValue)> for AssignResult {
    fn from(value: (GlobalPortIdx, BitVecValue)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl From<(BitVecValue, GlobalPortIdx)> for AssignResult {
    fn from(value: (BitVecValue, GlobalPortIdx)) -> Self {
        Self::new(value.1, value.0)
    }
}

/// An enum used to denote whether or not committed updates changed the state
#[derive(Debug)]
pub enum UpdateStatus {
    Unchanged,
    Changed,
}

impl From<bool> for UpdateStatus {
    fn from(value: bool) -> Self {
        if value {
            Self::Changed
        } else {
            Self::Unchanged
        }
    }
}

impl UpdateStatus {
    #[inline]
    /// If the status is unchanged and other is changed, updates the status of
    /// self to changed, otherwise does nothing
    pub fn update(&mut self, other: Self) {
        if !self.as_bool() && other.as_bool() {
            *self = UpdateStatus::Changed;
        }
    }

    #[inline]
    /// Returns `true` if the update status is [`Changed`][].
    ///
    /// [`Changed`]: UpdateStatus::Changed
    #[must_use]
    pub fn as_bool(&self) -> bool {
        matches!(self, Self::Changed)
    }
}

impl std::ops::BitOr for UpdateStatus {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        if self.as_bool() || rhs.as_bool() {
            UpdateStatus::Changed
        } else {
            UpdateStatus::Unchanged
        }
    }
}

impl std::ops::BitOr for &UpdateStatus {
    type Output = UpdateStatus;

    fn bitor(self, rhs: Self) -> Self::Output {
        if self.as_bool() || rhs.as_bool() {
            UpdateStatus::Changed
        } else {
            UpdateStatus::Unchanged
        }
    }
}

impl std::ops::BitOrAssign for UpdateStatus {
    fn bitor_assign(&mut self, rhs: Self) {
        self.update(rhs)
    }
}

pub type UpdateResult = RuntimeResult<UpdateStatus>;

pub trait Primitive {
    fn exec_comb(&self, _port_map: &mut PortMap) -> UpdateResult {
        Ok(UpdateStatus::Unchanged)
    }

    fn exec_cycle(&mut self, _port_map: &mut PortMap) -> RuntimeResult<()> {
        Ok(())
    }

    fn has_comb_path(&self) -> bool {
        true
    }

    fn has_stateful_path(&self) -> bool {
        true
    }

    /// Serialize the state of this primitive, if any.
    fn serialize(&self, _code: Option<PrintCode>) -> Serializable {
        Serializable::Empty
    }

    // more efficient to override this with true in stateful cases
    fn has_serializable_state(&self) -> bool {
        self.serialize(None).has_state()
    }

    fn dump_memory_state(&self) -> Option<Vec<u8>> {
        None
    }

    fn get_ports(&self) -> SplitIndexRange<GlobalPortIdx>;

    /// Returns `true` if this primitive only has a combinational part
    fn is_combinational(&self) -> bool {
        self.has_comb_path() && !self.has_stateful_path()
    }

    fn clone_boxed(&self) -> Box<dyn Primitive>;
}

pub trait RaceDetectionPrimitive: Primitive {
    fn exec_comb_checked(
        &self,
        port_map: &mut PortMap,
        _clock_map: &mut ClockMap,
        _thread_map: &ThreadMap,
    ) -> UpdateResult {
        self.exec_comb(port_map)
    }

    fn exec_cycle_checked(
        &mut self,
        port_map: &mut PortMap,
        _clock_map: &mut ClockMap,
        _thread_map: &ThreadMap,
    ) -> RuntimeResult<()> {
        self.exec_cycle(port_map)
    }

    /// Get a reference to the underlying primitive. Unfortunately cannot add an
    /// optional default implementation due to size rules
    fn as_primitive(&self) -> &dyn Primitive;

    fn clone_boxed_rd(&self) -> Box<dyn RaceDetectionPrimitive>;
}
