use std::ffi::CString;
use std::net::{Ipv4Addr, UdpSocket};
use std::ptr;
use std::time::Duration;

use super::*;
use crate::protocol::{POSE_PORT, encode_pose_data_header};

fn sample_pose(marker_id: u16) -> MarkerPose {
    MarkerPose {
        timestamp: 100,
        marker_id,
        x: 1.0,
        y: 2.0,
        z: 3.0,
        rotation: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        position_rmse: 0.25,
        rotation_rmse: 0.025,
        sensors: 2,
        observed_emitters: 4,
    }
}

fn encode_pose_data(serial: u32, poses: &[MarkerPose]) -> Vec<u8> {
    let mut packet = encode_pose_data_header(serial).to_vec();
    packet.extend(MarkerPose::encode_batch(poses));
    packet
}

#[test]
fn connect_null_ip_returns_null() {
    let handle = unsafe { enpose_pose_stream_connect(ptr::null(), false) };
    assert!(handle.is_null());
}

#[test]
fn connect_invalid_ip_returns_null() {
    let ip = CString::new("not-an-ip").unwrap();
    let handle = unsafe { enpose_pose_stream_connect(ip.as_ptr(), false) };
    assert!(handle.is_null());
}

#[test]
fn receive_null_args_is_invalid() {
    let status =
        unsafe { enpose_pose_stream_receive(ptr::null_mut(), false, ptr::null_mut(), ptr::null_mut()) };
    assert_eq!(status, EnposeStatus::InvalidArg);
}

#[test]
fn discover_null_args_is_invalid() {
    let status = unsafe { enpose_discover(ptr::null_mut(), ptr::null_mut()) };
    assert_eq!(status, EnposeStatus::InvalidArg);
}

#[test]
fn free_functions_accept_null() {
    // None of these should crash.
    unsafe {
        enpose_pose_stream_free(ptr::null_mut());
        enpose_marker_pose_array_free(ptr::null_mut(), 0);
        enpose_device_info_array_free(ptr::null_mut(), 0);
    }
}

#[test]
fn connect_receive_free_round_trip() {
    // `connect` targets the fixed POSE_PORT, so the fake device must bind it.
    // Skip rather than fail if the port is unavailable in this environment.
    let Ok(device) = UdpSocket::bind((Ipv4Addr::LOCALHOST, POSE_PORT)) else {
        eprintln!("skipping: POSE_PORT {POSE_PORT} unavailable");
        return;
    };
    device.set_read_timeout(Some(Duration::from_secs(2))).unwrap();

    let ip = CString::new("127.0.0.1").unwrap();
    let stream = unsafe { enpose_pose_stream_connect(ip.as_ptr(), false) };
    assert!(!stream.is_null());

    // The connect call sends an initial subscribe; learn the client address.
    let mut buf = [0u8; 64];
    let (_, client) = device.recv_from(&mut buf).unwrap();

    // Device sends a pose batch.
    let poses = vec![sample_pose(1), sample_pose(2)];
    device.send_to(&encode_pose_data(7, &poses), client).unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // Receive through the C API.
    let mut out_poses: *mut MarkerPose = ptr::null_mut();
    let mut out_count: usize = 0;
    let status = unsafe { enpose_pose_stream_receive(stream, false, &mut out_poses, &mut out_count) };
    assert_eq!(status, EnposeStatus::Ok);
    assert_eq!(out_count, 2);
    assert!(!out_poses.is_null());

    let received = unsafe { std::slice::from_raw_parts(out_poses, out_count) };
    assert_eq!(received, poses.as_slice());

    unsafe {
        enpose_marker_pose_array_free(out_poses, out_count);
        enpose_pose_stream_free(stream);
    }
}

// ---------------------------------------------------------------------------
// C ABI layout.
//
// These assertions pin the layout that every non-Rust binding hand-mirrors
// (see the table in this module's documentation): reordering or retyping a
// field here fails the build instead of silently breaking the C, C++, Python
// and .NET bindings at run time. Each of those mirrors asserts the same
// numbers on its own side.
//
// 64-bit targets only — the ones the SDK ships. A 32-bit x86 ABI aligns `f64`
// to 4 and legitimately produces a different, smaller layout.
// ---------------------------------------------------------------------------

/// Byte offset and width of one field, as `(offset, width)`.
///
/// `std::mem::offset_of!` would give the offset directly, but it is newer
/// (Rust 1.77) than this crate's MSRV. The width matters as much as the
/// offset: widening a field can consume the padding beside it and leave every
/// offset and the total size unchanged (`u16` -> `u32` for `marker_id`, say),
/// while the mirrors then disagree about how many of those bytes carry the
/// value.
#[cfg(target_pointer_width = "64")]
fn field_at<T, F>(value: &T, field: &F) -> (usize, usize) {
    let offset = (field as *const F as usize) - (value as *const T as usize);
    (offset, std::mem::size_of_val(field))
}

#[cfg(target_pointer_width = "64")]
#[test]
fn marker_pose_layout_matches_c_abi() {
    use std::mem::{align_of, size_of};

    let p = sample_pose(1);
    assert_eq!(size_of::<MarkerPose>(), 136, "sizeof(EnposeMarkerPose)");
    assert_eq!(align_of::<MarkerPose>(), 8, "alignof(EnposeMarkerPose)");
    assert_eq!(field_at(&p, &p.timestamp), (0, 8), "timestamp");
    assert_eq!(field_at(&p, &p.marker_id), (8, 2), "marker_id");
    assert_eq!(field_at(&p, &p.x), (16, 8), "x");
    assert_eq!(field_at(&p, &p.y), (24, 8), "y");
    assert_eq!(field_at(&p, &p.z), (32, 8), "z");
    assert_eq!(field_at(&p, &p.rotation), (40, 72), "rotation");
    assert_eq!(field_at(&p, &p.position_rmse), (112, 8), "position_rmse");
    assert_eq!(field_at(&p, &p.rotation_rmse), (120, 8), "rotation_rmse");
    assert_eq!(field_at(&p, &p.sensors), (128, 1), "sensors");
    assert_eq!(field_at(&p, &p.observed_emitters), (129, 1), "observed_emitters");
}

#[cfg(target_pointer_width = "64")]
#[test]
fn device_info_layout_matches_c_abi() {
    use std::mem::{align_of, size_of};

    let d = EnposeDeviceInfo { ip: [0; IP_BUF_LEN], serial: 0, compatible: false };
    assert_eq!(size_of::<EnposeDeviceInfo>(), 56, "sizeof(EnposeDeviceInfo)");
    assert_eq!(align_of::<EnposeDeviceInfo>(), 4, "alignof(EnposeDeviceInfo)");
    assert_eq!(field_at(&d, &d.ip), (0, 46), "ip");
    assert_eq!(field_at(&d, &d.serial), (48, 4), "serial");
    assert_eq!(field_at(&d, &d.compatible), (52, 1), "compatible");
}

#[test]
fn status_codes_match_c_abi() {
    // A #[repr(C)] enum is a C `int`, and `bool` crosses the boundary as one
    // byte — both assumed by every mirror of this ABI.
    assert_eq!(std::mem::size_of::<EnposeStatus>(), std::mem::size_of::<std::ffi::c_int>());
    assert_eq!(std::mem::size_of::<bool>(), 1);
    assert_eq!(EnposeStatus::Ok as i32, 0);
    assert_eq!(EnposeStatus::InvalidArg as i32, -1);
    assert_eq!(EnposeStatus::Io as i32, -2);
    assert_eq!(EnposeStatus::Panic as i32, -3);
}

#[test]
fn device_to_c_writes_null_terminated_ip() {
    let info = DeviceInfo {
        ip: std::net::IpAddr::V4(Ipv4Addr::new(192, 168, 10, 10)),
        serial: 0x1234,
        compatible: true,
    };
    let c = device_to_c(&info);
    // Reinterpret the buffer as bytes and read up to the null terminator.
    let bytes: Vec<u8> = c.ip.iter().take_while(|&&b| b != 0).map(|&b| b as u8).collect();
    assert_eq!(String::from_utf8(bytes).unwrap(), "192.168.10.10");
    assert_eq!(c.serial, 0x1234);
    assert!(c.compatible);
}
