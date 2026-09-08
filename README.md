# enpose-api

Open source API for the 6-DoF tracking systems by [Enpose](https://enpose.tech).

Enpose builds high-precision optical real-time location systems (RTLS). Active
infrared markers transmit a unique coded-light signal that a network of
high-speed camera sensors detects and decodes, recovering the full 6-DoF pose
(position and orientation) of every marker with sub-millimeter accuracy. The
coded light makes markers identifiable and trackable even in direct sunlight,
for both indoor and outdoor use.

This API lets external applications talk to Enpose optical marker trackers on
the local network. It covers the full client workflow:

- **Discovery** — find Enpose devices on the local network.
- **Pose streaming** — connect to a device and receive a live stream of marker
  poses (position + orientation, with per-marker error estimates and a
  timestamp).

The API is implemented in Rust and exposed to other languages through a C ABI.
Four bindings are provided:

| Language | Location | Notes |
|----------|----------|-------|
| Rust     | [`rust/`](rust)     | Native crate; the source of truth for the API. |
| C        | [`c/`](c)           | C header over the shared library. |
| C++      | [`cpp/`](cpp)       | Header-only RAII wrapper over the C API. |
| Python   | [`python/`](python) | cffi binding that loads the shared library at run time. |
| .NET     | [`dotnet/`](dotnet) | P/Invoke binding that loads the shared library at run time. |

## Coordinate system

Poses are reported in a right-handed world frame whose origin and axes are
decided by the device's extrinsics calibration:

- **By default** the frame is the reference sensor's optical frame: the origin
  sits at that sensor, **+X** points to its right, **+Y** down, and **+Z** into
  the scene — so a marker in front of the device has a positive `z`.
- **Calibrated against a measured origin**, the frame is the one marked out
  there instead.
- **With a measured ground plane**, that plane becomes `z = 0` and **+Z** points
  up out of it.

Positions are in meters. Treat the axes as a property of the calibration rather
than a fixed convention — recalibrating a device can move the frame.

## Layout

```
enpose-api/
├── CMakeLists.txt   top-level CMake package (C/C++ targets, install, packaging)
├── rust/            Rust crate (the API and the C ABI it exports)
├── c/               C header + example
├── cpp/             header-only C++ wrapper + example
├── python/          cffi binding + example
└── dotnet/          P/Invoke binding + example
```

## Using it from C or C++

The top-level CMake package exposes two targets — linking one pulls in the right
header(s) **and** the shared library, so there is nothing to copy or locate:

- `enpose_api::enpose_api` — the C interface.
- `enpose_api::enpose_api_cpp` — the header-only C++ wrapper.

### From a CMake project (recommended)

Pull the repository in with `FetchContent` (or a submodule + `add_subdirectory`)
and link a target. The Rust library is built automatically via `cargo`:

```cmake
include(FetchContent)
FetchContent_Declare(enpose_api
    GIT_REPOSITORY https://github.com/enpose-tech/enpose-api
    GIT_TAG main)
FetchContent_MakeAvailable(enpose_api)

target_link_libraries(my_app PRIVATE enpose_api::enpose_api_cpp)  # or ::enpose_api for C
```

### From a prebuilt binary SDK (no Rust toolchain)

Build a relocatable package once, then ship it to consumers who only need the
binaries:

```bash
cmake -S . -B build
cmake --build build --target dist   # -> build/enpose_api-<version>-<system>.{tar.gz,zip}
```

Unpacked, that package mirrors this repository — one directory per binding,
each with its own README — so the two read the same way:

```
enpose_api-<version>-<system>/
├── LICENSE             the license, at the top level
├── README.md           consumer-facing overview of the unpacked package
├── include/            the C and C++ headers
├── lib/                the shared library + CMake config (lib/cmake/enpose_api)
├── docs/               generated API docs, one HTML site per binding (c/ cpp/ python/ dotnet/ rust/)
├── c/  cpp/            example/ — the example + a CMakeLists.txt that builds
│                       via find_package (the headers live in include/)
├── rust/               the full crate, built from source with cargo
├── python/             the cffi binding + example, loaded at run time
└── dotnet/             the P/Invoke binding + example, loaded at run time
```

Only the SDK proper departs from the repository layout: the headers and the
shared library sit in `include/` and `lib/`, the conventional locations that
`find_package` and the Python and .NET loaders look in.

The C and C++ examples build against the prebuilt library with no Rust
toolchain, the Python and .NET bindings load it at run time, and the full Rust
crate is bundled too for consumers that build from source:

```cmake
find_package(enpose_api CONFIG REQUIRED)               # point at the SDK via CMAKE_PREFIX_PATH
target_link_libraries(my_app PRIVATE enpose_api::enpose_api_cpp)
```

`cmake --install build --prefix <dir>` installs just the SDK (the `include/`
and `lib/` directories — headers, the shared library, and the CMake config);
`docs/` and the binding directories are bundled into the package only. Each
bundled binding builds (or runs) on its own — see the README beside it (or
[`c/README.md`](c/README.md), [`cpp/README.md`](cpp/README.md),
[`rust/README.md`](rust/README.md), [`python/README.md`](python/README.md),
[`dotnet/README.md`](dotnet/README.md) in this repo) for the one-line build.

## Building from source

### Rust

```bash
cd rust
cargo build --release
cargo run --example example   # discover devices and stream poses
```

### C / C++ examples

Build the examples through the top-level project:

```bash
cmake -S . -B build -DENPOSE_BUILD_EXAMPLES=ON
cmake --build build
./build/c/example/enpose_example
./build/cpp/example/enpose_example_cpp
```

### Python

The cffi binding needs no build — just the shared library on the loader path:

```bash
pip install cffi
cargo build --release --manifest-path rust/Cargo.toml   # produces the library
LD_LIBRARY_PATH=rust/target/release python3 python/example/example.py
```

### .NET

The P/Invoke binding needs no native build either — just the shared library,
pointed at with `ENPOSE_API_LIB` (or reachable on the loader path):

```bash
cargo build --release --manifest-path rust/Cargo.toml   # produces the library
ENPOSE_API_LIB=rust/target/release/libenpose_api.so dotnet run --project dotnet/example
```

See [`rust/README.md`](rust/README.md), [`c/README.md`](c/README.md),
[`cpp/README.md`](cpp/README.md), [`python/README.md`](python/README.md), and
[`dotnet/README.md`](dotnet/README.md) for per-binding details.

## API documentation

Each binding's API reference is generated from its own sources by its own
standard tool, into `docs/<binding>/`:

| Binding  | Generator      | Install with                    |
|----------|----------------|---------------------------------|
| C, C++   | Doxygen        | your package manager            |
| Python   | Sphinx         | `pip install sphinx furo`       |
| .NET     | docfx          | `dotnet tool install -g docfx`  |
| Rust     | rustdoc        | ships with the Rust toolchain   |

Build them all with the `docs` target:

```bash
cmake -S . -B build-docs -DENPOSE_BUILD_EXAMPLES=OFF
cmake --build build-docs --target docs
```

Then open `docs/<binding>/index.html`. (`ENPOSE_BUILD_EXAMPLES=OFF` because the
docs need no compiled artifacts; any build tree will do, so an existing `build/`
works just as well. To start from scratch, delete `docs/` and the build tree.)

Generation is **best-effort**: each binding is generated independently, and one
whose generator is missing — or that cannot run in this environment — is skipped
with a note rather than failing the others. The run closes with a summary
listing what was produced, what was skipped for want of a generator, and what
failed (with a `build-docs/docs-<binding>.log` to read). Configure with
`-DENPOSE_DOCS_STRICT=ON` to turn an incomplete run into an error instead, for
CI or a release build where every binding is expected.

On Windows, every generator must come from the same toolchain as CMake — all
MSYS2/MinGW, or all native. A tool from the other chain cannot consume the paths
CMake hands it and is reported as failed. Note also that docfx shells out to
`dotnet`, so the .NET SDK has to be on `PATH` of the shell you build from.

The `dist` package runs this same generation and bundles whatever it produced.

## License

MIT — see [LICENSE](LICENSE).
