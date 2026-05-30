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
