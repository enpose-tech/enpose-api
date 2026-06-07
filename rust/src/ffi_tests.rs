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
    }
}

fn encode_pose_data(serial: u32, poses: &[MarkerPose]) -> Vec<u8> {
    let mut packet = encode_pose_data_header(serial).to_vec();
    packet.extend(rmp_serde::to_vec(poses).unwrap());
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
