use crate::visuals::perfetto_protos::trace_packet::{
    Data, OptionalTrustedPacketSequenceId,
};
use crate::visuals::perfetto_protos::track_descriptor::StaticOrDynamicName;
use crate::visuals::perfetto_protos::track_event::{NameField, Type};
use crate::visuals::perfetto_protos::{
    Trace, TracePacket, TrackDescriptor, TrackEvent,
};
use anyhow::Result;
use prost::Message;
use prost::bytes::BytesMut;
use rustc_hash::FxHashSet;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

/// Trusted Packet Sequence ID; a number necessary for constructing the timeline protobuf
pub const TPSI: u32 = 8008;

/// A unique identifier used for specifying tracks in the timeline.
pub type Uuid = u64;

#[derive(Debug)]
pub(crate) struct Timeline {
    used_uuids: FxHashSet<u64>,
    output_file: File,
}

impl Timeline {
    pub fn new(
        out_dir: &str,
        out_file_name: &str, // "timeline_trace.pftrace"
    ) -> Result<Self> {
        let mut path = PathBuf::from(out_dir);
        path.push(out_file_name);
        // let mut file = File::create(path)?;
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            used_uuids: FxHashSet::default(),
            output_file: file,
        })
    }

    /// Creates a new track in the timeline.
    pub fn register_descriptor(
        &mut self,
        name: String,
        parent_uuid: Option<Uuid>,
    ) -> anyhow::Result<Uuid> {
        let uuid = generate_uuid(&self.used_uuids);
        self.used_uuids.insert(uuid);

        let descriptor = TrackDescriptor {
            uuid: Some(uuid),
            static_or_dynamic_name: Some(StaticOrDynamicName::Name(name)),
            parent_uuid,
            ..Default::default()
        };
        let packet = create_packet_helper(0, Data::TrackDescriptor(descriptor));
        self.push_packet(packet)?;
        anyhow::Ok(uuid)
    }

    /// Creates a new event in the timeline.
    pub fn register_event(
        &mut self,
        name: String,
        uuid: Uuid,
        timestamp: u64,
        event_type: Type,
    ) -> Result<()> {
        let event = TrackEvent {
            name_field: Some(NameField::Name(name)),
            r#type: Some(event_type as i32),
            track_uuid: Some(uuid),
            ..Default::default()
        };
        let packet = create_packet_helper(timestamp, Data::TrackEvent(event));
        self.push_packet(packet)
    }

    /// Outputs a packet onto the timeline file. (We write packets one by one
    /// to avoid an out-of-memory error.)
    pub fn push_packet(&mut self, packet: TracePacket) -> Result<()> {
        let trace = Trace {
            packet: vec![packet],
        };
        let encoded_len = trace.encoded_len();
        let mut buf = BytesMut::with_capacity(encoded_len);
        trace.encode(&mut buf)?;
        self.output_file.write_all(&buf)?;
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

/// Information necessary to add events of a cell/group/control to the timeline view.
#[derive(Clone, Debug, Default)]
pub struct TrackEventInfo {
    pub uuid: Uuid,
    pub name: String,
}
