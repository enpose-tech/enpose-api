#!/usr/bin/env python3
"""Example Python client for the Enpose API: discover a device on the local
network, then stream and print live marker poses for a few seconds.

Run it from the bundled example (needs `cffi` and the enpose_api shared
library on the loader path):

    pip install cffi
    LD_LIBRARY_PATH=<sdk>/lib python3 example.py

See ../README.md for details on locating the shared library.
"""

import os
import sys

# Make the sibling `enpose_api` package importable regardless of the working
# directory (it lives one level up, next to this example/ folder).
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import enpose_api

# How many batches of poses to print before exiting.
POSE_BATCHES = 200


def main() -> int:
    try:
        devices = enpose_api.discover()
    except enpose_api.Error as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if not devices:
        print("No Enpose devices found on the local network.")
        return 0

    print(f"Found {len(devices)} device(s):")
    chosen = None
    for device in devices:
        state = "compatible" if device.compatible else "INCOMPATIBLE"
        print(f"  - {device.ip} (serial {device.serial}): {state}")
        if chosen is None and device.compatible:
            chosen = device

    if chosen is None:
        print("No compatible device to stream poses from.")
        return 0

    print(f"\nStreaming poses from {chosen.ip}...")
    try:
        # Threaded mode: a background thread buffers poses between polls.
        with enpose_api.PoseStream(chosen, create_thread=True) as stream:
            for _ in range(POSE_BATCHES):
                # Blocking receive: waits up to 3 s for a pose update, so no
                # manual polling delay is needed.
                for pose in stream.receive(block=True):
                    print(
                        f"  t={pose.timestamp:>10} us  marker {pose.marker_id:>3}: "
                        f"pos=({pose.x:+.4f}, {pose.y:+.4f}, {pose.z:+.4f}) "
                        f"sensors={pose.sensors}"
                    )
    except enpose_api.Error as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
