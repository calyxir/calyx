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

pub struct Timeline {
    packets: Vec<TracePacket>,
    /// Cell/Control Group/Group name to track UUID
    // name_to_uuid: FxHashMap<String, u64>,
    used_uuids: FxHashSet<u64>,
    cell_to_uuid: FxHashMap<CellId, UUID>,
    control_to_uuid: FxHashMap<ControlId, UUID>,
    group_to_uuid: FxHashMap<GroupId, UUID>,
}

fn read_par_tracks(
    fname: String,
) -> Result<FxHashMap<String, FxHashMap<String, u32>>> {
    let f = File::open(fname)?;
    let par_track_map: FxHashMap<String, FxHashMap<String, u32>> =
        serde_json::from_reader(f)?;
    Ok(par_track_map)
}

impl Timeline {
    pub fn new(par_tracks_filename: String, d: &Design) -> Result<Self> {
        let par_tracks = read_par_tracks(par_tracks_filename)?;
        let mut s = Self {
            packets: Vec::new(),
            used_uuids: FxHashSet::default(),
            cell_to_uuid: FxHashMap::default(),
            control_to_uuid: FxHashMap::default(),
            group_to_uuid: FxHashMap::default(),
        };

        Ok(s)
    }
    pub fn register_group(
        &mut self,
        group_id: GroupId,
        uuid: UUID,
    ) -> Result<()> {
        self.group_to_uuid.insert(group_id, uuid);
        Ok(())
    }

    pub fn register_cell(
        &mut self,
        cell_id: CellId,
        name: String,
    ) -> Result<UUID> {
        let cell_uuid = self.register_descriptor(name, None)?;
        self.cell_to_uuid.insert(cell_id, cell_uuid);
        Ok(cell_uuid)
    }

    pub fn register_control(
        &mut self,
        name: String,
        control_id: ControlId,
        control_groups_uuid: UUID,
    ) -> Result<UUID> {
        let control_uuid =
            self.register_descriptor(name, Some(control_groups_uuid))?;
        self.control_to_uuid.insert(control_id, control_uuid);
        Ok(control_uuid)
    }

    /// TODO: scheme for generating uuids
    pub fn register_descriptor(
        &mut self,
        name: String,
        parent_uuid: Option<UUID>,
    ) -> Result<UUID> {
        // let parent_uuid = if let Some(parent) = parent_opt {
        //     let pu = self.name_to_uuid.get(&parent);
        //     assert!(pu.is_some());
        //     Some(*pu.unwrap())
        // } else {
        //     None
        // };
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
        track_name: &str,
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
