//! Public Rust API for the Enpose 6-DoF tracking system.
//!
//! This API lets external applications discover Enpose tracker devices on
//! the local network and (in future versions) connect to them to receive
//! tracking data. Discovery is built around [`DeviceDiscovery`].
//!
//! # Example
//!
//! ```no_run
//! use enpose_api::DeviceDiscovery;
//!
//! let devices = DeviceDiscovery::new().discover()?;
//! for device in devices {
//!     println!(
//!         "{} (serial {}) compatible={}",
//!         device.ip, device.serial, device.compatible,
//!     );
//! }
//! # Ok::<(), std::io::Error>(())
//! ```

pub mod devicediscovery;
pub mod protocol;

pub use devicediscovery::{DeviceDiscovery, DeviceInfo};
