use super::*;

#[test]
fn peer_info_round_trip() {
    let buf = encode_peer_info(0xDEADBEEF, true);
    let p = parse_packet(&buf).expect("valid packet");
    assert_eq!(p.serial, 0xDEADBEEF);
    assert!(p.has_extrinsics);
    assert_eq!(p.pkt_type, PKT_TYPE_PEER_INFO);
    assert_eq!(p.version, PROTOCOL_VERSION);
}

#[test]
fn discovery_request_round_trip() {
    let buf = encode_discovery_request();
    let p = parse_packet(&buf).expect("valid packet");
    assert_eq!(p.pkt_type, PKT_TYPE_DISCOVERY_REQUEST);
    assert_eq!(p.serial, 0);
    assert!(!p.has_extrinsics);
    assert_eq!(p.version, PROTOCOL_VERSION);
}

#[test]
fn pose_subscribe_round_trip() {
    let buf = encode_pose_subscribe();
    let p = parse_packet(&buf).expect("valid packet");
    assert_eq!(p.pkt_type, PKT_TYPE_POSE_SUBSCRIBE);
    assert_eq!(p.serial, 0);
    assert_eq!(p.version, PROTOCOL_VERSION);
}

#[test]
fn pose_unsubscribe_round_trip() {
    let buf = encode_pose_unsubscribe();
    let p = parse_packet(&buf).expect("valid packet");
    assert_eq!(p.pkt_type, PKT_TYPE_POSE_UNSUBSCRIBE);
}

#[test]
fn pose_data_header_carries_serial() {
    let buf = encode_pose_data_header(0xCAFEBABE);
    let p = parse_packet(&buf).expect("valid packet");
    assert_eq!(p.pkt_type, PKT_TYPE_POSE_DATA);
    assert_eq!(p.serial, 0xCAFEBABE);
}

#[test]
fn rejects_bad_magic() {
    let mut buf = encode_peer_info(1, false);
    buf[0] ^= 0xFF;
    assert!(parse_packet(&buf).is_none());
}

#[test]
fn rejects_short_packet() {
    let buf = encode_peer_info(1, false);
    assert!(parse_packet(&buf[..buf.len() - 1]).is_none());
}

#[test]
fn version_mismatch_is_surfaced_not_rejected() {
    // Discovery clients need to see version-mismatched packets so they
    // can list incompatible devices to the user. parse_packet therefore
    // accepts the packet and leaves the policy decision to the caller.
    let mut buf = encode_peer_info(1, false);
    buf[4..6].copy_from_slice(&(PROTOCOL_VERSION + 1).to_be_bytes());
    let p = parse_packet(&buf).expect("magic still matches, packet should parse");
    assert_eq!(p.version, PROTOCOL_VERSION + 1);
}

#[test]
fn magic_matches_ascii_enpr() {
    // 'E'=0x45, 'n'=0x6e, 'p'=0x70, 'R'=0x52 — big-endian on the wire so
    // a tcpdump shows the literal magic bytes EnpR.
    assert_eq!(MAGIC, 0x456e7052);
    let buf = encode_peer_info(0, false);
    assert_eq!(&buf[0..4], b"EnpR");
}
