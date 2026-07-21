use crate::design::Stack;
use baa::BitVecValue;
use indexmap::IndexMap;

#[derive(Default, Clone, Debug)]
pub struct PyProfilingInfo {
    // mixed flame stack
    mixed_flame_map: IndexMap<BitVecValue, (Vec<Stack>, u64)>,
    // adl flame stack
    adl_flame_map: IndexMap<BitVecValue, (Vec<Stack>, u64)>,
}
