"""Python binding for the Enpose 6-DoF tracking API.

A thin, Pythonic wrapper over the ``enpose_api`` C ABI using `cffi
<https://cffi.readthedocs.io>`_ in ABI (``dlopen``) mode: it loads the prebuilt
shared library at run time, so no C compiler or build step is needed — only the
library and ``cffi`` (``pip install cffi``).

It mirrors the other bindings and covers the full client workflow:

* :func:`discover` finds Enpose devices on the local network.
* :class:`PoseStream` connects to a device and yields live :class:`MarkerPose`
  updates.

Locating the shared library: by default the loader searches a ``lib/`` directory
alongside the unpacked SDK and then the system loader path (``LD_LIBRARY_PATH``
etc.). Set ``ENPOSE_API_LIB`` to an explicit path to override.
"""

from __future__ import annotations

import os
import pathlib
import sys
from dataclasses import dataclass

from cffi import FFI

__all__ = ["Error", "DeviceInfo", "MarkerPose", "PoseStream", "discover"]

# C ABI declarations — kept in sync with c/include/enpose_api.h.
_CDEF = """
typedef enum {
    ENPOSE_OK = 0,
    ENPOSE_ERR_INVALID_ARG = -1,
    ENPOSE_ERR_IO = -2,
    ENPOSE_ERR_PANIC = -3
} EnposeStatus;

typedef struct EnposeDeviceInfo {
    char ip[46];
    uint32_t serial;
    bool compatible;
} EnposeDeviceInfo;

typedef struct EnposeMarkerPose {
    uint64_t timestamp;
    uint16_t marker_id;
    double x;
    double y;
    double z;
    double rotation[9];
    double position_rmse;
    double rotation_rmse;
    uint8_t sensors;
    uint8_t observed_emitters;
} EnposeMarkerPose;

typedef struct EnposePoseStream EnposePoseStream;

EnposeStatus enpose_discover(EnposeDeviceInfo **out_devices, size_t *out_count);
void enpose_device_info_array_free(EnposeDeviceInfo *devices, size_t count);
EnposePoseStream *enpose_pose_stream_connect(const char *ip, bool create_thread);
EnposeStatus enpose_pose_stream_receive(EnposePoseStream *stream,
                                        bool block,
                                        EnposeMarkerPose **out_poses,
                                        size_t *out_count);
void enpose_marker_pose_array_free(EnposeMarkerPose *poses, size_t count);
void enpose_pose_stream_free(EnposePoseStream *stream);
"""

# Human-readable text for the non-OK status codes.
_STATUS_TEXT = {
    -1: "invalid argument",
    -2: "I/O error",
    -3: "internal error",
}


class Error(RuntimeError):
    """Raised when a call into the Enpose library fails."""


def _library_filename() -> str:
    if sys.platform == "darwin":
        return "libenpose_api.dylib"
    if os.name == "nt":
        return "enpose_api.dll"
    return "libenpose_api.so"


def _candidate_paths(filename: str):
    """Yield library locations to try, most specific first."""
    override = os.environ.get("ENPOSE_API_LIB")
    if override:
        yield override
    # In the SDK layout the library sits in a `lib/` directory above this
    # package; check every ancestor for one.
    here = pathlib.Path(__file__).resolve()
    for parent in here.parents:
        yield str(parent / "lib" / filename)
    # Fall back to the bare name, resolved via the system loader path.
    yield filename


def _load_library():
    ffi = FFI()
    ffi.cdef(_CDEF)
    filename = _library_filename()
    attempts = []
    for path in _candidate_paths(filename):
        try:
            return ffi, ffi.dlopen(path)
        except OSError as exc:
            attempts.append(f"{path}: {exc}")
    tried = "\n  ".join(attempts)
    raise Error(
        f"could not load the enpose_api shared library ({filename}); tried:\n  "
        f"{tried}\nSet ENPOSE_API_LIB to its path, or add the SDK's lib/ "
        "directory to your loader path (e.g. LD_LIBRARY_PATH)."
    )


_ffi, _lib = _load_library()


@dataclass(frozen=True)
class DeviceInfo:
    """One device discovered on the local network."""

    ip: str
    """Device IPv4 address."""
    serial: int
    """Factory serial number."""
    compatible: bool
    """True if the device's protocol version matches this binding."""


@dataclass(frozen=True)
class MarkerPose:
    """Pose of one tracked marker in world coordinates.

    Units: ``x``/``y``/``z`` and ``position_rmse`` are in meters;
    ``rotation_rmse`` is in radians.

    The world frame is right-handed, and where its origin and axes lie is
    decided by the device's extrinsics calibration. By default it is the
    reference sensor's optical frame -- origin at that sensor, ``+X`` to its
    right, ``+Y`` down and ``+Z`` into the scene, so a marker in front of the
    device has a positive ``z``. Calibrating against a measured origin puts
    the frame there instead, and measuring a ground plane makes that plane
    ``z = 0`` with ``+Z`` pointing up out of it. Treat the axes as a property
    of the calibration rather than a fixed convention: recalibrating a device
    can move the frame.
    """

    timestamp: int
    """Microseconds since the device started."""
    marker_id: int
    """Marker identifier."""
    x: float
    """World-space X position in meters (of the first model emitter)."""
    y: float
    """World-space Y position in meters."""
    z: float
    """World-space Z position in meters."""
    rotation: tuple  # 9 floats, row-major 3x3, model frame -> world
    """Row-major 3x3 rotation (9 values), model frame to world."""
    position_rmse: float
    """RMS position error in meters (the same units as x/y/z)."""
    rotation_rmse: float
    """RMS rotation error (radians, axis-angle magnitude)."""
    sensors: int
    """Number of sensors that contributed to this pose."""
    observed_emitters: int
    """Number of marker emitters (LEDs) that contributed to this pose.

    The standard marker carries four. A lower count means some were occluded
    or could not be decoded, so the pose was fitted from a reduced set of
    points and is correspondingly biased; ``0`` means the pose was carried
    purely by prediction. Filter on this value to reject poses computed from
    a partial view.
    """


def _raise_status(func: str, status: int) -> None:
    if status != 0:
        text = _STATUS_TEXT.get(int(status), f"status {int(status)}")
        raise Error(f"{func} failed: {text}")


def discover() -> list:
    """Discover Enpose devices on the local network.

    Returns a list of :class:`DeviceInfo` (empty if none are found). Raises
    :class:`Error` on an I/O failure.
    """
    out_devices = _ffi.new("EnposeDeviceInfo **")
    out_count = _ffi.new("size_t *")
    _raise_status("enpose_discover", _lib.enpose_discover(out_devices, out_count))
    devices = out_devices[0]
    count = out_count[0]
    try:
        return [
            DeviceInfo(
                ip=_ffi.string(devices[i].ip).decode("utf-8"),
                serial=int(devices[i].serial),
                compatible=bool(devices[i].compatible),
            )
            for i in range(count)
        ]
    finally:
        _lib.enpose_device_info_array_free(devices, count)


def _marker_pose(c) -> MarkerPose:
    return MarkerPose(
        timestamp=int(c.timestamp),
        marker_id=int(c.marker_id),
        x=float(c.x),
        y=float(c.y),
        z=float(c.z),
        rotation=tuple(float(c.rotation[i]) for i in range(9)),
        position_rmse=float(c.position_rmse),
        rotation_rmse=float(c.rotation_rmse),
        sensors=int(c.sensors),
        observed_emitters=int(c.observed_emitters),
    )


class PoseStream:
    """A live pose stream from one device.

    Closes itself when used as a context manager or garbage-collected; call
    :meth:`close` to release it eagerly.
    """

    def __init__(self, target, create_thread: bool = True):
        """Connect to ``target`` — a :class:`DeviceInfo` or an IPv4 string.

        The Enpose API is IPv4-only. When ``create_thread`` is true (the
        default and preferred mode), a background thread receives and buffers
        poses. Raises :class:`Error` on failure.
        """
        ip = target.ip if isinstance(target, DeviceInfo) else str(target)
        handle = _lib.enpose_pose_stream_connect(ip.encode("utf-8"), create_thread)
        if handle == _ffi.NULL:
            raise Error(f"failed to connect pose stream to {ip}")
        self._handle = handle

    def receive(self, block: bool = False) -> list:
        """Return poses received from the stream.

        When ``block`` is false (the default), returns the poses that have
        arrived since the previous call (empty if none) without waiting. When
        ``block`` is true, waits for at least one pose update, up to a 3-second
        timeout — so the result is still empty if none arrives in that window.
        Raises :class:`Error` on an unrecoverable failure.
        """
        if self._handle is None:
            raise Error("pose stream is closed")
        out_poses = _ffi.new("EnposeMarkerPose **")
        out_count = _ffi.new("size_t *")
        _raise_status(
            "enpose_pose_stream_receive",
            _lib.enpose_pose_stream_receive(self._handle, block, out_poses, out_count),
        )
        poses = out_poses[0]
        count = out_count[0]
        try:
            return [_marker_pose(poses[i]) for i in range(count)]
        finally:
            _lib.enpose_marker_pose_array_free(poses, count)

    def close(self) -> None:
        """Disconnect and release the stream. Idempotent."""
        handle, self._handle = getattr(self, "_handle", None), None
        if handle is not None:
            _lib.enpose_pose_stream_free(handle)

    def __enter__(self) -> "PoseStream":
        return self

    def __exit__(self, *_exc) -> None:
        self.close()

    def __del__(self) -> None:
        self.close()
