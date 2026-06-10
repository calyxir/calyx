use anyhow::{Context, Ok, Result, anyhow};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs::File,
    io::BufReader,
};

use serde::{Deserialize, Serialize};

// ORIGINALLY FROM fileinfo_emitter tool
// Obtaining the original line numbers of Calyx
#[derive(PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
struct ControlCalyxPosInfo {
    // TODO: Probably good to add filename as well in case the Calyx component
    // TODO: Also we need to make sure that we pull out the line number from Calyx (check if file is .futil)
    // pub filename: String,
    pub pos_num: u32,
    pub linenum: u32,
    pub ctrl_node: String,
}

// ORIGINALLY FROM uniquefy_enables Calyx pass
// path_descriptor_infos: BTreeMap<String, PathDescriptorInfo>,
/// Information to serialize for locating path descriptors
#[derive(Serialize, Deserialize)]
struct PathDescriptorInfo {
    /// enable id --> descriptor
    pub enables: BTreeMap<String, String>,
    /// descriptor --> position set
    /// (Stringeally I'd do a position set --> descriptor mapping but
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

pub fn parse(
    tdcc_filename: String,
    pd_filename: String,
    pos_filename: String,
) -> Result<()> {
    println!("Parsing {}", tdcc_filename);
    let tdcc_file = File::open(tdcc_filename)?;
    let tdcc_profiling_info: HashSet<ProfilingInfo> =
        serde_json::from_reader(BufReader::new(tdcc_file))?;

    println!("Parsing {}", pd_filename);
    let pd_file = File::open(pd_filename)?;
    let pd: BTreeMap<String, PathDescriptorInfo> =
        serde_json::from_reader(BufReader::new(pd_file))?;

    println!("Parsing {}", pos_filename);
    let pos_file = File::open(pos_filename)?;
    let pos: HashMap<String, Vec<ControlCalyxPosInfo>> =
        serde_json::from_reader(BufReader::new(pos_file))?;

    Ok(())
}
