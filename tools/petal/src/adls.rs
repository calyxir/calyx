use crate::calyx_timeline::CurrentlyActive;
use crate::dahlia_design::DahliaProfilingInfo;
use crate::design::Design;
use anyhow::{Ok, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

#[derive(PartialEq, Eq, Hash, Clone, Deserialize, Debug)]
pub enum Adl {
    Calyx,
    Py,
    Dahlia,
}

#[derive(PartialEq, Eq, Hash, Clone, Deserialize)]
pub struct AdlInfo {
    pub adl: Adl,
    pub components: Vec<ComponentInfo>,
}

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

#[derive(PartialEq, Eq, Hash, Clone, Deserialize)]
pub struct PosInfo {
    pub name: String,
    pub filename: String,
    pub linenum: u64,
    pub varname: String,
}

pub enum AdlIntermediateInfo {
    Dahlia(DahliaProfilingInfo),
}

impl AdlIntermediateInfo {
    pub fn new(
        adl_filename: &str,
        dahlia_parent_file: Option<String>,
        d: &Design,
    ) -> Result<Self> {
        let adl_file = File::open(adl_filename)?;
        let AdlInfo { adl, components } =
            serde_json::from_reader(BufReader::new(adl_file))?;
        match adl {
            Adl::Calyx => {
                panic!("Calyx \"ADL\" file should not be passed in!")
            }
            Adl::Py => {
                todo!()
            }
            Adl::Dahlia => {
                let dpi = DahliaProfilingInfo::new(
                    components,
                    dahlia_parent_file,
                    d,
                )?;
                Ok(Self::Dahlia(dpi))
            }
        }
    }

    pub fn process_cycle(
        &mut self,
        calyx_active: &CurrentlyActive,
        cycle_count: u64,
    ) -> Result<()> {
        match self {
            AdlIntermediateInfo::Dahlia(d) => {
                d.process_cycle(calyx_active, cycle_count)
            }
        }
    }

    pub fn close(&mut self, total_cycles: u64) -> Result<()> {
        match self {
            AdlIntermediateInfo::Dahlia(d) => d.close(total_cycles),
        }
    }

    pub fn output_flame(&mut self, out_dir: &str) -> Result<()> {
        match self {
            AdlIntermediateInfo::Dahlia(d) => d.output_flame(out_dir),
        }
    }

    pub fn output_timeline(self, out_dir: &str) -> Result<()> {
        match self {
            AdlIntermediateInfo::Dahlia(d) => d.output_timeline(out_dir),
        }
    }
}
