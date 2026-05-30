use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::thread;
use std::time::{Duration, Instant};

use super::*;
use crate::protocol::{
    PACKET_SIZE, PKT_TYPE_DISCOVERY_REQUEST, PROTOCOL_VERSION, encode_peer_info, parse_packet,
};

/// Spawn a loopback thread that mimics the primary's behavior: bind a
/// UDP socket on `127.0.0.1`, wait for one discovery request, then send
/// each `reply` packet back to the requester's address. Returns the
/// socket address tests should target via `DeviceDiscovery::with_target`.
fn spawn_fake_primary(replies: Vec<[u8; PACKET_SIZE]>) -> (SocketAddr, thread::JoinHandle<()>) {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let addr = socket.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let mut buf = [0u8; PACKET_SIZE];
        let Ok((n, src)) = socket.recv_from(&mut buf) else {
            return;
        };
        let Some(p) = parse_packet(&buf[..n]) else {
            return;
        };
        if p.pkt_type != PKT_TYPE_DISCOVERY_REQUEST {
            return;
        }
        for reply in replies {
            socket.send_to(&reply, src).unwrap();
        }
    });
    (addr, handle)
}

#[test]
fn no_reply_returns_empty_within_budget() {
    // Bind a port and keep it alive (so the kernel doesn't send ICMP
    // unreachable back, which would surface as an io::Error on the next
    // recv), but never reply. Discover should time out and return Ok(empty).
    let silent = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let target = silent.local_addr().unwrap();
    let disc = DeviceDiscovery::with_target(target);
    let start = Instant::now();
    let result = disc.discover().unwrap();
    drop(silent);
    assert!(result.is_empty());
    // First-wait is 200 ms; we should be well under the 500 ms total cap.
    assert!(
        start.elapsed() < Duration::from_millis(450),
        "discover should return within the first-wait window, took {:?}",
        start.elapsed(),
    );
}

#[test]
fn single_reply_is_returned_with_compatible_true() {
    let reply = encode_peer_info(0x1234, true);
    let (addr, handle) = spawn_fake_primary(vec![reply]);
    let disc = DeviceDiscovery::with_target(addr);
    let result = disc.discover().unwrap();
    handle.join().unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].serial, 0x1234);
    assert!(result[0].compatible);
    assert_eq!(result[0].ip, IpAddr::V4(Ipv4Addr::LOCALHOST));
}

#[test]
fn version_mismatch_marks_incompatible() {
    let mut reply = encode_peer_info(0x9999, false);
    reply[4..6].copy_from_slice(&(PROTOCOL_VERSION + 1).to_be_bytes());
    let (addr, handle) = spawn_fake_primary(vec![reply]);
    let disc = DeviceDiscovery::with_target(addr);
    let result = disc.discover().unwrap();
    handle.join().unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].serial, 0x9999);
    assert!(!result[0].compatible);
}

#[test]
fn duplicate_serial_is_filtered() {
    // Simulate a network echo by replying twice with the same packet.
    // The discovery loop should keep only one entry.
    let reply = encode_peer_info(0xABCD, true);
    let (addr, handle) = spawn_fake_primary(vec![reply, reply]);
    let disc = DeviceDiscovery::with_target(addr);
    let result = disc.discover().unwrap();
    handle.join().unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].serial, 0xABCD);
}
