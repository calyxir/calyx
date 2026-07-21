use crate::design::{Design, Stack};
use crate::visuals::flamegraph::{compute_flame, write_flames};
use anyhow::Result;
use baa::BitVecValue;
use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use std::path::PathBuf;

#[derive(Default, Clone, Debug)]
pub struct PyProfilingInfo {
    // adl flame stack
    adl_flame_map: IndexMap<BitVecValue, (Vec<Stack>, u64)>,
}

impl PyProfilingInfo {
    pub fn update(&mut self, d: &Design, v: &BitVecValue) -> Result<()> {
        if let Some((_stack, count)) = self.adl_flame_map.get_mut(v) {
            *count += 1;
        } else {
            let (stack, _, _) = d.compute_cycle_trace(v, true)?;
            self.adl_flame_map.insert(v.clone(), (stack, 1));
        }
        Ok(())
    }

    pub fn output_flame(&self, out_dir: &str) -> Result<()> {
        let mut flame_input_map: FxHashMap<Vec<Stack>, u64> =
            FxHashMap::default();
        for (_, (stack, count)) in self.adl_flame_map.iter() {
            let e = flame_input_map
                .entry((*stack.clone()).to_owned())
                .or_default();
            *e += *count;
        }
        let flame_input: Vec<(&u64, &Vec<Stack>)> =
            flame_input_map.iter().map(|(k, v)| (v, k)).collect();
        let flame = compute_flame(flame_input)?;
        let mut scaled_flame = PathBuf::from(out_dir);
        scaled_flame.push("py-scaled-flame.folded");
        let mut flat_flame = PathBuf::from(out_dir);
        flat_flame.push("py-flat-flame.folded");
        write_flames(&flame, Some(scaled_flame), Some(flat_flame))?;
        Ok(())

        // let flame_input: Vec<(&u64, &Vec<Stack>)> =
        //     self.trace_info.iter().map(|(s, c)| (c, s)).collect();
        // let flame = compute_flame(flame_input)?;
        // let mut scaled_flame = PathBuf::from(out_dir);
        // scaled_flame.push("dahlia-scaled-flame.folded");
        // let mut flat_flame = PathBuf::from(out_dir);
        // flat_flame.push("dahlia-flat-flame.folded");
        // write_flames(&flame, Some(scaled_flame), Some(flat_flame))?;
        // anyhow::Ok(())
    }
}
