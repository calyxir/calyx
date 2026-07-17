use crate::design::{CellId, ControlId, GroupId, RegisterId};
use crate::perfetto_protos::trace_packet::{
    Data, OptionalTrustedPacketSequenceId,
};
use crate::perfetto_protos::track_descriptor::StaticOrDynamicName;
use crate::perfetto_protos::track_event::{NameField, Type};
use crate::perfetto_protos::{Trace, TracePacket, TrackDescriptor, TrackEvent};
use anyhow::{Ok, Result};
use prost::Message;
use prost::bytes::BytesMut;
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

/// Trusted Packet Sequence ID; a number necessary for constructing the timeline protobuf
const TPSI: u32 = 8008;

/// A unique identifier used for specifying tracks in the timeline.
pub type Uuid = u64;

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

/// Information necessary to add events of a cell/grouo/control to the timeline view.
#[derive(Clone, Debug, Default)]
struct TrackEventInfo {
    uuid: Uuid,
    name: String,
}

/// Constructs and outputs protobuf messages for constructing a timeline view.
/// In the timeline, each component cell has a distinct track which contains tracks containing the
/// group/control group/control register activity within that cell, organized as below:
/// - Control register updates get a single track ("Control register updates")
/// - Each control group gets its own track, under the "Control groups" track
/// - Groups are organized by their statically defined thread IDs. Each thread gets its own track.
pub struct Timeline {
    packets: Vec<TracePacket>,
    /// Cell/Control Group/Group name to track UUID
    used_uuids: FxHashSet<u64>,
    cell_to_info: FxHashMap<CellId, TrackEventInfo>,
    control_to_info: FxHashMap<ControlId, TrackEventInfo>,
    group_to_info: FxHashMap<GroupId, TrackEventInfo>,
    register_to_uuid: FxHashMap<RegisterId, Uuid>,
}

impl Timeline {
    pub fn new() -> Result<Self> {
        let s = Self {
            packets: Vec::new(),
            used_uuids: FxHashSet::default(),
            cell_to_info: FxHashMap::default(),
            control_to_info: FxHashMap::default(),
            group_to_info: FxHashMap::default(),
            register_to_uuid: FxHashMap::default(),
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

    /// Creates a track for a new cell and adds information about the cell.
    pub fn register_cell(
        &mut self,
        cell_id: CellId,
        name: String,
    ) -> Result<Uuid> {
        let uuid = self.register_descriptor(name.clone(), None)?;
        self.cell_to_info
            .insert(cell_id, TrackEventInfo { uuid, name });
        Ok(uuid)
    }

    /// Creates the "Control Register Updates" track under a cell (whose UUID is passed in)
    /// and maps each control register in the Cell to the UUID of the created track.
    pub fn register_control_registers(
        &mut self,
        cell_uuid: Uuid,
        register_ids: &SmallVec<[RegisterId; 6]>,
    ) -> Result<()> {
        // register "Control Register Updates" track and keep track of its UUID
        let registers_uuid = self.register_descriptor(
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
        let uuid = self.register_descriptor(
            track_name.clone(),
            Some(control_groups_uuid),
        )?;
        self.control_to_info
            .insert(control_id, TrackEventInfo { uuid, name });
        Ok(uuid)
    }

    /// Creates a new track in the timeline.
    pub fn register_descriptor(
        &mut self,
        name: String,
        parent_uuid: Option<Uuid>,
    ) -> Result<Uuid> {
        let uuid = generate_uuid(&self.used_uuids);
        self.used_uuids.insert(uuid);

        let descriptor = TrackDescriptor {
            uuid: Some(uuid),
            static_or_dynamic_name: Some(StaticOrDynamicName::Name(name)),
            parent_uuid,
            ..Default::default()
        };
        let packet = create_packet_helper(0, Data::TrackDescriptor(descriptor));
        self.packets.push(packet);
        Ok(uuid)
    }

    /// Creates a new event in the timeline.
    pub fn register_event(
        &mut self,
        name: String,
        uuid: Uuid,
        timestamp: u64,
        event_type: Type,
    ) {
        let event = TrackEvent {
            name_field: Some(NameField::Name(name)),
            r#type: Some(event_type as i32),
            track_uuid: Some(uuid),
            // TODO: find the track uuid for this event
            ..Default::default()
        };
        let packet = create_packet_helper(timestamp, Data::TrackEvent(event));
        self.packets.push(packet);
    }

    pub fn output_timeline(self, out_dir: &str) -> Result<()> {
        // we can move self.packets because we will no longer add any information to it.
        let trace = Trace {
            packet: self.packets,
        };
        let encoded_len = trace.encoded_len();
        let mut buf = BytesMut::with_capacity(encoded_len);
        trace.encode(&mut buf)?;
        let mut path = PathBuf::from(out_dir);
        path.push("timeline_trace.pftrace");
        let mut file = File::create(path)?;
        file.write_all(&buf)?;
        Ok(())
    }

    /// Adds events for cells/control/groups that started or ended.
    pub fn update(
        &mut self,
        started: &CurrentlyActive,
        ended: &CurrentlyActive,
        cycle_count: u64,
    ) -> Result<()> {
        // register all end events
        self.update_helper(ended, cycle_count, Type::SliceEnd);
        // register all start events
        self.update_helper(started, cycle_count, Type::SliceBegin);
        Ok(())
    }
}

impl Timeline {
    /// Helper function of update_timeline.
    /// Updates the timeline based on the diff of active cells/groups/control between
    /// the previous cycle and this cycle.
    fn update_helper(
        &mut self,
        diff: &CurrentlyActive,
        cycle_count: u64,
        event_type: Type,
    ) {
        for &cell in diff.cells.iter() {
            let TrackEventInfo { uuid, name } =
                self.cell_to_info.get(&cell).unwrap();
            self.register_event(name.clone(), *uuid, cycle_count, event_type);
        }

        for &control in diff.control.iter() {
            let TrackEventInfo { uuid, name } =
                self.control_to_info.get(&control).unwrap();
            self.register_event(name.clone(), *uuid, cycle_count, event_type);
        }

        for &group in diff.groups.iter() {
            let TrackEventInfo { uuid, name } =
                self.group_to_info.get(&group).unwrap();
            self.register_event(name.clone(), *uuid, cycle_count, event_type);
        }
    }
}

fn generate_uuid(used_uuids: &FxHashSet<u64>) -> u64 {
    let mut r = rand::random::<u64>();
    while used_uuids.contains(&r) {
        r = rand::random::<u64>();
    }
    r
}

/// Helper function for `register_descriptor()` and `register_event()`.
/// Descriptor specifications and events need to be wrapped in a TracePacket.
fn create_packet_helper(timestamp: u64, data: Data) -> TracePacket {
    TracePacket {
        timestamp: Some(timestamp),
        data: Some(data),
        optional_trusted_packet_sequence_id: Some(
            OptionalTrustedPacketSequenceId::TrustedPacketSequenceId(TPSI),
        ),
        ..Default::default()
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
