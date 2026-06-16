use anyhow::{Context, Ok, Result, anyhow};
use cranelift_entity::{PrimaryMap, entity_impl};
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

/// Represents the registers that do the bookkeeping for a
/// control group
enum ControlRegister {
    FSM(String),
    PD(Vec<String>),
}

struct ControlMeta {
    name: String, // name of the TDCC group (later changed into signal?)
    groups: Vec<String>, // groups that are invoked under this control node
    component: String,
    control_type: String, // seq, par, etc. optimize into enum?
    registers: Option<ControlRegister>,
    pos: u32,
    line_num: u32,
}

struct AllControl {
    component_to_controls: HashMap<String, HashMap<u32, ControlMeta>>,
}

impl AllControl {
    pub fn new(
        tdcc_filename: String,
        pd_filename: String,
        pos_filename: String,
    ) -> Result<Self> {
        let mut component_to_controls = HashMap::new();

        // need to process pos_file first because it gives us the Calyx positions of all control nodes.
        println!("Parsing {}", pos_filename);
        let pos_file = File::open(pos_filename)?;
        let pos: HashMap<String, Vec<ControlCalyxPosInfo>> =
            serde_json::from_reader(BufReader::new(pos_file))?;
        for (component, pos_list) in pos {
            let mut controls: HashMap<u32, ControlMeta> = HashMap::new();
            for pos in pos_list {
                controls.insert(
                    pos.pos_num,
                    ControlMeta {
                        name: "".to_string(), // will be filled in by tdcc_file
                        groups: vec![],       // will be filled in by pd_file
                        component: component.clone(),
                        control_type: pos.ctrl_node,
                        registers: None, // will be filled in by tdcc_file
                        pos: pos.pos_num,
                        line_num: pos.linenum,
                    },
                );
            }
            component_to_controls.insert(component, controls);
        }

        println!("Parsing {}", tdcc_filename);
        let tdcc_file = File::open(tdcc_filename)?;
        let tdcc_profiling_info: HashSet<ProfilingInfo> =
            serde_json::from_reader(BufReader::new(tdcc_file))?;
        for control_group in tdcc_profiling_info {
            match control_group {
                ProfilingInfo::Fsm(fsminfo) => {
                    // search component_to_controls for the entry that matches this FSM
                    let mut found = false;
                    let comp_ctrl_map = component_to_controls
                        .get_mut(&fsminfo.component)
                        .unwrap();
                    for pos in fsminfo.pos {
                        // there can be multiple positions because of
                        if let Some(c) = comp_ctrl_map.get_mut(&pos) {
                            c.name = fsminfo.group.clone();
                            c.registers =
                                Some(ControlRegister::FSM(fsminfo.fsm.clone()));
                            found = true;
                        }
                    }
                    assert!(found);
                }
                ProfilingInfo::Par(par_info) => {
                    // search component_to_controls for the entry that matches this par group
                }
                ProfilingInfo::SingleEnable(_single_enable_info) => { // do nothing since there is no control group
                }
            }
        }

        println!("Parsing {}", pd_filename);
        let pd_file = File::open(pd_filename)?;
        let pd: BTreeMap<String, PathDescriptorInfo> =
            serde_json::from_reader(BufReader::new(pd_file))?;

        let mut out = Self {
            component_to_controls,
        };
        Ok(out)
    }
    // pub fn parse(
    //     tdcc_filename: String,
    //     pd_filename: String,
    //     pos_filename: String,
    // ) -> Result<()> {
    //     println!("Parsing {}", tdcc_filename);
    //     let tdcc_file = File::open(tdcc_filename)?;
    //     let tdcc_profiling_info: HashSet<ProfilingInfo> =
    //         serde_json::from_reader(BufReader::new(tdcc_file))?;

    //     println!("Parsing {}", pd_filename);
    //     let pd_file = File::open(pd_filename)?;
    //     let pd: BTreeMap<String, PathDescriptorInfo> =
    //         serde_json::from_reader(BufReader::new(pd_file))?;

    //     println!("Parsing {}", pos_filename);
    //     let pos_file = File::open(pos_filename)?;
    //     let pos: HashMap<String, Vec<ControlCalyxPosInfo>> =
    //         serde_json::from_reader(BufReader::new(pos_file))?;

    //     Ok(())
    // }
}

// #[derive(Clone, Copy, Hash, PartialEq, Eq, Default)]
// pub struct ControlId(u32);
// entity_impl!(ControlId, "control");

// #[derive(Debug, Clone)]

// struct Control {
//     name: String,

//     // probe: SignalRef,
//     // invokes: SmallVec<[InvokeId; 6]>,
//     // probe_idx: u32,
// }
