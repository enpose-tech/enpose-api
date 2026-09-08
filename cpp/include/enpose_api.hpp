/**
 * @file enpose_api.hpp
 * @brief Header-only C++ wrapper around the Enpose C API.
 *
 * This is a thin, RAII-style convenience layer over the C interface declared
 * in `enpose_api.h` (from the C interface's `include/` directory, which must
 * be on the include path). It adds:
 *   * std::vector results instead of caller-freed C arrays,
 *   * std::string addresses,
 *   * a PoseStream class that frees itself on destruction,
 *   * exceptions (enpose::Error) instead of status codes / NULL.
 *
 * Everything is inline; just include this header and link the enpose_api
 * shared library.
 */

#pragma once

#include "enpose_api.h"

#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace enpose {

/// Pose of one tracked marker in world coordinates.
///
/// Alias of the C POD struct, so all fields are available directly:
/// `timestamp`, `marker_id`, `x`/`y`/`z`, `rotation[9]`, `position_rmse`,
/// `rotation_rmse`, `sensors`, `observed_emitters`. Units: `x`/`y`/`z` and
/// `position_rmse` are in meters; `rotation_rmse` is in radians.
///
/// `observed_emitters` counts the marker LEDs whose measurements contributed
/// to the pose (four on the standard marker); a lower count means some were
/// occluded, leaving the pose fitted from a reduced set of points.
///
/// The world frame is right-handed, and where its origin and axes lie is
/// decided by the device's extrinsics calibration. By default it is the
/// reference sensor's optical frame — origin at that sensor, `+X` to its right,
/// `+Y` down and `+Z` into the scene, so a marker in front of the device has a
/// positive `z`. Calibrating against a measured origin puts the frame there
/// instead, and measuring a ground plane makes that plane `z = 0` with `+Z`
/// pointing up out of it. Treat the axes as a property of the calibration
/// rather than a fixed convention: recalibrating a device can move the frame.
using MarkerPose = ::EnposeMarkerPose;

/// Exception thrown by the wrapper when a C call fails.
class Error : public std::runtime_error {
public:
    explicit Error(const std::string& message) : std::runtime_error(message) {}
};

/// One device discovered on the local network.
struct DeviceInfo {
    std::string ip;        ///< Device IPv4 address.
    std::uint32_t serial;  ///< Factory serial number.
    bool compatible;       ///< True if the device's protocol version matches.
};

/// Discover Enpose devices on the local network.
///
/// Returns one entry per device (empty if none are found). Throws Error on an
/// I/O failure.
inline std::vector<DeviceInfo> discover() {
    EnposeDeviceInfo* devices = nullptr;
    std::size_t count = 0;
    if (enpose_discover(&devices, &count) != ENPOSE_OK) {
        throw Error("enpose_discover failed");
    }
    std::vector<DeviceInfo> result;
    result.reserve(count);
    for (std::size_t i = 0; i < count; ++i) {
        result.push_back(DeviceInfo{std::string(devices[i].ip), devices[i].serial,
                                    devices[i].compatible});
    }
    enpose_device_info_array_free(devices, count);
    return result;
}

/// RAII handle to a live pose stream from one device.
///
/// Move-only; the connection is closed automatically on destruction.
class PoseStream {
public:
    /// Connect to the device at `ip` (an IPv4 string; the Enpose API is
    /// IPv4-only). When `create_thread` is true (the default and preferred
    /// mode), a background thread receives and buffers poses. Throws Error on
    /// failure.
    explicit PoseStream(const std::string& ip, bool create_thread = true)
        : handle_(enpose_pose_stream_connect(ip.c_str(), create_thread)) {
        if (handle_ == nullptr) {
            throw Error("failed to connect pose stream to " + ip);
        }
    }

    /// Connect to a discovered device.
    explicit PoseStream(const DeviceInfo& device, bool create_thread = true)
        : PoseStream(device.ip, create_thread) {}

    ~PoseStream() {
        if (handle_ != nullptr) {
            enpose_pose_stream_free(handle_);
        }
    }

    PoseStream(const PoseStream&) = delete;
    PoseStream& operator=(const PoseStream&) = delete;

    PoseStream(PoseStream&& other) noexcept
        : handle_(std::exchange(other.handle_, nullptr)) {}

    PoseStream& operator=(PoseStream&& other) noexcept {
        if (this != &other) {
            if (handle_ != nullptr) {
                enpose_pose_stream_free(handle_);
            }
            handle_ = std::exchange(other.handle_, nullptr);
        }
        return *this;
    }

    /// Return poses received from the stream.
    ///
    /// When `block` is false (the default), returns the poses that have
    /// arrived since the previous call (empty if none) without waiting. When
    /// `block` is true, waits for at least one pose update, up to a 3-second
    /// timeout — so the result is still empty if none arrives in that window.
    /// Throws Error on an unrecoverable communication failure.
    std::vector<MarkerPose> receive(bool block = false) {
        EnposeMarkerPose* poses = nullptr;
        std::size_t count = 0;
        if (enpose_pose_stream_receive(handle_, block, &poses, &count) != ENPOSE_OK) {
            throw Error("enpose_pose_stream_receive failed");
        }
        std::vector<MarkerPose> result(poses, poses + count);
        enpose_marker_pose_array_free(poses, count);
        return result;
    }

private:
    EnposePoseStream* handle_;
};

}  // namespace enpose
