//! Wire protocol shared by the Enpose API and the on-device daemon.
//!
//! All packets are exactly [`PACKET_SIZE`] bytes laid out big-endian on
//! the wire so a packet capture shows the literal magic bytes `EnpR`
//! regardless of host byte order.
//!
//! Packet layout:
//!
//! ```text
//! offset  size  field
//! 0       4     MAGIC ("EnpR")
//! 4       2     PROTOCOL_VERSION
//! 6       4     serial number
//! 10      1     has_extrinsics flag (0 or 1)
//! 11      1     packet type (PKT_TYPE_*)
//! ```

/// UDP port the Enpose role-negotiation and discovery protocol uses.
///
/// Devices broadcast peer-announcement packets on this port at 1 Hz,
/// and clients send discovery requests to this port. The primary
/// device of every cluster replies to discovery requests by unicast
/// from this port back to the requester's ephemeral port.
pub const BROADCAST_PORT: u16 = 50884;

/// Wire-protocol version this API was built against.
///
/// Bumped only on incompatible packet-format changes. Packets carrying
/// a different version are still surfaced by [`crate::DeviceDiscovery`]
/// (with `compatible = false`) so the caller can present a helpful
/// "upgrade your firmware / client" entry instead of silently dropping
/// the device.
pub const PROTOCOL_VERSION: u16 = 1;

/// Magic prefix of every packet — the ASCII bytes `EnpR` interpreted
/// as a big-endian `u32`. Distinguishes Enpose traffic from any other
/// UDP datagram that happens to land on [`BROADCAST_PORT`].
pub const MAGIC: u32 = 0x456e7052;

/// Fixed packet size across all packet types, so receivers can use a
/// single `recv_from` buffer.
pub const PACKET_SIZE: usize = 12;

/// Packet type: a device announces its own identity (serial,
/// extrinsics-calibration state). Sent both as the 1 Hz cluster
/// broadcast and as the unicast reply to a discovery request.
pub const PKT_TYPE_PEER_INFO: u8 = 0;

/// Packet type: a client asks any reachable primary to identify
/// itself. Only the cluster's elected primary replies, with a
/// [`PKT_TYPE_PEER_INFO`] packet sent unicast to the requester.
pub const PKT_TYPE_DISCOVERY_REQUEST: u8 = 1;

/// UDP port the pose-streaming protocol uses. Clients send subscribe /
/// keep-alive packets to this port on the device's primary, and the
/// device unicasts [`PKT_TYPE_POSE_DATA`] packets back to each subscribed
/// client. Separate from [`BROADCAST_PORT`] so discovery and streaming
/// traffic never share a socket.
pub const POSE_PORT: u16 = 50885;

/// Packet type: a client subscribes to the pose stream. The same packet
/// doubles as the keep-alive — a client resends it at 1 Hz, and the
/// device drops a client it has not heard from within
/// [`POSE_KEEPALIVE_TIMEOUT_SECS`]. Sent client → device on
/// [`POSE_PORT`].
pub const PKT_TYPE_POSE_SUBSCRIBE: u8 = 2;

/// Packet type: a client unsubscribes from the pose stream. Lets the
/// device drop the client immediately instead of waiting for the
/// keep-alive timeout. Sent client → device on [`POSE_PORT`].
pub const PKT_TYPE_POSE_UNSUBSCRIBE: u8 = 3;

/// Packet type: a pose-data datagram. The fixed [`PACKET_SIZE`] header is
/// followed by a MessagePack-encoded `Vec<MarkerPose>` starting at offset
/// [`PACKET_SIZE`]. One datagram carries all markers localized from a
/// single camera frame. Sent device → client on [`POSE_PORT`].
pub const PKT_TYPE_POSE_DATA: u8 = 4;

/// How long the device keeps a pose-stream client without hearing a
/// subscribe/keep-alive packet from it before dropping the connection.
pub const POSE_KEEPALIVE_TIMEOUT_SECS: u64 = 5;

/// Interval at which a pose-stream client should resend its
/// subscribe/keep-alive packet to stay connected.
pub const POSE_KEEPALIVE_INTERVAL_SECS: u64 = 1;

/// Decoded contents of a packet that passed the magic-bytes check.
///
/// The `version` field is intentionally not validated by
/// [`parse_packet`]; callers decide whether to drop a version-mismatch
/// packet (the daemon does this for peer announcements) or surface it
/// to the user (discovery clients do this so an incompatible device
/// can still be listed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedPacket {
    pub version: u16,
    pub serial: u32,
    pub has_extrinsics: bool,
    pub pkt_type: u8,
}

/// Build a peer-info packet using this API's current
/// [`PROTOCOL_VERSION`].
pub fn encode_peer_info(serial: u32, has_extrinsics: bool) -> [u8; PACKET_SIZE] {
    encode(serial, has_extrinsics, PKT_TYPE_PEER_INFO)
}

/// Build a discovery-request packet. Carries `serial = 0` because the
/// requester is anonymous — the replying device fills its own serial
/// into the response.
pub fn encode_discovery_request() -> [u8; PACKET_SIZE] {
    encode(0, false, PKT_TYPE_DISCOVERY_REQUEST)
}

/// Build a pose-stream subscribe / keep-alive packet. Carries
/// `serial = 0` because the client is anonymous; the device identifies
/// the client by its source address.
pub fn encode_pose_subscribe() -> [u8; PACKET_SIZE] {
    encode(0, false, PKT_TYPE_POSE_SUBSCRIBE)
}

/// Build a pose-stream unsubscribe packet.
pub fn encode_pose_unsubscribe() -> [u8; PACKET_SIZE] {
    encode(0, false, PKT_TYPE_POSE_UNSUBSCRIBE)
}

/// Build the fixed header of a [`PKT_TYPE_POSE_DATA`] packet. The caller
/// appends the MessagePack-encoded pose payload after this header; the
/// receiver decodes the payload from offset [`PACKET_SIZE`]. `serial` is
/// the sending device's factory serial.
pub fn encode_pose_data_header(serial: u32) -> [u8; PACKET_SIZE] {
    encode(serial, false, PKT_TYPE_POSE_DATA)
}

fn encode(serial: u32, has_extrinsics: bool, pkt_type: u8) -> [u8; PACKET_SIZE] {
    let mut buf = [0u8; PACKET_SIZE];
    buf[0..4].copy_from_slice(&MAGIC.to_be_bytes());
    buf[4..6].copy_from_slice(&PROTOCOL_VERSION.to_be_bytes());
    buf[6..10].copy_from_slice(&serial.to_be_bytes());
    buf[10] = has_extrinsics as u8;
    buf[11] = pkt_type;
    buf
}

/// Decode a packet.
///
/// Returns `None` only when the buffer is shorter than [`PACKET_SIZE`]
/// or when the magic prefix does not match — those are the conditions
/// that mean "this is not an Enpose packet at all".
///
/// A packet with an unrecognised [`ParsedPacket::pkt_type`] or a
/// [`ParsedPacket::version`] different from [`PROTOCOL_VERSION`] is
/// returned to the caller unmodified; rejection policy is the
/// caller's choice.
pub fn parse_packet(data: &[u8]) -> Option<ParsedPacket> {
    if data.len() < PACKET_SIZE {
        return None;
    }
    let magic = u32::from_be_bytes(data[0..4].try_into().expect("length checked above"));
    if magic != MAGIC {
        return None;
    }
    let version = u16::from_be_bytes(data[4..6].try_into().expect("length checked above"));
    let serial = u32::from_be_bytes(data[6..10].try_into().expect("length checked above"));
    let has_extrinsics = data[10] != 0;
    let pkt_type = data[11];
    Some(ParsedPacket {
        version,
        serial,
        has_extrinsics,
        pkt_type,
    })
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
