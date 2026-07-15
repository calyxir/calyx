use crate::design::{CellId, GroupId};
use crate::timeline::CurrentlyActive;
use anyhow::{Context, Ok, Result, anyhow};
use cranelift_entity::SecondaryMap;
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::collections::HashMap;
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

/// Output Cell stats CSV table
#[derive(Debug, Clone, Serialize)]
struct CellStatsOut {
    name: String,
    num_fsms: u32,
    total_cycles: u64,
    times_active: u64,
    avg: f64,
    useful_cycles: u64,
    useful_cycles_percent: f64,
    group_or_primitive: f64,
    fsm_update: f64,
    pd_update: f64,
    other: f64,
}

#[derive(Debug, Clone, Default)]
pub struct Statistics {
    group_to_stats: SecondaryMap<GroupId, GroupStats>,
    // cell_to_stats: SecondaryMap<CellId, CellStats>,
}

impl Statistics {
    pub fn new(group_to_names: Vec<(GroupId, String)>) -> Self {
        let group_to_stats = SecondaryMap::from_iter(
            group_to_names
                .into_iter()
                .map(|(group_id, name)| (group_id, GroupStats::new(name))),
        );
        Self { group_to_stats }
    }

    pub fn update(
        &mut self,
        started: &CurrentlyActive,
        ended: &CurrentlyActive,
        cycle: u64,
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
    }

    pub fn output(&self, out_dir: &str) -> Result<()> {
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
}
