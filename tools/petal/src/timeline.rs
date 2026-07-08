use perfetto_trace_proto::trace_packet::{
    Data, OptionalTrustedPacketSequenceId,
};
use perfetto_trace_proto::track_descriptor::StaticOrDynamicName;
use perfetto_trace_proto::track_event::{NameField, Type};
use perfetto_trace_proto::{Trace, TracePacket, TrackDescriptor, TrackEvent};
use prost::bytes::BytesMut;
use std::fs::File;
use rustc_hash::FxHashMap;

/// Trusted Packet Sequence ID; a number necessary
const tpsi: u32 = 8008;

pub struct Timeline {
    packets: Vec<TracePacket>,
    name_to_uuid : FxHashMap<String, u64>,
}

impl Timeline {
    pub fn new(packets: Vec<TracePacket>) -> Self {
        Self { packets }
    }

    /// TODO: scheme for generating uuids
    pub fn register_descriptor(&self, name: String, parent_opt: Option<String>) {
        let parent_uuid = if let Some(parent) = parent_opt {
            let pu = self.name_to_uuid.get(&parent);
            assert!(pu.is_some());
            pu
        } else {
            None
        };
        let descriptor = TrackDescriptor {
            uuid =
        }
    }
}

fn generate_uuid() -> u64 {

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

fn write_pftrace_attempt() -> anyhow::Result<()> {
    let mut track_descriptor_parent = TrackDescriptor::default();
    track_descriptor_parent.uuid = Some(100);
    track_descriptor_parent.static_or_dynamic_name = Some(
        StaticOrDynamicName::Name("My sample track parent".to_string()),
    );
    let mut track_descriptor_parent_event = TracePacket::default();
    track_descriptor_parent_event.timestamp = Some(0);
    track_descriptor_parent_event.data =
        Some(Data::TrackDescriptor(track_descriptor_parent));
    track_descriptor_parent_event.optional_trusted_packet_sequence_id =
        Some(OptionalTrustedPacketSequenceId::TrustedPacketSequenceId(
            TRUSTED_PACKET_SEQUENCE_ID,
        ));

    let mut track_descriptor_attempt = TrackDescriptor::default();
    track_descriptor_attempt.uuid = Some(1);
    track_descriptor_attempt.static_or_dynamic_name =
        Some(StaticOrDynamicName::Name("My sample track".to_string()));
    track_descriptor_attempt.parent_uuid = Some(100);
    let mut track_descriptor_event = TracePacket::default();
    track_descriptor_event.timestamp = Some(0);
    track_descriptor_event.data =
        Some(Data::TrackDescriptor(track_descriptor_attempt));
    track_descriptor_event.optional_trusted_packet_sequence_id =
        Some(OptionalTrustedPacketSequenceId::TrustedPacketSequenceId(
            TRUSTED_PACKET_SEQUENCE_ID,
        ));

    let mut trace_packet_attempt = TracePacket::default();
    trace_packet_attempt.timestamp = Some(0);
    let mut track_event_attempt = TrackEvent::default();
    // track_event_attempt.timestamp = Some(Timestamp::TimestampAbsoluteUs(0));
    track_event_attempt.track_uuid = Some(1);
    track_event_attempt.name_field = Some(NameField::Name("hello".to_string()));
    track_event_attempt.r#type = Some(Type::SliceBegin as i32);
    trace_packet_attempt.data = Some(Data::TrackEvent(track_event_attempt));
    trace_packet_attempt.optional_trusted_packet_sequence_id =
        Some(OptionalTrustedPacketSequenceId::TrustedPacketSequenceId(
            TRUSTED_PACKET_SEQUENCE_ID,
        ));
    // trace_packet_attempt.timestamp_clock_id =
    //     Some(BuiltinClocks::Realtime as u32);

    let mut trace_packet_attempt2 = TracePacket::default();
    trace_packet_attempt2.timestamp = Some(100);
    let mut track_event_attempt2 = TrackEvent::default();
    // track_event_attempt2.timestamp = Some(Timestamp::TimestampAbsoluteUs(100));
    track_event_attempt2.track_uuid = Some(1);
    track_event_attempt2.name_field =
        Some(NameField::Name("hello".to_string()));
    track_event_attempt2.r#type = Some(Type::SliceEnd as i32);
    trace_packet_attempt2.data = Some(Data::TrackEvent(track_event_attempt2));
    trace_packet_attempt2.optional_trusted_packet_sequence_id =
        Some(OptionalTrustedPacketSequenceId::TrustedPacketSequenceId(
            TRUSTED_PACKET_SEQUENCE_ID,
        ));
    // trace_packet_attempt2.timestamp_clock_id =
    //     Some(BuiltinClocks::Realtime as u32);
    let trace = Trace {
        packet: vec![
            // clock_msg,
            track_descriptor_parent_event,
            track_descriptor_event,
            trace_packet_attempt,
            trace_packet_attempt2,
        ],
    };
    let encoded_len = trace.encoded_len();
    let mut buf = BytesMut::with_capacity(encoded_len);
    trace.encode(&mut buf)?;
    let mut file = File::create("custom.pftrace")?;
    file.write_all(&buf)?;
    anyhow::Ok(())
}
