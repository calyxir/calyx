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
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;

/// Trusted Packet Sequence ID; a number necessary
const tpsi: u32 = 8008;

pub struct Timeline {
    packets: Vec<TracePacket>,
    track_name_to_uuid: FxHashMap<String, u64>,
    used_uuids: FxHashSet<u64>,
}

impl Timeline {
    pub fn new() -> Self {
        Self {
            packets: Vec::new(),
            track_name_to_uuid: FxHashMap::default(),
            used_uuids: FxHashSet::default(),
        }
    }

    /// TODO: scheme for generating uuids
    pub fn register_descriptor(
        &mut self,
        name: String,
        parent_opt: Option<String>,
    ) {
        let parent_uuid = if let Some(parent) = parent_opt {
            let pu = self.track_name_to_uuid.get(&parent);
            assert!(pu.is_some());
            Some(*pu.unwrap())
        } else {
            None
        };
        let uuid = generate_uuid(&self.used_uuids);
        self.used_uuids.insert(uuid);
        self.track_name_to_uuid.insert(name.clone(), uuid);
        let descriptor = TrackDescriptor {
            uuid: Some(uuid),
            static_or_dynamic_name: Some(StaticOrDynamicName::Name(name)),
            parent_uuid,
            ..Default::default()
        };
        let packet = create_packet_helper(0, Data::TrackDescriptor(descriptor));
        self.packets.push(packet);
    }

    pub fn register_event(
        &mut self,
        name: String,
        track_name: &str,
        timestamp: u64,
        event_type: Type,
    ) {
        let track_uuid = self.track_name_to_uuid[track_name];
        let event = TrackEvent {
            name_field: Some(NameField::Name(name)),
            r#type: Some(event_type as i32),
            track_uuid: Some(track_uuid),
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

pub fn write_pftrace_attempt() -> Result<()> {
    let mut t = Timeline::new();
    t.register_descriptor("My sample track parent".to_string(), None);
    t.register_descriptor(
        "My sample track".to_string(),
        Some("My sample track parent".to_string()),
    );
    t.register_event(
        "hello".to_string(),
        "My sample track",
        0,
        Type::SliceBegin,
    );
    t.register_event(
        "hello".to_string(),
        "My sample track",
        100,
        Type::SliceEnd,
    );
    t.output_timeline("custom2.pftrace")?;
    Ok(())
}

// fn write_pftrace_attempt() -> anyhow::Result<()> {
//     let mut track_descriptor_parent = TrackDescriptor::default();
//     track_descriptor_parent.uuid = Some(100);
//     track_descriptor_parent.static_or_dynamic_name = Some(
//         StaticOrDynamicName::Name("My sample track parent".to_string()),
//     );
//     let mut track_descriptor_parent_event = TracePacket::default();
//     track_descriptor_parent_event.timestamp = Some(0);
//     track_descriptor_parent_event.data =
//         Some(Data::TrackDescriptor(track_descriptor_parent));
//     track_descriptor_parent_event.optional_trusted_packet_sequence_id =
//         Some(OptionalTrustedPacketSequenceId::TrustedPacketSequenceId(
//             TRUSTED_PACKET_SEQUENCE_ID,
//         ));
//
//     let mut track_descriptor_attempt = TrackDescriptor::default();
//     track_descriptor_attempt.uuid = Some(1);
//     track_descriptor_attempt.static_or_dynamic_name =
//         Some(StaticOrDynamicName::Name("My sample track".to_string()));
//     track_descriptor_attempt.parent_uuid = Some(100);
//     let mut track_descriptor_event = TracePacket::default();
//     track_descriptor_event.timestamp = Some(0);
//     track_descriptor_event.data =
//         Some(Data::TrackDescriptor(track_descriptor_attempt));
//     track_descriptor_event.optional_trusted_packet_sequence_id =
//         Some(OptionalTrustedPacketSequenceId::TrustedPacketSequenceId(
//             TRUSTED_PACKET_SEQUENCE_ID,
//         ));
//
//     let mut trace_packet_attempt = TracePacket::default();
//     trace_packet_attempt.timestamp = Some(0);
//     let mut track_event_attempt = TrackEvent::default();
//     // track_event_attempt.timestamp = Some(Timestamp::TimestampAbsoluteUs(0));
//     track_event_attempt.track_uuid = Some(1);
//     track_event_attempt.name_field = Some(NameField::Name("hello".to_string()));
//     track_event_attempt.r#type = Some(Type::SliceBegin as i32);
//     trace_packet_attempt.data = Some(Data::TrackEvent(track_event_attempt));
//     trace_packet_attempt.optional_trusted_packet_sequence_id =
//         Some(OptionalTrustedPacketSequenceId::TrustedPacketSequenceId(
//             TRUSTED_PACKET_SEQUENCE_ID,
//         ));
//     // trace_packet_attempt.timestamp_clock_id =
//     //     Some(BuiltinClocks::Realtime as u32);
//
//     let mut trace_packet_attempt2 = TracePacket::default();
//     trace_packet_attempt2.timestamp = Some(100);
//     let mut track_event_attempt2 = TrackEvent::default();
//     // track_event_attempt2.timestamp = Some(Timestamp::TimestampAbsoluteUs(100));
//     track_event_attempt2.track_uuid = Some(1);
//     track_event_attempt2.name_field =
//         Some(NameField::Name("hello".to_string()));
//     track_event_attempt2.r#type = Some(Type::SliceEnd as i32);
//     trace_packet_attempt2.data = Some(Data::TrackEvent(track_event_attempt2));
//     trace_packet_attempt2.optional_trusted_packet_sequence_id =
//         Some(OptionalTrustedPacketSequenceId::TrustedPacketSequenceId(
//             TRUSTED_PACKET_SEQUENCE_ID,
//         ));
//     // trace_packet_attempt2.timestamp_clock_id =
//     //     Some(BuiltinClocks::Realtime as u32);
//     let trace = Trace {
//         packet: vec![
//             // clock_msg,
//             track_descriptor_parent_event,
//             track_descriptor_event,
//             trace_packet_attempt,
//             trace_packet_attempt2,
//         ],
//     };
//     let encoded_len = trace.encoded_len();
//     let mut buf = BytesMut::with_capacity(encoded_len);
//     trace.encode(&mut buf)?;
//     let mut file = File::create("custom.pftrace")?;
//     file.write_all(&buf)?;
//     anyhow::Ok(())
// }

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
