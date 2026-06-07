//! The [`MarkerPose`] wire type, shared between the Enpose API and the
//! on-device daemon.
//!
//! [`MarkerPose`] is serialized with MessagePack on the wire (see
//! [`crate::posestream`]). It carries no networking logic itself, so this
//! module compiles on any target — including ones without a sockets stack —
//! and is available regardless of the `net` feature.

use serde::{Deserialize, Serialize};

/// Position and orientation of one tracked marker in world coordinates.
///
/// One [`MarkerPose`] is produced per localized marker per tracking
/// update. A [`crate::PoseStream`] delivers these as they arrive from the
/// device.
///
/// # Units
///
/// All positions ([`x`](Self::x), [`y`](Self::y), [`z`](Self::z) and
/// [`position_rmse`](Self::position_rmse)) are in **meters**; rotation error
/// ([`rotation_rmse`](Self::rotation_rmse)) is in **radians**.
///
/// The layout is `#[repr(C)]` so the C API can use a struct with the same
/// fields directly. This does not affect the MessagePack wire format, which
/// is derived from the field definitions, not the memory layout.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[repr(C)]
pub struct MarkerPose {
    /// Microseconds since the tracking device started. All markers from the
    /// same tracking update share one timestamp. Subtract two poses'
    /// timestamps to get the elapsed time between them; the absolute value
    /// is not a wall-clock time and is only meaningful relative to other
    /// poses from the same device session.
    pub timestamp: u64,
    /// Identifier of the marker this pose belongs to.
    pub marker_id: u16,
    /// World-space position in meters (of the first model emitter).
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// Rotation matrix (row-major 3x3), model frame to world frame.
    pub rotation: [f64; 9],
    /// Estimated RMS error of the position, in meters (the same units as
    /// x/y/z).
    pub position_rmse: f64,
    /// Estimated RMS error of the rotation, expressed as the magnitude of an
    /// axis-angle perturbation 3-vector (radians).
    pub rotation_rmse: f64,
    /// Number of sensors that contributed to this measurement.
    pub sensors: u8,
}
