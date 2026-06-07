# Enpose API — C interface

C applications use the Enpose tracking API through the C ABI exported by the
`enpose_api` shared library. The interface is declared in the SDK header
`enpose_api.h`.

## Consuming the API

In your own CMake project, link the C target — it pulls in the header and the
shared library transitively:

```cmake
find_package(enpose_api CONFIG REQUIRED)   # point CMAKE_PREFIX_PATH at the SDK
target_link_libraries(my_app PRIVATE enpose_api::enpose_api)
```

(In an in-tree / `FetchContent` build the same target is provided without
`find_package`; see the [top-level README](../README.md).)

## Building the bundled example

`example/example.c` is a complete discover-and-stream program. From the
`example/` directory:

```bash
cmake -S . -B build -DCMAKE_PREFIX_PATH=<path-to-sdk>
cmake --build build
./build/enpose_example
```

`CMAKE_PREFIX_PATH` points at the unpacked SDK (the directory containing
`lib/cmake/enpose_api`). At runtime the program needs to find the shared
library — e.g. `LD_LIBRARY_PATH=<sdk>/lib ./build/enpose_example`.

## API overview

| Function | Purpose |
|----------|---------|
| `enpose_discover` | Find devices on the local network. |
| `enpose_device_info_array_free` | Release a discovered-device array. |
| `enpose_pose_stream_connect` | Open a pose stream to a device by IP. |
| `enpose_pose_stream_receive` | Get poses received since the last call (optionally blocking up to 3 s for at least one). |
| `enpose_marker_pose_array_free` | Release a pose array. |
| `enpose_pose_stream_free` | Disconnect and free a pose stream. |

Arrays returned by the library are owned by the library; release each with its
matching `*_array_free` function rather than `free()`. See the doc comments in
`enpose_api.h` for details and the full workflow in `example/example.c`.
