use crate::design::{CellId, ControlId, GroupId, RegisterId};
use crate::visuals::perfetto_protos::track_event::Type;
use crate::visuals::timeline::{Timeline, TrackEventInfo, Uuid};
use anyhow::{Ok, Result};
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;
use std::fs::File;

pub const NON_ID_THREAD: u32 = u32::MAX;

/// Represents a set of active groups/cells/control. This is used for the timeline view where
/// we need to know the timestamps of groups/cells/control activity.
#[derive(Clone, Debug, Default)]
pub struct CurrentlyActive {
    groups: FxHashSet<GroupId>,
    cells: FxHashSet<CellId>,
    control: FxHashSet<ControlId>,
}

impl CurrentlyActive {
    pub fn new() -> Self {
        Self {
            groups: FxHashSet::default(),
            cells: FxHashSet::default(),
            control: FxHashSet::default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
            && self.cells.is_empty()
            && self.control.is_empty()
    }

    pub fn add_active_group(&mut self, group: GroupId) {
        self.groups.insert(group);
    }

    pub fn add_active_cell(&mut self, cell: CellId) {
        self.cells.insert(cell);
    }

    pub fn add_active_control(&mut self, control: ControlId) {
        self.control.insert(control);
    }

    pub fn get_active_groups(&self) -> &FxHashSet<GroupId> {
        &self.groups
    }

    pub fn get_active_cells(&self) -> &FxHashSet<CellId> {
        &self.cells
    }

    /// Results the diffs between existing active cells/control/groups vs active cells/control/groups
    /// Returns (Started cells/control/groups, Ended cells/control/groups).
    pub fn resolve(&self, new: &Self) -> Result<(Self, Self)> {
        let ended = Self {
            groups: self.groups.difference(&new.groups).copied().collect(),
            cells: self.cells.difference(&new.cells).copied().collect(),
            control: self.control.difference(&new.control).copied().collect(),
        };
        let started = Self {
            groups: new.groups.difference(&self.groups).copied().collect(),
            cells: new.cells.difference(&self.cells).copied().collect(),
            control: new.control.difference(&self.control).copied().collect(),
        };
        Ok((started, ended))
    }
}

#[derive(Clone, Debug, Default)]
struct TrackZeroInfo {
    uuid: Uuid,
    occupant: Option<GroupId>,
    backups: Vec<Uuid>,
    backup_occupants: FxHashMap<Uuid, GroupId>,
}

impl TrackZeroInfo {
    pub fn new(uuid: Uuid) -> Self {
        Self {
            uuid,
            ..Default::default()
        }
    }

    pub fn start_group(
        &mut self,
        t: &mut Timeline,
        group: GroupId,
    ) -> Result<Uuid> {
        if self.occupant.is_some()
            && self.backups.len() > self.backup_occupants.len()
        {
            // find the first unoccupied
            let idx = self
                .backups
                .iter()
                .position(|u| !self.backup_occupants.contains_key(u))
                .unwrap();
            let uuid = self.backups[idx];
            self.backup_occupants.insert(uuid, group);
            Ok(uuid)
        } else if self.occupant.is_some() {
            // all of the backups are occupied; create a new thread
            let idx = self.backups.len();
            let name = format!("Thread 000_{idx}");
            let new_uuid = t.register_descriptor(name, Some(self.uuid))?;
            self.backups.push(new_uuid);
            self.backup_occupants.insert(new_uuid, group);
            Ok(new_uuid)
        } else {
            assert!(self.occupant.is_none());
            self.occupant = Some(group);
            Ok(self.uuid)
        }
    }

    pub fn end_group(&mut self, group: GroupId) -> Result<Uuid> {
        if Some(group) == self.occupant {
            self.occupant = None;
            Ok(self.uuid)
        } else {
            let (u, _) = self
                .backup_occupants
                .iter()
                .find(|(_, v)| **v == group)
                .unwrap();

            let uuid = *u;
            self.backup_occupants.remove(&uuid);
            Ok(uuid)
        }
    }
}

/// Constructs and outputs protobuf messages for constructing a timeline view.
/// In the timeline, each component cell has a distinct track which contains tracks containing the
/// group/control group/control register activity within that cell, organized as below:
/// - Control register updates get a single track ("Control register updates")
/// - Each control group gets its own track, under the "Control groups" track
/// - Groups are organized by their statically defined thread IDs. Each thread gets its own track.
pub struct CalyxTimeline {
    timeline: Timeline,
    cell_to_info: FxHashMap<CellId, TrackEventInfo>,
    control_to_info: FxHashMap<ControlId, TrackEventInfo>,
    group_to_info: FxHashMap<GroupId, TrackEventInfo>,
    register_to_uuid: FxHashMap<RegisterId, Uuid>,
    // accounting for the corner case where there are multiple groups mapped to the cell's
    // track 0, but they are optimized to run in parallel
    track_zero_to_backups: FxHashMap<Uuid, TrackZeroInfo>,
}

impl CalyxTimeline {
    pub fn new() -> Result<Self> {
        let s = Self {
            timeline: Timeline::new(),
            cell_to_info: FxHashMap::default(),
            control_to_info: FxHashMap::default(),
            group_to_info: FxHashMap::default(),
            register_to_uuid: FxHashMap::default(),
            track_zero_to_backups: FxHashMap::default(),
        };
        Ok(s)
    }

    pub fn get_control_register_uuid(&self, register: &RegisterId) -> &Uuid {
        self.register_to_uuid.get(register).unwrap()
    }

    /// Adds information about a group. This function does not construct a new descriptor because
    /// groups are assigned to a "Thread n" track.
    pub fn register_group(
        &mut self,
        group_id: GroupId,
        uuid: Uuid,
        name: String,
    ) -> Result<()> {
        self.group_to_info
            .insert(group_id, TrackEventInfo { uuid, name });
        Ok(())
    }

    /// Creates a track for a new cell, the "Non-id-ed groups" track within the cell,
    /// and all group thread tracks within the cell and adds information about the cell.
    pub fn register_cell(
        &mut self,
        cell_id: CellId,
        name: String,
        component_par_track_opt: Option<&FxHashMap<String, u32>>,
    ) -> Result<(Uuid, FxHashMap<u32, u64>)> {
        let uuid = self.timeline.register_descriptor(name.clone(), None)?;
        self.cell_to_info
            .insert(cell_id, TrackEventInfo { uuid, name });

        // par_track --> track uuid for groups
        let mut thread_tracks: FxHashMap<u32, u64> = FxHashMap::default();

        // create additional "Non-id-ed groups" track for structurally enabled groups
        let non_id_name = "Non-id-ed groups".to_string();
        let non_id_uuid =
            self.timeline.register_descriptor(non_id_name, Some(uuid))?;
        thread_tracks.insert(NON_ID_THREAD, non_id_uuid);

        // create all thread tracks ahead of time
        if let Some(component_par_tracks) = component_par_track_opt {
            for thread in component_par_tracks.values() {
                if !thread_tracks.contains_key(thread) {
                    let thread_name = format!("Thread {:03}", thread);
                    let thread_uuid = self
                        .timeline
                        .register_descriptor(thread_name, Some(uuid))?;
                    thread_tracks.insert(*thread, thread_uuid);
                    if *thread == 0 {
                        self.track_zero_to_backups.insert(
                            thread_uuid,
                            TrackZeroInfo::new(thread_uuid),
                        );
                    }
                }
            }
        }
        Ok((uuid, thread_tracks))
    }

    /// Create the "Control groups" track, which will be the parent of all control groups.
    pub fn register_control_groups_track(
        &mut self,
        cell_uuid: Uuid,
    ) -> Result<Uuid> {
        self.timeline
            .register_descriptor("Control Groups".to_string(), Some(cell_uuid))
    }

    /// Creates the "Control Register Updates" track under a cell (whose UUID is passed in)
    /// and maps each control register in the Cell to the UUID of the created track.
    pub fn register_control_registers(
        &mut self,
        cell_uuid: Uuid,
        register_ids: &SmallVec<[RegisterId; 6]>,
    ) -> Result<()> {
        // register "Control Register Updates" track and keep track of its UUID
        let registers_uuid = self.timeline.register_descriptor(
            "Control Register Updates".to_string(),
            Some(cell_uuid),
        )?;
        for r in register_ids {
            self.register_to_uuid.insert(*r, registers_uuid);
        }
        Ok(())
    }

    /// Creates a track for a single control group, under the "Control groups" track (control_groups_uuid)
    /// and adds information about the control group.
    pub fn register_control(
        &mut self,
        track_name: String,
        name: String,
        control_id: ControlId,
        control_groups_uuid: Uuid,
    ) -> Result<Uuid> {
        let uuid = self.timeline.register_descriptor(
            track_name.clone(),
            Some(control_groups_uuid),
        )?;
        self.control_to_info
            .insert(control_id, TrackEventInfo { uuid, name });
        Ok(uuid)
    }

    /// Adds events for cells/control/groups that started or ended.
    pub fn update(
        &mut self,
        started: &CurrentlyActive,
        ended: &CurrentlyActive,
        cycle_count: u64,
    ) -> Result<()> {
        // register all end events
        self.update_helper(ended, cycle_count, Type::SliceEnd)?;
        // register all start events
        self.update_helper(started, cycle_count, Type::SliceBegin)?;
        Ok(())
    }

    pub fn register_event(
        &mut self,
        name: String,
        uuid: Uuid,
        timestamp: u64,
        event_type: Type,
    ) {
        self.timeline
            .register_event(name, uuid, timestamp, event_type);
    }

    pub fn output_timeline(self, out_dir: &str) -> anyhow::Result<()> {
        self.timeline
            .output_timeline(out_dir, "timeline_trace.pftrace")
    }
}

impl CalyxTimeline {
    /// Helper function of update_timeline.
    /// Updates the timeline baesed on the diff of active cells/groups/control between
    /// the previous cycle and this cycle.
    fn update_helper(
        &mut self,
        diff: &CurrentlyActive,
        cycle_count: u64,
        event_type: Type,
    ) -> Result<()> {
        for &cell in diff.cells.iter() {
            let TrackEventInfo { uuid, name } =
                self.cell_to_info.get(&cell).unwrap();
            self.timeline.register_event(
                name.clone(),
                *uuid,
                cycle_count,
                event_type,
            );
        }

        for &control in diff.control.iter() {
            let TrackEventInfo { uuid, name } =
                self.control_to_info.get(&control).unwrap();
            self.timeline.register_event(
                name.clone(),
                *uuid,
                cycle_count,
                event_type,
            );
        }

        for &group in diff.groups.iter() {
            let TrackEventInfo { uuid, name } =
                self.group_to_info.get(&group).unwrap();
            let real_uuid: Uuid =
                if let Some(tz) = self.track_zero_to_backups.get_mut(uuid) {
                    match event_type {
                        Type::SliceBegin => {
                            tz.start_group(&mut self.timeline, group)?
                        }
                        Type::SliceEnd => tz.end_group(group)?,
                        _ => {
                            panic!("Unexpected event type")
                        }
                    }
                } else {
                    *uuid
                };
            self.timeline.register_event(
                name.clone(),
                real_uuid,
                cycle_count,
                event_type,
            );
        }

        Ok(())
    }
}

/// Parse file for determining which thread each group enable belongs to.
pub fn read_par_tracks(
    fname: String,
) -> Result<FxHashMap<String, FxHashMap<String, u32>>> {
    let f = File::open(fname)?;
    let par_track_map: FxHashMap<String, FxHashMap<String, u32>> =
        serde_json::from_reader(f)?;
    Ok(par_track_map)
}
