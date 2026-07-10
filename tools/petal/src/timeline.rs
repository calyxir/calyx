use crate::design::{CellId, ControlId, Design, GroupId};
use anyhow::{Context, Ok, Result, anyhow};
use perfetto_trace_proto::trace_packet::{
    Data, OptionalTrustedPacketSequenceId,
};
use perfetto_trace_proto::track_descriptor::StaticOrDynamicName;
use perfetto_trace_proto::track_event::{NameField, Type};
use perfetto_trace_proto::{Trace, TracePacket, TrackDescriptor, TrackEvent};
use prost::Message;
use prost::bytes::BytesMut;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::Write;

/// Trusted Packet Sequence ID; a number necessary
const tpsi: u32 = 8008;

type UUID = u64;

#[derive(Clone, Debug)]
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

    pub fn add_active_group(&mut self, group: GroupId) {
        self.groups.insert(group);
    }

    pub fn add_active_cell(&mut self, cell: CellId) {
        self.cells.insert(cell);
    }

    pub fn add_active_control(&mut self, control: ControlId) {
        self.control.insert(control);
    }

    /// Results the diffs between existing active cells/control/groups vs active cells/control/groups
    /// Returns (Ended cells/control/groups, Started cells/control/groups)
    pub fn resolve(&self, new: &Self) -> Result<(Self, Self)> {
        let ended = Self {
            groups: self.groups.difference(&new.groups).map(|&g| g).collect(),
            cells: self.cells.difference(&new.cells).map(|&c| c).collect(),
            control: self
                .control
                .difference(&new.control)
                .map(|&c| c)
                .collect(),
        };
        let started = Self {
            groups: new
                .groups
                .difference(&self.groups)
                .map(|&g| g.clone())
                .collect(),
            cells: new
                .cells
                .difference(&self.cells)
                .map(|&c| c.clone())
                .collect(),
            control: new
                .control
                .difference(&self.control)
                .map(|&c| c.clone())
                .collect(),
        };
        Ok((ended, started))
    }
}

pub struct Timeline {
    packets: Vec<TracePacket>,
    /// Cell/Control Group/Group name to track UUID
    // name_to_uuid: FxHashMap<String, u64>,
    used_uuids: FxHashSet<u64>,
    cell_to_info: FxHashMap<CellId, (UUID, String)>,
    control_to_info: FxHashMap<ControlId, (UUID, String)>,
    group_to_info: FxHashMap<GroupId, (UUID, String)>,
    current_active: CurrentlyActive,
}

impl Timeline {
    pub fn new(d: &Design) -> Result<Self> {
        let s = Self {
            packets: Vec::new(),
            used_uuids: FxHashSet::default(),
            cell_to_info: FxHashMap::default(),
            control_to_info: FxHashMap::default(),
            group_to_info: FxHashMap::default(),
            current_active: CurrentlyActive::new(),
        };
        Ok(s)
    }
    pub fn register_group(
        &mut self,
        group_id: GroupId,
        uuid: UUID,
        name: &str,
    ) -> Result<()> {
        self.group_to_info
            .insert(group_id, (uuid, name.to_string()));
        Ok(())
    }

    pub fn register_cell(
        &mut self,
        cell_id: CellId,
        name: String,
    ) -> Result<UUID> {
        let cell_uuid = self.register_descriptor(name.clone(), None)?;
        self.cell_to_info
            .insert(cell_id, (cell_uuid, name.to_string()));
        Ok(cell_uuid)
    }

    pub fn register_control(
        &mut self,
        track_name: String,
        name: String,
        control_id: ControlId,
        control_groups_uuid: UUID,
    ) -> Result<UUID> {
        let control_uuid = self.register_descriptor(
            track_name.clone(),
            Some(control_groups_uuid),
        )?;
        self.control_to_info
            .insert(control_id, (control_uuid, name.to_string()));
        Ok(control_uuid)
    }

    pub fn register_descriptor(
        &mut self,
        name: String,
        parent_uuid: Option<UUID>,
    ) -> Result<UUID> {
        let uuid = generate_uuid(&self.used_uuids);
        self.used_uuids.insert(uuid);

        let descriptor = TrackDescriptor {
            uuid: Some(uuid.clone()),
            static_or_dynamic_name: Some(StaticOrDynamicName::Name(name)),
            parent_uuid,
            ..Default::default()
        };
        let packet = create_packet_helper(0, Data::TrackDescriptor(descriptor));
        self.packets.push(packet);
        Ok(uuid)
    }

    pub fn register_event(
        &mut self,
        name: String,
        uuid: UUID,
        timestamp: u64,
        event_type: Type,
    ) {
        // let track_uuid = self.name_to_uuid[track_name];
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

    pub fn output_timeline(self, file_name: &str) -> Result<()> {
        // we can move self.packets because we will no longer add any information to it.
        let trace = Trace {
            packet: self.packets,
        };
        let encoded_len = trace.encoded_len();
        let mut buf = BytesMut::with_capacity(encoded_len);
        trace.encode(&mut buf)?;
        let mut file = File::create(file_name)?;
        file.write_all(&buf)?;
        Ok(())
    }

    fn update(
        &mut self,
        diff: &CurrentlyActive,
        cycle_count: u64,
        event_type: Type,
    ) {
        for &cell in diff.cells.iter() {
            let (cell_uuid, cell_name) = self.cell_to_info.get(&cell).unwrap();
            self.register_event(
                cell_name.clone(),
                *cell_uuid,
                cycle_count,
                event_type,
            );
        }

        for &control in diff.control.iter() {
            let (control_uuid, control_name) =
                self.control_to_info.get(&control).unwrap();
            self.register_event(
                control_name.clone(),
                *control_uuid,
                cycle_count,
                event_type,
            );
        }

        for &group in diff.groups.iter() {
            if !self.group_to_info.contains_key(&group) {
                println!("Group without info: {group:?}");
            }
            let (group_uuid, group_name) =
                self.group_to_info.get(&group).unwrap();
            self.register_event(
                group_name.clone(),
                *group_uuid,
                cycle_count,
                event_type,
            );
        }
    }

    pub fn update_timeline(
        &mut self,
        active_this_cycle: &CurrentlyActive,
        cycle_count: u64,
    ) -> Result<()> {
        let (ended, started) =
            self.current_active.resolve(active_this_cycle)?;
        // register all end events
        self.update(&ended, cycle_count, Type::SliceEnd);
        // register all start events
        self.update(&started, cycle_count, Type::SliceBegin);

        self.current_active = active_this_cycle.clone();
        Ok(())
    }
}

fn generate_uuid(used_uuids: &FxHashSet<u64>) -> u64 {
    let mut r = rand::random::<u64>();
    while used_uuids.contains(&r) {
        r = rand::random::<u64>();
    }
    r
}

fn create_packet_helper(timestamp: u64, data: Data) -> TracePacket {
    TracePacket {
        timestamp: Some(timestamp),
        data: Some(data),
        optional_trusted_packet_sequence_id: Some(
            OptionalTrustedPacketSequenceId::TrustedPacketSequenceId(tpsi),
        ),
        ..Default::default()
    }
}

pub fn read_par_tracks(
    fname: String,
) -> Result<FxHashMap<String, FxHashMap<String, u32>>> {
    let f = File::open(fname)?;
    let par_track_map: FxHashMap<String, FxHashMap<String, u32>> =
        serde_json::from_reader(f)?;
    Ok(par_track_map)
}
