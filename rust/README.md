# Enpose API — Rust

The native Rust implementation of the Enpose tracking API, and the source of
truth for the [C](../c) and [C++](../cpp) bindings (which call into the C ABI
this crate exports).

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

## Features

- `net` *(default)* — enables the networking API (`DeviceDiscovery`,
  `PoseStream`) and the exported C ABI.
- With `default-features = false`, only the wire types (`MarkerPose`) and the
  pure protocol constants are built, for targets without a sockets stack (e.g.
  `wasm32`).

[`DeviceDiscovery`]: src/devicediscovery.rs
[`PoseStream`]: src/posestream.rs
[`MarkerPose`]: src/marker_pose.rs
