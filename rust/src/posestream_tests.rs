use std::net::{Ipv4Addr, UdpSocket};
use std::time::Duration;

use super::*;
use crate::protocol::{
    PACKET_SIZE, PKT_TYPE_POSE_SUBSCRIBE, PKT_TYPE_POSE_UNSUBSCRIBE, encode_pose_data_header,
    parse_packet,
};

/// Build a sample pose for tests.
fn sample_pose(marker_id: u16) -> MarkerPose {
    MarkerPose {
        timestamp: 12345,
        marker_id,
        x: 1.0,
        y: 2.0,
        z: 3.0,
        rotation: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        position_rmse: 0.1,
        rotation_rmse: 0.01,
        sensors: 2,
    }
}

/// Encode a pose-data packet the way the device would: fixed header
/// followed by the binary-encoded pose batch.
fn encode_pose_data(serial: u32, poses: &[MarkerPose]) -> Vec<u8> {
    let mut packet = encode_pose_data_header(serial).to_vec();
    packet.extend(MarkerPose::encode_batch(poses));
    packet
}

/// Bind a loopback UDP socket to act as the device side of the stream.
fn fake_device() -> UdpSocket {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    socket.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    socket
}

#[test]
fn single_threaded_subscribe_then_receive() {
    let device = fake_device();
    let device_addr = device.local_addr().unwrap();

    let mut stream = PoseStream::with_target(device_addr, false).unwrap();

    // The constructor sends the initial subscribe; the device learns the
    // client's address from it.
    let mut buf = [0u8; PACKET_SIZE];
    let (n, client) = device.recv_from(&mut buf).unwrap();
    let parsed = parse_packet(&buf[..n]).expect("subscribe packet");
    assert_eq!(parsed.pkt_type, PKT_TYPE_POSE_SUBSCRIBE);

    // Device replies with a pose batch.
    let poses = vec![sample_pose(7), sample_pose(8)];
    device.send_to(&encode_pose_data(0xABCD, &poses), client).unwrap();

    // Give the datagram a moment to land, then drain.
    std::thread::sleep(Duration::from_millis(50));
    let received = stream.receive_pose_updates(false).unwrap();
    assert_eq!(received, poses);
}

#[test]
fn ignores_non_pose_packets() {
    let device = fake_device();
    let device_addr = device.local_addr().unwrap();

    let mut stream = PoseStream::with_target(device_addr, false).unwrap();
    let mut buf = [0u8; PACKET_SIZE];
    let (_, client) = device.recv_from(&mut buf).unwrap();

    // Send a control packet (not pose data) and a valid pose batch.
    device.send_to(&crate::protocol::encode_pose_subscribe(), client).unwrap();
    let poses = vec![sample_pose(1)];
    device.send_to(&encode_pose_data(1, &poses), client).unwrap();

    std::thread::sleep(Duration::from_millis(50));
    let received = stream.receive_pose_updates(false).unwrap();
    assert_eq!(received, poses);
}

#[test]
fn from_ip_rejects_ipv6() {
    // The Enpose API is IPv4-only; an IPv6 target is rejected up front with a
    // typed Unsupported error rather than an opaque connect failure.
    let result = PoseStream::from_ip(std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST), false);
    let Err(err) = result else {
        panic!("expected an error for an IPv6 target");
    };
    assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
}

#[test]
fn drop_sends_unsubscribe() {
    let device = fake_device();
    let device_addr = device.local_addr().unwrap();

    let stream = PoseStream::with_target(device_addr, false).unwrap();
    let mut buf = [0u8; PACKET_SIZE];
    let (_, client) = device.recv_from(&mut buf).unwrap();

    drop(stream);

    // The disconnect packet should arrive from the same client address.
    let (n, src) = device.recv_from(&mut buf).unwrap();
    assert_eq!(src, client);
    let parsed = parse_packet(&buf[..n]).expect("unsubscribe packet");
    assert_eq!(parsed.pkt_type, PKT_TYPE_POSE_UNSUBSCRIBE);
}

#[test]
fn threaded_mode_buffers_poses() {
    let device = fake_device();
    let device_addr = device.local_addr().unwrap();

    let mut stream = PoseStream::with_target(device_addr, true).unwrap();
    let mut buf = [0u8; PACKET_SIZE];
    let (n, client) = device.recv_from(&mut buf).unwrap();
    assert_eq!(
        parse_packet(&buf[..n]).unwrap().pkt_type,
        PKT_TYPE_POSE_SUBSCRIBE
    );

    let poses = vec![sample_pose(42)];
    device.send_to(&encode_pose_data(2, &poses), client).unwrap();

    // The background thread receives and buffers it; poll until it shows up.
    let mut received = Vec::new();
    for _ in 0..40 {
        received = stream.receive_pose_updates(false).unwrap();
        if !received.is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    assert_eq!(received, poses);
}

#[test]
fn single_threaded_block_waits_for_pose() {
    let device = fake_device();
    let device_addr = device.local_addr().unwrap();

    let mut stream = PoseStream::with_target(device_addr, false).unwrap();

    // Read the initial subscribe to learn the client address.
    let mut buf = [0u8; PACKET_SIZE];
    let (_, client) = device.recv_from(&mut buf).unwrap();

    // Send a pose from another thread after a short delay, so the blocking
    // receive actually has to wait for it.
    let poses = vec![sample_pose(11)];
    let data = encode_pose_data(0xABCD, &poses);
    let sender = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        device.send_to(&data, client).unwrap();
    });

    // Blocks until the delayed pose arrives, then returns it.
    let received = stream.receive_pose_updates(true).unwrap();
    assert_eq!(received, poses);
    sender.join().unwrap();
}

#[test]
fn block_times_out_when_no_pose_arrives() {
    let device = fake_device();
    let device_addr = device.local_addr().unwrap();

    let mut stream = PoseStream::with_target(device_addr, false).unwrap();

    // Consume the initial subscribe but never send a pose, so the blocking
    // receive has to hit its timeout.
    let mut buf = [0u8; PACKET_SIZE];
    let (_, _client) = device.recv_from(&mut buf).unwrap();

    let start = std::time::Instant::now();
    let received = stream.receive_pose_updates(true).unwrap();
    let elapsed = start.elapsed();

    // Returns empty after roughly the 3-second timeout (with slack for
    // scheduling).
    assert!(received.is_empty());
    assert!(
        elapsed >= Duration::from_secs(3) && elapsed < Duration::from_secs(5),
        "blocking receive took {elapsed:?}, expected ~3s"
    );
}

#[test]
fn threaded_block_waits_for_pose() {
    let device = fake_device();
    let device_addr = device.local_addr().unwrap();

    let mut stream = PoseStream::with_target(device_addr, true).unwrap();

    let mut buf = [0u8; PACKET_SIZE];
    let (_, client) = device.recv_from(&mut buf).unwrap();

    // Delayed send so the condvar-based blocking receive has to wait.
    let poses = vec![sample_pose(99)];
    let data = encode_pose_data(3, &poses);
    let sender = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        device.send_to(&data, client).unwrap();
    });

    // Blocks until the background thread buffers the pose.
    let received = stream.receive_pose_updates(true).unwrap();
    assert_eq!(received, poses);
    sender.join().unwrap();
}

#[test]
fn from_device_uses_device_ip_and_pose_port() {
    use crate::protocol::POSE_PORT;

    // `from_device` connects to the real POSE_PORT, so the fake device must
    // bind it here. If the port is already taken in the test environment,
    // skip rather than fail spuriously.
    let Ok(device) = UdpSocket::bind((Ipv4Addr::LOCALHOST, POSE_PORT)) else {
        eprintln!("skipping: POSE_PORT {POSE_PORT} unavailable");
        return;
    };
    device.set_read_timeout(Some(Duration::from_secs(2))).unwrap();

    let info = DeviceInfo {
        ip: device.local_addr().unwrap().ip(),
        serial: 0x1234,
        compatible: true,
    };

    let _stream = PoseStream::from_device(&info, false).unwrap();
    let mut buf = [0u8; PACKET_SIZE];
    let (n, _) = device.recv_from(&mut buf).unwrap();
    assert_eq!(
        parse_packet(&buf[..n]).unwrap().pkt_type,
        PKT_TYPE_POSE_SUBSCRIBE
    );
}
