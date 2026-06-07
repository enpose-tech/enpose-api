# Enpose API — Python interface

Python applications use the Enpose tracking API through a [cffi](https://cffi.readthedocs.io)
binding that loads the prebuilt `enpose_api` shared library at run time (cffi's
ABI / `dlopen` mode). There is no build step and no C compiler involved — only
the shared library and the `cffi` package.

## Requirements

- Python 3.8+
- `cffi` — `pip install cffi`
- The `enpose_api` shared library (shipped in the SDK's `lib/` directory).

## Locating the shared library

The binding searches, in order:

1. `ENPOSE_API_LIB`, if set, as an explicit path to the library.
2. a `lib/` directory in any ancestor of the package (the SDK layout).
3. the bare library name, resolved via the system loader path
   (`LD_LIBRARY_PATH` on Linux, `DYLD_LIBRARY_PATH` on macOS, `PATH` on Windows).

So either set `ENPOSE_API_LIB=<sdk>/lib/libenpose_api.so`, or add `<sdk>/lib`
to your loader path.

## Usage

```python
import enpose_api

for device in enpose_api.discover():
    if not device.compatible:
        continue
    with enpose_api.PoseStream(device) as stream:   # threaded by default
        # block=True waits for a pose update (up to a 3-second timeout);
        # the default (False) returns immediately with whatever has arrived.
        for pose in stream.receive(block=True):
            print(pose.marker_id, pose.x, pose.y, pose.z)
```

`discover()` returns a list of `DeviceInfo`. `PoseStream.receive(block=False)`
returns a list of `MarkerPose`; with `block=True` it waits for at least one
update, up to a 3-second timeout, otherwise it returns immediately (poll it
repeatedly). Failures raise `enpose_api.Error`. The stream closes itself on
context-manager exit (or call `stream.close()`).

## Running the bundled example

`example/example.py` is a complete discover-and-stream program. From this
directory (the package's parent), with the library reachable:

```bash
pip install cffi
LD_LIBRARY_PATH=<path-to-sdk>/lib python3 example/example.py
```

The example puts this directory on `sys.path`, so it imports the `enpose_api`
package next to it without installation.

## API reference

The full API reference — every class, function, and field — is published at
<https://enpose.tech/docs/python/>.
