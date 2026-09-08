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
`find_package`; see the [top-level README](https://github.com/enpose-tech/enpose-api/blob/main/README.md).)

## Building the bundled example

`example.c` is a complete discover-and-stream program. From the `c/example`
directory (of the unpacked SDK or of a source checkout — the layout is the
same):

```bash
cmake -S . -B build
cmake --build build
./build/enpose_example
```

The example's `CMakeLists.txt` finds the SDK's CMake config automatically (it
sits two levels up under `lib/cmake/enpose_api`), so no `-DCMAKE_PREFIX_PATH`
is needed. At runtime the program needs to find the shared library — e.g.
`LD_LIBRARY_PATH=<sdk>/lib ./build/enpose_example`.

## API reference

The full API reference — every function, type, and field, generated from
`enpose_api.h` — is published at <https://enpose.tech/docs/c/>.
