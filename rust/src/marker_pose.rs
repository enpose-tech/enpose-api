//! The [`MarkerPose`] wire type, shared between the Enpose API and the
//! on-device daemon.
//!
//! On the wire a batch of poses is encoded with a small, fixed-layout
//! big-endian binary codec ([`MarkerPose::encode_batch`] /
//! [`MarkerPose::decode_batch`]) — no external serialization framework. The
//! type carries no networking logic itself, so this module compiles on any
//! target — including ones without a sockets stack — and is available
//! regardless of the `net` feature.

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
/// # Coordinate system
///
/// The world frame is right-handed, and where its origin and axes lie is
/// decided by the device's extrinsics calibration:
///
/// * By default it is the reference sensor's optical frame — the origin at
///   that sensor, **+X** to its right, **+Y** down and **+Z** pointing into
///   the scene, so a marker in front of the device has a positive `z`.
/// * If the device was calibrated against a measured origin, the frame is the
///   one the operator marked out there instead.
/// * If a ground plane was measured, that plane becomes `z = 0` with **+Z**
///   pointing up out of it.
///
/// A device therefore reports poses in whichever of these frames it was last
/// calibrated for, and recalibrating it can move the frame. Treat the axes as
/// a property of the calibration rather than a fixed convention.
///
/// The layout is `#[repr(C)]` so the C API can use a struct with the same
/// fields directly. This does not affect the MessagePack wire format, which
/// is derived from the field definitions, not the memory layout.
#[derive(Clone, Debug, PartialEq)]
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

impl MarkerPose {
    /// Wire size of one pose record produced by [`Self::encode_batch`]:
    /// `timestamp(8) + marker_id(2) + x/y/z(24) + rotation(72) +
    /// position_rmse(8) + rotation_rmse(8) + sensors(1)`.
    const ENCODED_SIZE: usize = 123;

    /// Encode a batch of poses into the pose-data payload that follows the
    /// fixed packet header on the wire.
    ///
    /// The layout is a 2-byte big-endian count followed by that many
    /// fixed-size, big-endian pose records — matching the byte-order
    /// convention of the rest of the protocol (see [`crate::protocol`]) and
    /// using no external serialization framework. Counterpart to
    /// [`Self::decode_batch`].
    ///
    /// A batch with more than `u16::MAX` poses is truncated to `u16::MAX`;
    /// real tracking frames carry only a handful of markers, so this never
    /// occurs in practice.
    pub fn encode_batch(poses: &[MarkerPose]) -> Vec<u8> {
        let count = poses.len().min(u16::MAX as usize);
        let mut buf = Vec::with_capacity(2 + count * Self::ENCODED_SIZE);
        buf.extend_from_slice(&(count as u16).to_be_bytes());
        for pose in &poses[..count] {
            buf.extend_from_slice(&pose.timestamp.to_be_bytes());
            buf.extend_from_slice(&pose.marker_id.to_be_bytes());
            buf.extend_from_slice(&pose.x.to_be_bytes());
            buf.extend_from_slice(&pose.y.to_be_bytes());
            buf.extend_from_slice(&pose.z.to_be_bytes());
            for value in pose.rotation {
                buf.extend_from_slice(&value.to_be_bytes());
            }
            buf.extend_from_slice(&pose.position_rmse.to_be_bytes());
            buf.extend_from_slice(&pose.rotation_rmse.to_be_bytes());
            buf.push(pose.sensors);
        }
        buf
    }

    /// Decode a pose-data payload produced by [`Self::encode_batch`].
    ///
    /// Returns `None` if the buffer is malformed — shorter than the 2-byte
    /// count, or a length that does not exactly match the declared count.
    pub fn decode_batch(data: &[u8]) -> Option<Vec<MarkerPose>> {
        let count = u16::from_be_bytes(data.get(0..2)?.try_into().ok()?) as usize;
        let body = &data[2..];
        if body.len() != count * Self::ENCODED_SIZE {
            return None;
        }
        Some(body.chunks_exact(Self::ENCODED_SIZE).map(Self::decode_one).collect())
    }

    /// Decode one fixed-size pose record. `b.len()` is exactly
    /// [`Self::ENCODED_SIZE`], guaranteed by [`Self::decode_batch`], so every
    /// slice index below is in bounds.
    fn decode_one(b: &[u8]) -> MarkerPose {
        let f64_at = |off: usize| f64::from_be_bytes(b[off..off + 8].try_into().unwrap());
        let mut rotation = [0.0f64; 9];
        for (i, slot) in rotation.iter_mut().enumerate() {
            *slot = f64_at(34 + i * 8);
        }
        MarkerPose {
            timestamp: u64::from_be_bytes(b[0..8].try_into().unwrap()),
            marker_id: u16::from_be_bytes(b[8..10].try_into().unwrap()),
            x: f64_at(10),
            y: f64_at(18),
            z: f64_at(26),
            rotation,
            position_rmse: f64_at(106),
            rotation_rmse: f64_at(114),
            sensors: b[122],
        }
    }
}

#[cfg(test)]
#[path = "marker_pose_tests.rs"]
mod tests;
