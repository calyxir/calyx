use anyhow::Result;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::path::Path;

#[derive(PartialEq, Eq, Hash, Clone, Deserialize)]
/// An instance of a cell being shared; obtained from cell-share compiler pass.
struct ShareEntry {
    original: String, // cell to be replaced
    new: String,      // replacement cell (shared)
    cell_type: String,
}

/// Maps shared cells in each component.
pub struct SharedCellsInfo {
    /// component --> {original cell --> new cell}
    shared_map: FxHashMap<String, FxHashMap<String, String>>,
}

impl SharedCellsInfo {
    pub fn new(fname: String) -> Result<Self> {
        let mut shared_map: FxHashMap<String, FxHashMap<String, String>> =
            FxHashMap::default();
        let p = Path::new(&fname);
        if p.exists() {
            let f = File::open(fname)?;
            let shared_cells: BTreeMap<String, Vec<ShareEntry>> =
                serde_json::from_reader(f).unwrap();

            for (component, shared_vec) in shared_cells {
                // old -> new
                let mut internal_map: FxHashMap<String, String> =
                    FxHashMap::default();
                for shared in shared_vec {
                    internal_map.insert(shared.original, shared.new);
                }
                shared_map.insert(component, internal_map);
            }
        }
        Ok(Self { shared_map })
    }

    /// Returns the replacement (new) cell for a potentially shared cell, if one exists.
    pub fn get_replacement(
        &self,
        component: String,
        name: String,
    ) -> Option<String> {
        self.shared_map.get(&component)?.get(&name).cloned()
    }
}
