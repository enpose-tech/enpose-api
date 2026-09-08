using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

namespace Enpose
{
    /// <summary>
    /// Entry point of the Enpose .NET binding: device discovery on the local
    /// network. Connect to a discovered device with <see cref="PoseStream"/>.
    /// </summary>
    public static class EnposeApi
    {
        /// <summary>Discover Enpose devices on the local network.</summary>
        /// <returns>
        /// One entry per device, empty if none are found.
        /// </returns>
        /// <exception cref="EnposeException">Discovery failed (e.g. an I/O error).</exception>
        /// <exception cref="DllNotFoundException">
        /// The <c>enpose_api</c> shared library could not be located; see the
        /// binding's README for the search order.
        /// </exception>
        public static IReadOnlyList<DeviceInfo> Discover()
        {
            Interop.ThrowIfFailed("enpose_discover",
                NativeMethods.Discover(out IntPtr devices, out UIntPtr count));

            int n = Interop.Count(count);
            var result = new List<DeviceInfo>(n);
            try
            {
                int size = Marshal.SizeOf<NativeDeviceInfo>();
                for (int i = 0; i < n; i++)
                {
                    var native = Marshal.PtrToStructure<NativeDeviceInfo>(devices + i * size);
                    result.Add(new DeviceInfo(native.Ip, native.Serial, native.Compatible));
                }
            }
            finally
            {
                // The array is owned by the library; hand it straight back.
                NativeMethods.DeviceInfoArrayFree(devices, count);
            }
            return result;
        }
    }
}
