/**
 * @file enpose_api.h
 * @brief Enpose API — C interface for the 6-DoF tracking system.
 *
 * This header is hand-maintained and must be kept in sync with the Rust FFI
 * layer in `rust/src/ffi.rs`. Link against the `enpose_api` shared library
 * (built with `cargo build --release`).
 *
 * Workflow: discover devices on the local network with enpose_discover(),
 * open a pose stream to one of them with enpose_pose_stream_connect(), poll it
 * with enpose_pose_stream_receive(), and release it with
 * enpose_pose_stream_free().
 *
 * Ownership: arrays returned by the library (device lists, pose batches) are
 * owned by the library and must be released with the matching *_array_free
 * function. Do not free them with free(). Passing NULL to any free function
 * is allowed and does nothing.
 */
#pragma once

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

/**
 * @defgroup enpose_c_api Enpose C API
 * @brief Functions, types, and status codes of the Enpose C interface.
 * @{
 */
#ifdef __cplusplus
extern "C" {
#endif

/** Result code returned by the fallible functions. */
typedef enum EnposeStatus {
    ENPOSE_OK = 0,           /**< The call succeeded. */
    ENPOSE_ERR_INVALID_ARG = -1, /**< A required argument was null/invalid. */
    ENPOSE_ERR_IO = -2,      /**< An I/O error occurred (e.g. network failure). */
    ENPOSE_ERR_PANIC = -3    /**< Internal error; the call was aborted cleanly. */
} EnposeStatus;

/** C view of one discovered device. */
typedef struct EnposeDeviceInfo {
    char ip[46];        /**< Null-terminated IPv4 address string. */
    uint32_t serial;    /**< Factory serial number. */
    bool compatible;    /**< True if the device's protocol version matches. */
} EnposeDeviceInfo;

/**
 * Pose of one tracked marker in world coordinates. Mirrors the Rust
 * `MarkerPose` type field-for-field (the Rust type is #[repr(C)]).
 *
 * Units: x/y/z and position_rmse are in meters; rotation_rmse is in radians.
 */
typedef struct EnposeMarkerPose {
    uint64_t timestamp;     /**< Microseconds since the device started. */
    uint16_t marker_id;     /**< Marker identifier. */
    double x;               /**< World position in meters (first model emitter). */
    double y;
    double z;
    double rotation[9];     /**< Row-major 3x3 rotation, model frame to world. */
    double position_rmse;   /**< RMS position error, in meters (units of x/y/z). */
    double rotation_rmse;   /**< RMS rotation error (radians, axis-angle magnitude). */
    uint8_t sensors;        /**< Number of sensors that contributed. */
} EnposeMarkerPose;

/** Opaque handle to a live pose stream. */
typedef struct EnposePoseStream EnposePoseStream;

/**
 * Discover Enpose devices on the local network.
 *
 * On ENPOSE_OK, *out_devices points to a library-allocated array of
 * *out_count entries (or NULL with count 0 when none were found). Release it
 * with enpose_device_info_array_free(). out_devices and out_count must be
 * non-null.
 */
EnposeStatus enpose_discover(EnposeDeviceInfo **out_devices, size_t *out_count);

/** Release an array returned by enpose_discover(). */
void enpose_device_info_array_free(EnposeDeviceInfo *devices, size_t count);

/**
 * Connect a pose stream to the device at `ip` (an IPv4 string).
 *
 * The Enpose API is IPv4-only; a non-IPv4 address fails. When create_thread is
 * true, a background thread receives and buffers poses (the preferred mode);
 * otherwise poses are collected when enpose_pose_stream_receive() is called.
 * Returns an opaque handle, or NULL on failure (a non-IPv4 or invalid address,
 * or a connection error). Release the handle with enpose_pose_stream_free().
 */
EnposePoseStream *enpose_pose_stream_connect(const char *ip, bool create_thread);

/**
 * Return the poses received from the stream.
 *
 * When `block` is true, waits for at least one pose update before returning,
 * up to a 3-second timeout (after which it returns with none); otherwise
 * returns immediately with whatever has arrived since the previous call
 * (possibly none). On ENPOSE_OK, *out_poses points to a library-allocated
 * array of *out_count entries (or NULL with count 0 when none have arrived).
 * Release it with enpose_marker_pose_array_free().
 */
EnposeStatus enpose_pose_stream_receive(EnposePoseStream *stream,
                                        bool block,
                                        EnposeMarkerPose **out_poses,
                                        size_t *out_count);

/** Release an array returned by enpose_pose_stream_receive(). */
void enpose_marker_pose_array_free(EnposeMarkerPose *poses, size_t count);

/** Disconnect and free a pose stream handle. */
void enpose_pose_stream_free(EnposePoseStream *stream);

#ifdef __cplusplus
} /* extern "C" */
#endif
/** @} */ /* enpose_c_api */
