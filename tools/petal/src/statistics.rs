use crate::design::{CellId, GroupId, RegisterId};
use crate::timeline::CurrentlyActive;
use anyhow::{Ok, Result};
use cranelift_entity::SecondaryMap;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Serialize, Serializer};
use std::fs::File;
use std::path::PathBuf;

/// Output Group stats CSV table
#[derive(Debug, Clone, Serialize)]
struct GroupStatsOut {
    name: String,
    num_times_active: u64,
    total_cycles: u64,
    min: u64,
    max: u64,
    avg: f64,
    can_static: bool,
}

#[derive(Debug, Clone, Default)]
struct GroupStats {
    name: String,
    num_times_active: u64,
    total_cycles: u64,
    min: u64,
    max: u64,
    // the start cycle of this "span", if there exists one.
    curr_start: Option<u64>,
}

impl GroupStats {
    pub fn new(name: String) -> Self {
        Self {
            name,
            num_times_active: 0,
            total_cycles: 0,
            min: u64::MAX,
            max: u64::MIN,
            curr_start: None,
        }
    }

    pub fn group_start(&mut self, cycle: u64) {
        assert!(self.curr_start.is_none());
        self.curr_start = Some(cycle);
    }

    pub fn group_end(&mut self, cycle: u64) {
        assert!(self.curr_start.is_some());
        let start = self.curr_start.unwrap();
        let length = cycle - start;
        if length > self.max {
            self.max = length;
        }
        if length < self.min {
            self.min = length;
        }
        self.num_times_active += 1;
        self.total_cycles += length;

        self.curr_start = None;
    }

    pub fn convert_to_csv_struct(&self) -> GroupStatsOut {
        let avg = self.num_times_active as f64 / self.total_cycles as f64;
        let can_static = self.max == self.min;
        GroupStatsOut {
            name: self.name.clone(),
            total_cycles: self.total_cycles,
            min: self.min,
            max: self.max,
            num_times_active: self.num_times_active,
            avg,
            can_static,
        }
    }
}

fn format_float<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let rounded = (value * 100.0).round() / 100.0;
    let s = format!("{:.2}", rounded);
    serializer.serialize_str(&s)
}

/// Output Cell stats CSV table
#[derive(Debug, Clone, Serialize)]
struct CellStatsOut {
    name: String,
    num_fsms: u32,
    total_cycles: u64,
    times_active: u64,
    #[serde(serialize_with = "format_float")]
    avg: f64,
    useful_cycles: u64,
    #[serde(serialize_with = "format_float")]
    useful_cycles_percent: f64,
    #[serde(serialize_with = "format_float")]
    group_or_primitive: f64,
    #[serde(serialize_with = "format_float")]
    fsm_update: f64,
    #[serde(serialize_with = "format_float")]
    pd_update: f64,
    #[serde(serialize_with = "format_float")]
    other: f64,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum CycleType {
    GroupOrPrimitive,
    FsmUpdate,
    PdUpdate,
    MultControl,
    Other,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
struct CellStats {
    name: String,
    num_fsms: u32,
    // fsms: FxHashSet<RegisterId>,
    // pds: FxHashSet<RegisterId>,
    total_cycles: u64,
    times_active: u64,
    type_to_num_cycles: FxHashMap<CycleType, u64>,
}

impl CellStats {
    pub fn new(
        name: String,
        fsms: FxHashSet<RegisterId>,
        // pds: FxHashSet<RegisterId>,
    ) -> Self {
        let mut s = Self {
            name,
            num_fsms: fsms.len() as u32,
            // fsms,
            // pds,
            total_cycles: 0,
            times_active: 0,
            type_to_num_cycles: FxHashMap::default(),
        };
        s.type_to_num_cycles.insert(CycleType::GroupOrPrimitive, 0);
        s.type_to_num_cycles.insert(CycleType::FsmUpdate, 0);
        s.type_to_num_cycles.insert(CycleType::PdUpdate, 0);
        s.type_to_num_cycles.insert(CycleType::Other, 0);
        println!("Cell stats: {s:?}");
        s
    }

    pub fn active_cell(&mut self, cycle_type: CycleType, started_now: bool) {
        self.type_to_num_cycles.insert(
            cycle_type.clone(),
            self.type_to_num_cycles[&cycle_type] + 1,
        );
        self.total_cycles += 1;
        if started_now {
            self.times_active += 1;
        }
    }

    pub fn convert_to_csv_struct(&self) -> CellStatsOut {
        let avg = self.total_cycles as f64 / self.times_active as f64;
        let useful_cycles = self.type_to_num_cycles
            [&CycleType::GroupOrPrimitive]
            + self.type_to_num_cycles[&CycleType::Other];
        let useful_cycles_percent =
            ((useful_cycles as f64) / (self.total_cycles as f64)) * 100.0;
        let group_or_primitive =
            self.get_type_percent(CycleType::GroupOrPrimitive);
        let fsm_update = self.get_type_percent(CycleType::FsmUpdate);
        let pd_update = self.get_type_percent(CycleType::PdUpdate);
        let other = self.get_type_percent(CycleType::Other);

        CellStatsOut {
            name: self.name.clone(),
            num_fsms: self.num_fsms,
            total_cycles: self.total_cycles,
            avg,
            times_active: self.times_active,
            useful_cycles,
            useful_cycles_percent,
            group_or_primitive,
            fsm_update,
            pd_update,
            other,
        }
    }
}

impl CellStats {
    fn get_type_percent(&self, t: CycleType) -> f64 {
        let count = self.type_to_num_cycles[&t] as f64;
        (count / self.total_cycles as f64) * 100.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct Statistics {
    group_to_stats: SecondaryMap<GroupId, GroupStats>,
    cell_to_stats: SecondaryMap<CellId, CellStats>,
    fsms: FxHashSet<RegisterId>,
    pds: FxHashSet<RegisterId>,
}

impl Statistics {
    pub fn new(
        group_to_names: Vec<(GroupId, String)>,
        cell_info: Vec<(
            CellId,
            String,
            FxHashSet<RegisterId>,
            FxHashSet<RegisterId>,
        )>,
    ) -> Self {
        let fsms: FxHashSet<RegisterId> = FxHashSet::default();
        let pds: FxHashSet<RegisterId> = FxHashSet::default();
        let group_to_stats = SecondaryMap::from_iter(
            group_to_names
                .into_iter()
                .map(|(group_id, name)| (group_id, GroupStats::new(name))),
        );
        let cell_to_stats = SecondaryMap::from_iter(cell_info.into_iter().map(
            |(cell_id, name, fsms, pds)| (cell_id, CellStats::new(name, fsms)),
        ));
        Self {
            group_to_stats,
            cell_to_stats,
            fsms,
            pds,
        }
    }

    pub fn update(
        &mut self,
        started: &CurrentlyActive,
        ended: &CurrentlyActive,
        cycle: u64,
        gp_flag: bool,
        register_value_diffs: &FxHashMap<u64, FxHashMap<RegisterId, u64>>,
        active_cells: &FxHashSet<CellId>,
    ) {
        // process all groups that started
        for started_group in started.get_active_groups() {
            assert!(self.group_to_stats.get(*started_group).is_some());
            self.group_to_stats[*started_group].group_start(cycle);
        }

        // process all groups that ended
        for ended_group in ended.get_active_groups() {
            assert!(self.group_to_stats.get(*ended_group).is_some());
            self.group_to_stats[*ended_group].group_end(cycle);
        }

        let cycle_type =
            self.classify_cycle(gp_flag, &register_value_diffs.get(&cycle));
        for cell in active_cells {
            let started_now = started.get_active_cells().contains(cell);
            self.cell_to_stats[*cell]
                .active_cell(cycle_type.clone(), started_now);
        }
    }

    pub fn close(&mut self, to_close: &CurrentlyActive, cycle: u64) {
        // close out all groups that are still active
        for ended_group in to_close.get_active_groups() {
            assert!(self.group_to_stats.get(*ended_group).is_some());
            self.group_to_stats[*ended_group].group_end(cycle);
        }
    }

    pub fn output(&self, out_dir: &str) -> Result<()> {
        self.output_group(out_dir)?;
        self.output_cell(out_dir)?;
        Ok(())
    }
}

impl Statistics {
    fn output_cell(&self, out_dir: &str) -> Result<()> {
        let mut path = PathBuf::from(out_dir);
        path.push("cell-stats.csv");
        let file = File::create(path)?;

        let mut writer = csv::Writer::from_writer(file);
        // serialize in sorted order of cell name
        let name_to_stats_csv: FxHashMap<String, CellStatsOut> = self
            .cell_to_stats
            .iter()
            .filter(|(_, stats)| **stats != CellStats::default())
            .map(|(_, stats)| {
                (stats.name.clone(), stats.convert_to_csv_struct())
            })
            .collect();
        let mut sorted_names: Vec<String> =
            name_to_stats_csv.keys().cloned().collect();
        sorted_names.sort();
        for name in sorted_names {
            writer.serialize(name_to_stats_csv[&name].clone())?;
        }

        Ok(())
    }

    fn output_group(&self, out_dir: &str) -> Result<()> {
        let mut path = PathBuf::from(out_dir);
        path.push("group-stats.csv");
        let file = File::create(path)?;

        let mut writer = csv::Writer::from_writer(file);
        // serialize in sorted order of groups' static name.
        let name_to_stats_csv: FxHashMap<String, GroupStatsOut> = self
            .group_to_stats
            .iter()
            .map(|(_, stats)| {
                (stats.name.clone(), stats.convert_to_csv_struct())
            })
            .collect();
        let mut sorted_names: Vec<String> =
            name_to_stats_csv.keys().cloned().collect();
        sorted_names.sort();
        for name in sorted_names {
            writer.serialize(name_to_stats_csv[&name].clone())?;
        }
        Ok(())
    }

    fn classify_cycle(
        &self,
        gp_flag: bool,
        register_value_diffs: &Option<&FxHashMap<RegisterId, u64>>,
    ) -> CycleType {
        let changed_registers =
            if let Some(register_value_diffs) = register_value_diffs {
                register_value_diffs.len()
            } else {
                0
            };
        if gp_flag {
            CycleType::GroupOrPrimitive
        } else if changed_registers == 0 {
            CycleType::Other
        } else {
            let updated_fsms_count = if let Some(r) = register_value_diffs {
                r.keys().filter(|r| self.fsms.contains(*r)).count()
            } else {
                0
            };
            if updated_fsms_count == changed_registers {
                CycleType::FsmUpdate
            } else if updated_fsms_count == 0 {
                CycleType::PdUpdate
            } else {
                CycleType::MultControl
            }
        }
    }
}
