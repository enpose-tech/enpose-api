# Enpose API — Rust

[Enpose](https://enpose.tech) builds optical tracking systems that deliver
precise 6-DoF poses of active markers in real time.

The native Rust implementation of the Enpose tracking API.

It covers the full client workflow:

- **Discovery** — [`DeviceDiscovery`] finds Enpose devices on the local network.
- **Pose streaming** — [`PoseStream`] connects to a device and delivers a live
  stream of [`MarkerPose`] updates.

## Usage

```rust
use enpose_api::{DeviceDiscovery, PoseStream};

fn main() -> std::io::Result<()> {
    // Find a device, then stream poses from it.
    if let Some(device) = DeviceDiscovery::new().discover()?.into_iter().next() {
        // Threaded mode (preferred): a background thread buffers poses.
        let mut stream = PoseStream::from_device(&device, true)?;
        loop {
            // `true` waits for a pose update (up to a 3-second timeout); pass
            // `false` to poll without waiting.
            for pose in stream.receive_pose_updates(true)? {
                println!("marker {} @ ({}, {}, {})", pose.marker_id, pose.x, pose.y, pose.z);
            }
        }
    }
    Ok(())
}
```

A `PoseStream` can also be opened directly from an address with
`PoseStream::from_ip(ip, create_thread)`, and it disconnects automatically when
dropped.

## Building and running

```bash
cargo build --release
cargo run --example example   # discover devices and stream poses
cargo test
```

The release build also produces the C-ABI shared library
(`target/release/libenpose_api.so`) used by the C and C++ bindings.

## API reference

The full API reference is published at <https://enpose.tech/docs/rust/> (and is
available offline via `cargo doc --open`).

[`DeviceDiscovery`]: src/devicediscovery.rs
[`PoseStream`]: src/posestream.rs
[`MarkerPose`]: src/marker_pose.rs
