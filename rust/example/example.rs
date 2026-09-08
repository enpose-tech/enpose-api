//! Example: discover Enpose tracker devices on the local network, then
//! stream live marker poses from the first one found and print them to the
//! terminal.
//!
//! Run with: `cargo run --example example`
//!
//! The program first broadcasts a discovery request and lists every device
//! that replies. It then opens a [`PoseStream`] to the first compatible
//! device and prints each [`MarkerPose`] it receives for a few seconds
//! before disconnecting (which happens automatically when the stream is
//! dropped).

use std::time::{Duration, Instant};

use enpose_api::{DeviceDiscovery, PoseStream};

/// How long to stream poses before exiting.
const STREAM_DURATION: Duration = Duration::from_secs(10);

fn main() -> std::io::Result<()> {
    let devices = DeviceDiscovery::new().discover()?;

    if devices.is_empty() {
        println!("No Enpose devices found on the local network.");
        return Ok(());
    }

    println!("Found {} device(s):", devices.len());
    for device in &devices {
        let status = if device.compatible {
            "compatible"
        } else {
            "INCOMPATIBLE (protocol version mismatch)"
        };
        println!("  - {} (serial {}): {}", device.ip, device.serial, status);
    }

    // Pick the first compatible device to stream from.
    let Some(device) = devices.iter().find(|d| d.compatible) else {
        println!("No compatible device to stream poses from.");
        return Ok(());
    };

    println!("\nStreaming poses from {} for {:?}...", device.ip, STREAM_DURATION);

    // Threaded mode: a background thread receives and buffers poses, so none
    // are missed regardless of how often we poll below.
    let mut stream = PoseStream::from_device(device, true)?;

    let start = Instant::now();
    while start.elapsed() < STREAM_DURATION {
        // Blocking receive: waits up to 3 s for a pose update, so no manual
        // polling delay is needed.
        for pose in stream.receive_pose_updates(true)? {
            println!(
                "  t={:>10}us marker {:>3}: pos=({:+.4}, {:+.4}, {:+.4}) rmse={:.4} sensors={} emitters={}",
                pose.timestamp,
                pose.marker_id,
                pose.x,
                pose.y,
                pose.z,
                pose.position_rmse,
                pose.sensors,
                pose.observed_emitters,
            );
        }
    }

    // `stream` is dropped here, which disconnects from the device.
    Ok(())
}
