// Example .NET client for the Enpose API: discover a device on the local
// network, then stream and print live marker poses for a few seconds.
//
// Run it from the SDK's dotnet/example directory, where the shared
// library is picked up from the SDK's lib/ automatically (see ../README.md):
//   dotnet run
//
// Or, from the enpose-api directory, against a freshly built library:
//   cargo build --release --manifest-path rust/Cargo.toml
//   ENPOSE_API_LIB=rust/target/release/libenpose_api.so \
//       dotnet run --project dotnet/example

using System;
using System.Collections.Generic;
using System.Globalization;
using Enpose;

namespace EnposeExample
{
    internal static class Program
    {
        // How many batches of poses to print before exiting.
        private const int PoseBatches = 200;

        private static int Main()
        {
            IReadOnlyList<DeviceInfo> devices;
            try
            {
                devices = EnposeApi.Discover();
            }
            catch (EnposeException e)
            {
                Console.Error.WriteLine($"error: {e.Message}");
                return 1;
            }

            if (devices.Count == 0)
            {
                Console.WriteLine("No Enpose devices found on the local network.");
                return 0;
            }

            Console.WriteLine($"Found {devices.Count} device(s):");
            DeviceInfo? chosen = null;
            foreach (DeviceInfo device in devices)
            {
                string state = device.Compatible ? "compatible" : "INCOMPATIBLE";
                Console.WriteLine($"  - {device.Ip} (serial {device.Serial}): {state}");
                if (chosen == null && device.Compatible)
                {
                    chosen = device;
                }
            }

            if (chosen == null)
            {
                Console.WriteLine("No compatible device to stream poses from.");
                return 0;
            }

            Console.WriteLine($"{Environment.NewLine}Streaming poses from {chosen.Ip}...");
            try
            {
                // Threaded mode: a background thread buffers poses between polls.
                using var stream = new PoseStream(chosen, createThread: true);

                for (int batch = 0; batch < PoseBatches; batch++)
                {
                    // Blocking receive: waits up to 3 s for a pose update, so no
                    // manual polling delay is needed.
                    foreach (MarkerPose pose in stream.Receive(block: true))
                    {
                        Console.WriteLine(
                            $"  t={pose.Timestamp,10} us  marker {pose.MarkerId,3}: " +
                            $"pos=({Coord(pose.X)}, {Coord(pose.Y)}, {Coord(pose.Z)}) " +
                            $"sensors={pose.Sensors} emitters={pose.ObservedEmitters}");
                    }
                }
            }
            catch (EnposeException e)
            {
                Console.Error.WriteLine($"error: {e.Message}");
                return 1;
            }

            return 0;
        }

        // A signed coordinate in meters, always with a '.' decimal separator
        // (invariant culture) so the output matches the other bindings.
        private static string Coord(double meters) =>
            meters.ToString("+0.0000;-0.0000", CultureInfo.InvariantCulture);
    }
}
