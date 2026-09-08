# Enpose API SDK

A prebuilt SDK for the Enpose 6-DoF optical tracking API: discover devices on
the local network, open a pose stream, and read the 6-DoF pose of each tracked
marker. This archive carries everything a consumer needs — no Rust toolchain
required for the C, C++, Python, or .NET bindings.

## Layout

```
.
├── LICENSE             the license
├── README.md           this file
├── include/            the C (enpose_api.h) and C++ (enpose_api.hpp) headers
├── lib/                the shared library + CMake config (lib/cmake/enpose_api)
├── docs/               generated API docs, one HTML site per binding (c/ cpp/ python/ dotnet/ rust/)
└── one directory per binding, each with its own README.md:
    ├── c/  cpp/        example/ — the example + a CMakeLists.txt that builds
    │                   via find_package (the headers are in include/)
    ├── rust/           the full crate, built from source with cargo
    ├── python/         the cffi binding + example, loaded at run time
    └── dotnet/         the P/Invoke binding + example, loaded at run time
```

The binding directories mirror the [source
repository](https://github.com/enpose-tech/enpose-api), so anything written
about the repo layout applies here too. Open `docs/<binding>/index.html` in a
browser for the full API reference, and see `<binding>/README.md` for how to
build and run that binding.

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
cmake -S cpp/example -B build -DCMAKE_PREFIX_PATH="$PWD"
cmake --build build
```

## Python

The cffi binding loads the prebuilt shared library at run time — no build step,
just put `lib/` on the loader path:

```bash
pip install cffi
LD_LIBRARY_PATH=lib python3 python/example/example.py
```

## .NET

The P/Invoke binding loads the prebuilt shared library at run time as well, and
finds this package's `lib/` directory on its own:

```bash
cd dotnet/example
dotnet run                    # discover devices and stream poses
```

## Rust

The full crate is bundled for consumers that build from source:

```bash
cd rust
cargo run --example example   # discover devices and stream poses
```

## License

See [LICENSE](LICENSE).
