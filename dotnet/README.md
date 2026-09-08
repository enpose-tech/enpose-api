# Enpose API — .NET interface

.NET applications use the Enpose tracking API through a managed
[P/Invoke](https://learn.microsoft.com/dotnet/standard/native-interop/pinvoke)
binding over the `enpose_api` C ABI. It loads the prebuilt shared library at
run time, so there is no native build step and no C or Rust toolchain involved —
only the shared library and the `Enpose.Api` assembly.

## Requirements

- .NET 8.0 or newer — or any runtime that consumes .NET Standard 2.0
  (.NET Framework 4.6.1+, Mono, Unity).
- The `enpose_api` shared library (shipped in the SDK's `lib/` directory).

## Locating the shared library

On .NET 5 and newer the binding searches, in order:

1. `ENPOSE_API_LIB`, if set, as an explicit path to the library.
2. a `lib/` directory in any ancestor of the application directory (the SDK
   layout).
3. the bare library name, resolved by the runtime's default probing — the
   application directory and the system loader path (`LD_LIBRARY_PATH` on
   Linux, `DYLD_LIBRARY_PATH` on macOS, `PATH` on Windows).

So an app run from inside the unpacked SDK finds the library with no setup at
all; anywhere else, either set `ENPOSE_API_LIB=<sdk>/lib/libenpose_api.so` or
put `<sdk>/lib` on your loader path. On the .NET Standard 2.0 target
(.NET Framework, Mono, Unity) only step 3 applies, since the custom resolver
needs .NET 5+.

If the library cannot be found, the first call throws `DllNotFoundException`.

## Usage

```csharp
using Enpose;

foreach (DeviceInfo device in EnposeApi.Discover())
{
    if (!device.Compatible) continue;
    using var stream = new PoseStream(device);   // threaded by default
    // Receive(block: true) waits for a pose update (up to a 3-second timeout);
    // Receive() (the default) returns immediately with whatever has arrived.
    foreach (MarkerPose pose in stream.Receive(block: true))
    {
        Console.WriteLine($"{pose.MarkerId} {pose.X} {pose.Y} {pose.Z}");
    }
}
```

`EnposeApi.Discover()` returns an `IReadOnlyList<DeviceInfo>`.
`PoseStream.Receive(block: false)` returns an `IReadOnlyList<MarkerPose>`; with
`block: true` it waits for at least one update, up to a 3-second timeout,
otherwise it returns immediately (poll it repeatedly). Failures throw
`EnposeException`. The stream disconnects when it is disposed — use it with
`using`, as above.

## Consuming the API

Reference the binding project from your own project:

```xml
<ProjectReference Include="<sdk>/dotnet/Enpose.Api/Enpose.Api.csproj" />
```

(or `dotnet add reference` it, or build the project once and reference the
resulting `Enpose.Api.dll`). Nothing needs to be copied at build time; the
shared library is located at run time as described above.

## Running the bundled example

`example/Example.cs` is a complete discover-and-stream program. From the
`dotnet/example` directory of the unpacked SDK:

```bash
dotnet run
```

There it picks up the library from the SDK's `lib/` directory automatically. In
a source checkout, point the binding at the freshly built library instead:

```bash
cargo build --release --manifest-path rust/Cargo.toml
ENPOSE_API_LIB=rust/target/release/libenpose_api.so dotnet run --project dotnet/example
```

## API reference

The full API reference — every class, method, and property — is published at
<https://enpose.tech/docs/dotnet/>.
