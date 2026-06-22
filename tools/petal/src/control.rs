use anyhow::{Ok, Result, anyhow};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs::File,
    io::BufReader,
};

// ORIGINALLY FROM fileinfo_emitter tool
// Obtaining the original line numbers of Calyx
#[derive(PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
struct ControlCalyxPosInfo {
    pub pos_num: u32,
    pub linenum: u32,
    pub ctrl_node: String,
}

// ORIGINALLY FROM uniquefy_enables Calyx pass
// path_descriptor_infos: BTreeMap<String, PathDescriptorInfo>,
/// Information to serialize for locating path descriptors
#[derive(Serialize, Deserialize, Debug)]
pub struct PathDescriptorInfo {
    /// enable id --> descriptor
    pub enables: BTreeMap<String, String>,
    /// descriptor --> position set
    /// (Ideally I'd do a position set --> descriptor mapping but
    /// a set shouldn't be a key.)
    pub control_pos: BTreeMap<String, BTreeSet<u32>>,
}

/// ORIGINALLY FROM TDCC Calyx pass; replaced all instances of Id with String
/// Information to serialize for profiling purposes
#[derive(PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
enum ProfilingInfo {
    Fsm(FSMInfo),
    Par(ParInfo),
    SingleEnable(SingleEnableInfo),
}

/// ORIGINALLY FROM TDCC Calyx pass; replaced all instances of Id with String
/// Information to be serialized for a group that isn't managed by a FSM
/// This can happen if the group is the only group in a control block or a par arm
#[derive(PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
struct SingleEnableInfo {
    pub component: String,
    pub group: String,
}

/// ORIGINALLY FROM TDCC Calyx pass; replaced all instances of Id with String
#[derive(PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
struct ParInfo {
    pub component: String,
    pub par_group: String,
    pub child_groups: Vec<ParChildInfo>,
    /// Values in the position set attribute that was associated with the control par node
    /// that generated this par group.
    pub pos: Vec<u32>,
}

/// ORIGINALLY FROM TDCC Calyx pass; replaced all instances of Id with String
#[derive(PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
struct ParChildInfo {
    pub group: String,
    pub register: String,
}

/// ORIGINALLY FROM TDCC Calyx pass; replaced all instances of Id with String
/// Information to be serialized for a single FSM
#[derive(PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
struct FSMInfo {
    pub component: String,
    pub group: String,
    pub fsm: String,
    pub states: Vec<FSMStateInfo>,
    /// Values in the position set attribute that was associated with the control node
    /// that generated the TDCC group.
    pub pos: Vec<u32>,
}

/// ORIGINALLY FROM TDCC Calyx pass; replaced all instances of Id with String
/// Mapping of FSM state ids to corresponding group names
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Serialize, Deserialize,
)]
struct FSMStateInfo {
    id: u64,
    group: String,
}

/// Represents the registers that do the bookkeeping for a control group
/// NOTE: Will be necessary for the timeline view, but not for the flame graph.
// #[derive(Debug)]
// enum ControlRegister {
//     FSM(String),
//     PD(Vec<String>),
// }

#[derive(Debug, Clone)]
/// Information for the control group obtained from the TDCC compiler pass.
/// NOTE: More will be added for the timeline view.
pub struct TdccInfo {
    pub name: String,
}

#[derive(Debug)]
/// Represents all statically obtained information (from compiler passes & external tools)
/// about control nodes (ex. seq, par) and the control groups (ex. tdcc*) used to manage them.
pub struct ControlInfo {
    /// Obtained from the TDCC pass that converts control nodes to control groups.
    /// Note: If the original program was written in an ADL, there would be multiple entries
    /// for the same TDCC group (for each position from the ADL)
    /// We also use a vector because multiple control nodes may map to the same position in the ADL
    tdcc_map: FxHashMap<u32, Vec<TdccInfo>>,

    /// names for pretty printing (from control-pos file)
    pretty_map: FxHashMap<u32, String>,

    /// info taken straight from the path-descriptors file (for computing parent-child relationships
    /// between control groups & groups.
    pd: BTreeMap<String, PathDescriptorInfo>,
}

impl ControlInfo {
    pub fn descriptors(&self, c: &str) -> &PathDescriptorInfo {
        self.pd.get(c).unwrap()
    }

    pub fn get_pretty(&self, pos_set: &BTreeSet<u32>) -> Result<(String, u32)> {
        for pos in pos_set.iter() {
            if let Some(pretty) = self.pretty_map.get(pos) {
                return Ok((pretty.clone(), *pos));
            }
        }
        Err(anyhow!(
            "Positions in {:?} not found in pretty map",
            pos_set
        ))
    }

    pub fn get_tdcc(&self, pos: u32) -> Result<Option<&Vec<TdccInfo>>> {
        Ok(self.tdcc_map.get(&pos))
    }

    pub fn new(
        tdcc_filename: String,
        pd_filename: String,
        pos_filename: String,
    ) -> Result<Self> {
        let mut pp_name_map = FxHashMap::default();
        let mut tdcc_map = FxHashMap::default();

        // need to process pos_file first because it gives us the Calyx positions of all control nodes.
        log::debug!("Parsing {}", pos_filename);
        let pos_file = File::open(pos_filename)?;
        let pos: HashMap<String, Vec<ControlCalyxPosInfo>> =
            serde_json::from_reader(BufReader::new(pos_file))?;
        for (_component, pos_list) in pos {
            for pos in pos_list {
                let pp_name = format!("L{}:{}", pos.linenum, pos.ctrl_node);
                pp_name_map.insert(pos.pos_num, pp_name);
            }
        }

        log::debug!("Parsing {}", tdcc_filename);
        let tdcc_file = File::open(tdcc_filename)?;
        let tdcc_profiling_info: HashSet<ProfilingInfo> =
            serde_json::from_reader(BufReader::new(tdcc_file))?;
        // some control nodes may have been compiled away due to optimizations, etc.
        // need to filter nodes that don't remain in code.
        for control_group in tdcc_profiling_info {
            match control_group {
                ProfilingInfo::Fsm(fsminfo) => {
                    for pos in fsminfo.pos {
                        tdcc_map.entry(pos).or_insert(vec![]).push(TdccInfo {
                            name: fsminfo.group.clone(),
                        });
                    }
                }
                ProfilingInfo::Par(par_info) => {
                    for pos in par_info.pos {
                        tdcc_map.entry(pos).or_insert(vec![]).push(TdccInfo {
                            name: par_info.par_group.clone(),
                        });
                    }
                }
                ProfilingInfo::SingleEnable(_single_enable_info) => { // do nothing since there is no control group
                }
            }
        }

        log::debug!("Parsing {}", pd_filename);
        let pd_file = File::open(pd_filename)?;
        let pd: BTreeMap<String, PathDescriptorInfo> =
            serde_json::from_reader(BufReader::new(pd_file))?;

        let out = Self {
            tdcc_map,
            pretty_map: pp_name_map,
            pd,
        };

        Ok(out)
    }
}
