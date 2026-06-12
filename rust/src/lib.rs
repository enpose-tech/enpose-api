//! Public Rust API for the Enpose 6-DoF tracking system.
//!
//! [Enpose](https://enpose.tech) builds optical tracking systems that
//! deliver precise 6-DoF poses of active markers in real time.
//!
//! This API lets external applications discover Enpose tracker devices on
//! the local network ([`DeviceDiscovery`]) and connect to one to receive a
//! live stream of marker poses ([`PoseStream`]).
//!
//! The networking parts are behind the default `net` feature. Building with
//! `default-features = false` keeps only the wire types ([`MarkerPose`]) and
//! the pure [`protocol`] constants, which is what targets without a sockets
//! stack (e.g. `wasm32`) should depend on.
//!
//! # Example
//!
//! ```no_run
//! use enpose_api::{DeviceDiscovery, PoseStream};
//!
//! // Find a device, then stream poses from it.
//! if let Some(device) = DeviceDiscovery::new().discover()?.into_iter().next() {
//!     let mut stream = PoseStream::from_device(&device, false)?;
//!     // `true` waits for a pose update (up to a 3-second timeout).
//!     for pose in stream.receive_pose_updates(true)? {
//!         println!("marker {} @ ({}, {}, {})", pose.marker_id, pose.x, pose.y, pose.z);
//!     }
//! }
//! # Ok::<(), std::io::Error>(())
//! ```

pub mod marker_pose;
pub mod protocol;

#[cfg(feature = "net")]
pub mod devicediscovery;
#[cfg(feature = "net")]
pub mod ffi;
#[cfg(feature = "net")]
pub mod posestream;

pub use marker_pose::MarkerPose;

#[cfg(feature = "net")]
pub use devicediscovery::{DeviceDiscovery, DeviceInfo};
#[cfg(feature = "net")]
pub use posestream::PoseStream;
