# Enpose API SDK

A prebuilt SDK for the Enpose 6-DoF optical tracking API: discover devices on
the local network, open a pose stream, and read the 6-DoF pose of each tracked
marker. This archive carries everything a consumer needs — no Rust toolchain
required for the C, C++, or Python bindings.

## Layout

```
.
├── LICENSE             the license
├── README.md           this file
├── include/            the C (enpose_api.h) and C++ (enpose_api.hpp) headers
├── lib/                the shared library + CMake config (lib/cmake/enpose_api)
├── docs/               generated API docs, one HTML site per binding (c/ cpp/ python/ rust/)
└── examples/           buildable sources for all four bindings
    ├── c/  cpp/        example + a CMakeLists.txt that builds via find_package
    ├── rust/           the full crate, built from source with cargo
    └── python/         the cffi binding, loaded at run time
```

Open `docs/<binding>/index.html` in a browser for the full API reference.

## Using the SDK (C / C++)

The headers and shared library are a standard CMake package. Point a consumer
project at this directory and link the binding you want:

```cmake
find_package(enpose_api CONFIG REQUIRED)   # -DCMAKE_PREFIX_PATH=<this directory>
target_link_libraries(my_app PRIVATE enpose_api::enpose_api_cpp)  # or enpose_api_c
```

The C and C++ examples build against the prebuilt library with no Rust
toolchain:

```bash
cmake -S examples/cpp -B build -DCMAKE_PREFIX_PATH="$PWD"
cmake --build build
```

## Python

The cffi binding loads the prebuilt shared library at run time — no build step,
just put `lib/` on the loader path:

```bash
pip install cffi
LD_LIBRARY_PATH=lib python3 examples/python/example.py
```

## Rust

The full crate is bundled for consumers that build from source:

```bash
cd examples/rust
cargo run --example example   # discover devices and stream poses
```

## License

See [LICENSE](LICENSE).
