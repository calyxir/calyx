use crate::calyx_py::PyProfilingInfo;
use crate::calyx_timeline::CurrentlyActive;
use crate::dahlia_design::DahliaProfilingInfo;
use crate::design::Design;
use anyhow::{Ok, Result};
use baa::BitVecValue;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

// ORIGINALLY FROM fileinfo_emitter tool
#[derive(PartialEq, Eq, Hash, Clone, Deserialize, Debug)]
pub enum Adl {
    Calyx,
    Py,
    Dahlia,
}

// ORIGINALLY FROM fileinfo_emitter tool
#[derive(PartialEq, Eq, Hash, Clone, Deserialize)]
pub struct AdlInfo {
    pub adl: Adl,
    pub components: Vec<ComponentInfo>,
}

// ORIGINALLY FROM fileinfo_emitter tool
#[derive(PartialEq, Eq, Hash, Clone, Deserialize)]
pub struct ComponentInfo {
    // components may not have metadata attached.
    pub component: String,
    pub filename: Option<String>,
    pub linenum: Option<u64>,
    // association name
    pub varname: Option<String>,
    pub cells: Vec<PosInfo>,
    pub groups: Vec<PosInfo>,
}

impl ComponentInfo {
    pub fn cleanup(&mut self) {
        for c in &mut self.cells {
            c.cleanup();
        }

        for g in &mut self.groups {
            g.cleanup();
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Deserialize, Debug)]
pub struct PosInfo {
    pub name: String,
    pub filename: String,
    pub linenum: u64,
    pub varname: String,
}

impl PosInfo {
    /// Trims the internal filename to only the name of the file for easier visualization (and no parents).
    pub fn cleanup(&mut self) {
        let p = Path::new(&self.filename);
        self.filename = p.file_name().unwrap().to_str().unwrap().to_string();
        self.varname = self.varname.replace(";", "").replace("{", "");
    }

    /// Called to produce a Calyx-Py level ADL String
    pub fn adl_str(&self) -> String {
        format!("{{{}: {}}} {}", self.filename, self.linenum, self.varname)
    }

    /// Called while producing a Calyx-Py level Mixed flame graph.
    pub fn loc_str(&self) -> String {
        format!("{{{}: {}}}", self.filename, self.linenum)
    }
}

/// Intermediate information collected while profiling ADLs.
pub enum AdlIntermediateInfo {
    Dahlia(DahliaProfilingInfo),
    Py(PyProfilingInfo),
}

impl AdlIntermediateInfo {
    pub fn new(
        adl_filename: &str,
        dahlia_parent_file: Option<String>,
        d: &mut Design,
        out_dir: &str,
    ) -> Result<Self> {
        let adl_file = File::open(adl_filename)?;
        let AdlInfo {
            adl,
            mut components,
        } = serde_json::from_reader(BufReader::new(adl_file))?;
        // clean up all components
        for c in &mut components {
            c.cleanup();
        }
        match adl {
            Adl::Calyx => {
                panic!("Calyx \"ADL\" file should not be passed in!")
            }
            Adl::Py => {
                let pyi = PyProfilingInfo::default();
                // add position info into all nodes of the design
                d.embed_pos(&components);
                Ok(Self::Py(pyi))
            }
            Adl::Dahlia => {
                let dpi = DahliaProfilingInfo::new(
                    components,
                    dahlia_parent_file,
                    d,
                    out_dir,
                )?;
                Ok(Self::Dahlia(dpi))
            }
        }
    }

    /// Computes the ADL stacks/timeline based on active probes for a certain cycle.
    /// Should be called for every cycle.
    pub fn process_cycle(
        &mut self,
        value: &BitVecValue,
        design: &Design,
        calyx_active: &CurrentlyActive,
        cycle_count: u64,
    ) -> Result<()> {
        match self {
            AdlIntermediateInfo::Py(p) => p.update(design, value),
            AdlIntermediateInfo::Dahlia(d) => {
                d.process_cycle(calyx_active, cycle_count)
            }
        }
    }

    /// Terminates any remaining active ADL statements after the program terminates.
    pub fn close(&mut self, total_cycles: u64) -> Result<()> {
        match self {
            AdlIntermediateInfo::Py(_) => Ok(()),
            AdlIntermediateInfo::Dahlia(d) => d.close(total_cycles),
        }
    }

    pub fn output_flame(&mut self, out_dir: &str) -> Result<()> {
        match self {
            AdlIntermediateInfo::Py(p) => p.output_flame(out_dir),
            AdlIntermediateInfo::Dahlia(d) => d.output_flame(out_dir),
        }
    }
}
