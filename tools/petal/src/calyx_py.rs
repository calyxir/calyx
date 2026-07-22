use crate::design::{AdlMode, Design, Stack};
use crate::visuals::flamegraph::{compute_flame, write_flames};
use anyhow::Result;
use baa::BitVecValue;
use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use std::path::PathBuf;

/// Intermediate information collected to profile Calyx-Py programs.
#[derive(Default, Clone, Debug)]
pub struct PyProfilingInfo {
    /// Flame stacks at the Calyx-py level
    adl_flame_map: IndexMap<BitVecValue, (Vec<Stack>, u64)>,
    /// Flame stacks that combine the Calyx-py level and the Calyx level
    mixed_flame_map: IndexMap<BitVecValue, (Vec<Stack>, u64)>,
}

impl PyProfilingInfo {
    /// Updates the intermediate information based on the active probes in the cycle.
    pub fn update(&mut self, d: &Design, v: &BitVecValue) -> Result<()> {
        if let Some((_stack, count)) = self.adl_flame_map.get_mut(v) {
            *count += 1;
        } else {
            let (stack, _, _) = d.compute_cycle_trace(v, AdlMode::CalyxPy)?;
            self.adl_flame_map.insert(v.clone(), (stack, 1));
        }

        if let Some((_stack, count)) = self.mixed_flame_map.get_mut(v) {
            *count += 1;
        } else {
            let (stack, _, _) = d.compute_cycle_trace(v, AdlMode::Mixed)?;
            self.mixed_flame_map.insert(v.clone(), (stack, 1));
        }

        Ok(())
    }

    pub fn output_flame(&self, out_dir: &str) -> Result<()> {
        self.output_flame_helper(out_dir, &self.adl_flame_map, "py")?;

        self.output_flame_helper(out_dir, &self.mixed_flame_map, "mixed")?;

        Ok(())
    }
}

impl PyProfilingInfo {
    fn output_flame_helper(
        &self,
        out_dir: &str,
        m: &IndexMap<BitVecValue, (Vec<Stack>, u64)>,
        file_prefix: &str,
    ) -> Result<()> {
        let mut flame_input_map: FxHashMap<Vec<Stack>, u64> =
            FxHashMap::default();
        for (_, (stack, count)) in m.iter() {
            let e = flame_input_map
                .entry((*stack.clone()).to_owned())
                .or_default();
            *e += *count;
        }
        let flame_input: Vec<(&u64, &Vec<Stack>)> =
            flame_input_map.iter().map(|(k, v)| (v, k)).collect();
        let flame = compute_flame(flame_input)?;
        let mut scaled_flame = PathBuf::from(out_dir);
        scaled_flame.push(format!("{file_prefix}-scaled-flame.folded"));
        let mut flat_flame = PathBuf::from(out_dir);
        flat_flame.push(format!("{file_prefix}-flat-flame.folded"));
        write_flames(&flame, Some(scaled_flame), Some(flat_flame))?;
        Ok(())
    }
}
