//! C ABI for the Enpose API.
//!
//! This is a thin wrapper over the idiomatic Rust API ([`DeviceDiscovery`],
//! [`PoseStream`], [`MarkerPose`]); Rust applications should use those types
//! directly. The functions here exist so C (and other languages with a C
//! FFI) can drive the same discover-then-stream workflow.
//!
//! # Mirrors of this ABI
//!
//! This module defines the ABI; every other binding restates it by hand and
//! must be updated in the same change:
//!
//! * `c/include/enpose_api.h` — the C declarations. The C++ wrapper
//!   (`cpp/include/enpose_api.hpp`) includes that header and aliases its
//!   types, so it follows automatically.
//! * `python/enpose_api/__init__.py` — the cffi `_CDEF` block.
//! * `dotnet/Enpose.Api/NativeMethods.cs` — the P/Invoke signatures and the
//!   `[StructLayout]` structs.
//!
//! Nothing enforces that mechanically across languages, so each mirror
//! asserts the layout below instead: a reordered or retyped field fails
//! `cargo test` here, fails to compile in C and C++, and raises on load in
//! Python and .NET, rather than silently misreading every pose.
//!
//! On the 64-bit targets the SDK ships, that layout is:
//!
//! | Type | Size | Align | Field offsets |
//! |------|------|-------|---------------|
//! | [`EnposeDeviceInfo`] | 56 | 4 | `ip` 0 (46 bytes), `serial` 48, `compatible` 52 |
//! | [`MarkerPose`] | 136 | 8 | `timestamp` 0, `marker_id` 8, `x` 16, `y` 24, `z` 32, `rotation` 40 (72 bytes), `position_rmse` 112, `rotation_rmse` 120, `sensors` 128, `observed_emitters` 129 |
//!
//! [`EnposeStatus`] is a C `int`; a 32-bit target aligns `f64` to 4 and gives
//! [`MarkerPose`] a different, equally valid layout, so the checks are
//! 64-bit-only.
//!
//! Conventions:
//!
//! * Stateful objects are exposed as opaque pointers created by a `connect`
//!   function and released by a `free` function.
//! * Variable-length results (the device list, a pose batch) are returned as
//!   library-allocated arrays; the caller must release each with the matching
//!   `*_array_free` function and must not free the memory itself.
//! * Functions returning [`EnposeStatus`] report success as
//!   [`EnposeStatus::Ok`]; functions returning a pointer report failure as
//!   `NULL`.
//! * Every entry point is panic-safe: a panic is caught at the boundary and
//!   turned into an error result rather than unwinding into C.

use std::ffi::{CStr, c_char};
use std::net::IpAddr;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::str::FromStr;

use crate::devicediscovery::{DeviceDiscovery, DeviceInfo};
use crate::marker_pose::MarkerPose;
use crate::posestream::PoseStream;

/// Result code returned by the fallible C API functions.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnposeStatus {
    /// The call succeeded.
    Ok = 0,
    /// A required argument was null or otherwise invalid.
    InvalidArg = -1,
    /// An I/O error occurred (e.g. the network operation failed).
    Io = -2,
    /// The Rust side panicked; the call was aborted cleanly.
    Panic = -3,
}

/// Size of the IP string buffer in [`EnposeDeviceInfo`], including the
/// terminating null. The API is IPv4-only; the buffer keeps the 46-byte
/// `INET6_ADDRSTRLEN` size for ABI headroom.
const IP_BUF_LEN: usize = 46;

/// C-compatible view of one discovered device.
///
/// Mirrors [`DeviceInfo`], but the address is rendered into a fixed,
/// null-terminated string buffer instead of a Rust `IpAddr`.
#[repr(C)]
pub struct EnposeDeviceInfo {
    /// Null-terminated IPv4 address string.
    pub ip: [c_char; IP_BUF_LEN],
    /// Factory serial number of the device.
    pub serial: u32,
    /// Non-zero when the device's protocol version matches this library.
    pub compatible: bool,
}

/// Discover Enpose devices on the local network.
///
/// On [`EnposeStatus::Ok`], `*out_devices` points to a library-allocated
/// array of `*out_count` [`EnposeDeviceInfo`] entries (or `NULL` with count
/// `0` when none were found). Release it with
/// [`enpose_device_info_array_free`].
///
/// # Safety
///
/// `out_devices` and `out_count` must be valid, writable pointers.
#[no_mangle]
pub unsafe extern "C" fn enpose_discover(
    out_devices: *mut *mut EnposeDeviceInfo,
    out_count: *mut usize,
) -> EnposeStatus {
    let result = catch_unwind(|| {
        if out_devices.is_null() || out_count.is_null() {
            return EnposeStatus::InvalidArg;
        }
        let devices = match DeviceDiscovery::new().discover() {
            Ok(d) => d,
            Err(_) => return EnposeStatus::Io,
        };
        let (ptr, count) = leak_array(devices.iter().map(device_to_c).collect());
        unsafe {
            *out_devices = ptr;
            *out_count = count;
        }
        EnposeStatus::Ok
    });
    result.unwrap_or(EnposeStatus::Panic)
}

/// Release an array returned by [`enpose_discover`].
///
/// # Safety
///
/// `devices`/`count` must be a pair returned by [`enpose_discover`] and not
/// already freed. Passing `NULL` is allowed and does nothing.
#[no_mangle]
pub unsafe extern "C" fn enpose_device_info_array_free(
    devices: *mut EnposeDeviceInfo,
    count: usize,
) {
    let _ = catch_unwind(|| unsafe { free_array(devices, count) });
}

/// Connect a pose stream to the device at `ip` (an IPv4 string).
///
/// The Enpose API is IPv4-only; a non-IPv4 address fails. When `create_thread`
/// is non-zero, a background thread receives and buffers poses (the preferred
/// mode); otherwise poses are collected when [`enpose_pose_stream_receive`] is
/// called. Returns an opaque handle, or `NULL` on failure (a non-IPv4 or
/// invalid address, or a connection error). Release the handle with
/// [`enpose_pose_stream_free`].
///
/// # Safety
///
/// `ip` must be a valid, null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn enpose_pose_stream_connect(
    ip: *const c_char,
    create_thread: bool,
) -> *mut PoseStream {
    let result = catch_unwind(|| {
        if ip.is_null() {
            return ptr::null_mut();
        }
        let Ok(text) = (unsafe { CStr::from_ptr(ip) }).to_str() else {
            return ptr::null_mut();
        };
        let Ok(addr) = IpAddr::from_str(text) else {
            return ptr::null_mut();
        };
        match PoseStream::from_ip(addr, create_thread) {
            Ok(stream) => Box::into_raw(Box::new(stream)),
            Err(_) => ptr::null_mut(),
        }
    });
    result.unwrap_or(ptr::null_mut())
}

/// Return the marker poses received from the stream.
///
/// When `block` is non-zero, waits for at least one pose update before
/// returning, up to a 3-second timeout (after which it returns empty);
/// otherwise returns immediately with whatever has arrived since the previous
/// call (possibly none). On [`EnposeStatus::Ok`], `*out_poses` points to a
/// library-allocated array of `*out_count` [`MarkerPose`] entries (or `NULL`
/// with count `0` when none have arrived). Release it with
/// [`enpose_marker_pose_array_free`].
///
/// # Safety
///
/// `stream` must be a handle from [`enpose_pose_stream_connect`] that has
/// not been freed; `out_poses` and `out_count` must be valid, writable
/// pointers.
#[no_mangle]
pub unsafe extern "C" fn enpose_pose_stream_receive(
    stream: *mut PoseStream,
    block: bool,
    out_poses: *mut *mut MarkerPose,
    out_count: *mut usize,
) -> EnposeStatus {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if stream.is_null() || out_poses.is_null() || out_count.is_null() {
            return EnposeStatus::InvalidArg;
        }
        let stream = unsafe { &mut *stream };
        match stream.receive_pose_updates(block) {
            Ok(poses) => {
                let (ptr, count) = leak_array(poses);
                unsafe {
                    *out_poses = ptr;
                    *out_count = count;
                }
                EnposeStatus::Ok
            }
            Err(_) => EnposeStatus::Io,
        }
    }));
    result.unwrap_or(EnposeStatus::Panic)
}

/// Release an array returned by [`enpose_pose_stream_receive`].
///
/// # Safety
///
/// `poses`/`count` must be a pair returned by [`enpose_pose_stream_receive`]
/// and not already freed. Passing `NULL` is allowed and does nothing.
#[no_mangle]
pub unsafe extern "C" fn enpose_marker_pose_array_free(poses: *mut MarkerPose, count: usize) {
    let _ = catch_unwind(|| unsafe { free_array(poses, count) });
}

/// Disconnect and free a pose stream handle.
///
/// Disconnects from the device and releases all resources. Passing `NULL` is
/// allowed and does nothing.
///
/// # Safety
///
/// `stream` must be a handle from [`enpose_pose_stream_connect`] that has
/// not already been freed.
#[no_mangle]
pub unsafe extern "C" fn enpose_pose_stream_free(stream: *mut PoseStream) {
    if stream.is_null() {
        return;
    }
    // Dropping the box runs PoseStream's destructor, which disconnects.
    let _ = catch_unwind(AssertUnwindSafe(|| drop(unsafe { Box::from_raw(stream) })));
}

/// Convert a [`DeviceInfo`] into its C representation.
fn device_to_c(info: &DeviceInfo) -> EnposeDeviceInfo {
    let mut ip = [0 as c_char; IP_BUF_LEN];
    let text = info.ip.to_string();
    let bytes = text.as_bytes();
    // Leave at least one byte for the null terminator (already zero).
    let n = bytes.len().min(IP_BUF_LEN - 1);
    for (slot, &byte) in ip.iter_mut().zip(&bytes[..n]) {
        *slot = byte as c_char;
    }
    EnposeDeviceInfo {
        ip,
        serial: info.serial,
        compatible: info.compatible,
    }
}

/// Leak a `Vec<T>` into a `(ptr, len)` pair the caller owns. An empty vector
/// yields `(NULL, 0)` so the C side never sees a dangling pointer.
fn leak_array<T>(items: Vec<T>) -> (*mut T, usize) {
    if items.is_empty() {
        return (ptr::null_mut(), 0);
    }
    let boxed = items.into_boxed_slice();
    let count = boxed.len();
    let ptr = boxed.as_ptr() as *mut T;
    std::mem::forget(boxed);
    (ptr, count)
}

/// Reconstruct and drop an array previously produced by [`leak_array`].
///
/// # Safety
///
/// `ptr`/`count` must be a pair from [`leak_array`] that has not been freed,
/// or `(NULL, 0)`.
unsafe fn free_array<T>(ptr: *mut T, count: usize) {
    if ptr.is_null() || count == 0 {
        return;
    }
    unsafe {
        drop(Box::from_raw(std::slice::from_raw_parts_mut(ptr, count)));
    }
}

#[cfg(test)]
#[path = "ffi_tests.rs"]
mod tests;
