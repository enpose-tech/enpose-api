# Enpose API — C++ interface

A header-only C++ wrapper around the Enpose [C API](../c). It adds RAII,
`std::vector`/`std::string` results, and exceptions, while reusing the C ABI
underneath. The wrapper is declared in the SDK header `enpose_api.hpp` and
links the same `enpose_api` shared library as the C interface.

## Usage

```cpp
#include "enpose_api.hpp"

for (const enpose::DeviceInfo& device : enpose::discover()) {
    if (!device.compatible) continue;
    enpose::PoseStream stream(device);          // threaded by default; RAII
    // receive(true) waits for a pose update (up to a 3-second timeout);
    // receive() (the default) returns immediately with whatever has arrived.
    for (const enpose::MarkerPose& pose : stream.receive(/*block=*/true)) {
        // ... use pose.timestamp, pose.x, pose.rotation, ...
    }
}
```

Failures throw `enpose::Error`. The `PoseStream` disconnects automatically when
it goes out of scope.

## Consuming the API

In your own CMake project, link the C++ target — it pulls in the headers and the
shared library transitively:

```cmake
find_package(enpose_api CONFIG REQUIRED)   # point CMAKE_PREFIX_PATH at the SDK
target_link_libraries(my_app PRIVATE enpose_api::enpose_api_cpp)
```

(In an in-tree / `FetchContent` build the same target is provided without
`find_package`; see the [top-level README](../README.md).)

## Building the bundled example

`example.cpp` is a complete discover-and-stream program. From the SDK's
`examples/cpp` directory:

```bash
cmake -S . -B build
cmake --build build
./build/enpose_example_cpp
```

The example's `CMakeLists.txt` finds the SDK's CMake config automatically (it
sits two levels up under `lib/cmake/enpose_api`), so no `-DCMAKE_PREFIX_PATH`
is needed. At runtime the program needs to find the shared library — e.g.
`LD_LIBRARY_PATH=<sdk>/lib ./build/enpose_example_cpp`.

## API reference

The full API reference — every class, method, and field, generated from
`enpose_api.hpp` — is published at <https://enpose.tech/docs/cpp/>.
