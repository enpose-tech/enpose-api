/**
 * @file enpose_api.h
 * @brief Enpose API — C interface for the 6-DoF tracking system.
 *
 * This header is hand-maintained and must be kept in sync with the Rust FFI
 * layer in `rust/src/ffi.rs`, which defines the ABI and lists every binding
 * that mirrors it. The struct layouts are asserted at the bottom of this file,
 * so a mismatch is a compile error rather than a run-time misread. Link
 * against the `enpose_api` shared library (built with `cargo build --release`).
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
 *
 * Coordinate system: the world frame is right-handed, and where its origin and
 * axes lie is decided by the device's extrinsics calibration. By default it is
 * the reference sensor's optical frame — origin at that sensor, +X to its
 * right, +Y down and +Z into the scene, so a marker in front of the device has
 * a positive z. Calibrating against a measured origin puts the frame there
 * instead, and measuring a ground plane makes that plane z = 0 with +Z pointing
 * up out of it. Treat the axes as a property of the calibration rather than a
 * fixed convention: recalibrating a device can move the frame.
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
    /**
     * Number of marker emitters (LEDs) whose measurements contributed to this
     * pose. The standard marker carries four; a lower count means some were
     * occluded or could not be decoded, so the pose was fitted from a reduced
     * set of points and might be correspondingly biased. 0 on a pose carried purely
     * by prediction. Filter on this to reject poses from a partial view.
     */
    uint8_t observed_emitters;
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

/** @cond */
/*
 * Compile-time check that these declarations still describe the same memory
 * layout as the Rust definitions they mirror (rust/src/ffi.rs carries the
 * layout table and the list of mirrors). A field reordered or retyped on
 * either side breaks the build here — in every C and C++ translation unit that
 * includes this header, the C++ wrapper included — instead of silently
 * misreading every device and pose at run time.
 *
 * Field widths are checked alongside the offsets: widening a field can leave
 * every offset and the total size untouched by consuming the padding next to
 * it (uint16_t -> uint32_t for marker_id, say), while the two sides then
 * disagree about how many of those bytes carry the value.
 *
 * 64-bit targets only: a 32-bit x86 ABI aligns double to 4 and produces a
 * different, equally valid layout. Skipped as well on pre-C11 compilers, which
 * have no static assertion to use.
 */
#if defined(UINTPTR_MAX) && UINTPTR_MAX == 0xFFFFFFFFFFFFFFFFULL

#if defined(__cplusplus)
#define ENPOSE_STATIC_ASSERT(cond, msg) static_assert(cond, msg)
#elif defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
#define ENPOSE_STATIC_ASSERT(cond, msg) _Static_assert(cond, msg)
#else
/* Pre-C11: no static assertion to use. Expand to a harmless repeated
 * declaration rather than to nothing, which would leave a stray semicolon at
 * file scope. */
#define ENPOSE_STATIC_ASSERT(cond, msg) extern int enpose_static_assert_unavailable
#endif

ENPOSE_STATIC_ASSERT(sizeof(EnposeStatus) == sizeof(int), "EnposeStatus is a C int");
ENPOSE_STATIC_ASSERT(sizeof(bool) == 1, "bool crosses the ABI as one byte");

/* Offset and width of one field, in one line. */
#define ENPOSE_ASSERT_FIELD(type, field, off, width)                       \
    ENPOSE_STATIC_ASSERT(offsetof(type, field) == (off), #type "." #field " offset"); \
    ENPOSE_STATIC_ASSERT(sizeof(((type *)0)->field) == (width), #type "." #field " width")

ENPOSE_STATIC_ASSERT(sizeof(EnposeDeviceInfo) == 56, "EnposeDeviceInfo size");
ENPOSE_ASSERT_FIELD(EnposeDeviceInfo, ip, 0, 46);
ENPOSE_ASSERT_FIELD(EnposeDeviceInfo, serial, 48, 4);
ENPOSE_ASSERT_FIELD(EnposeDeviceInfo, compatible, 52, 1);

ENPOSE_STATIC_ASSERT(sizeof(EnposeMarkerPose) == 136, "EnposeMarkerPose size");
ENPOSE_ASSERT_FIELD(EnposeMarkerPose, timestamp, 0, 8);
ENPOSE_ASSERT_FIELD(EnposeMarkerPose, marker_id, 8, 2);
ENPOSE_ASSERT_FIELD(EnposeMarkerPose, x, 16, 8);
ENPOSE_ASSERT_FIELD(EnposeMarkerPose, y, 24, 8);
ENPOSE_ASSERT_FIELD(EnposeMarkerPose, z, 32, 8);
ENPOSE_ASSERT_FIELD(EnposeMarkerPose, rotation, 40, 72);
ENPOSE_ASSERT_FIELD(EnposeMarkerPose, position_rmse, 112, 8);
ENPOSE_ASSERT_FIELD(EnposeMarkerPose, rotation_rmse, 120, 8);
ENPOSE_ASSERT_FIELD(EnposeMarkerPose, sensors, 128, 1);
ENPOSE_ASSERT_FIELD(EnposeMarkerPose, observed_emitters, 129, 1);

#undef ENPOSE_ASSERT_FIELD

#undef ENPOSE_STATIC_ASSERT

#endif /* 64-bit target */
/** @endcond */
