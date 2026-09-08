// P/Invoke layer over the `enpose_api` C ABI.
//
// Everything here is internal and mirrors c/include/enpose_api.h field for
// field and function for function; the public, idiomatic surface lives in
// EnposeApi.cs, PoseStream.cs, DeviceInfo.cs and MarkerPose.cs.

using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
#if NET5_0_OR_GREATER
using System.Reflection;
#endif

namespace Enpose
{
    /// <summary>Result code returned by the fallible C functions.</summary>
    internal enum EnposeStatus
    {
        Ok = 0,
        InvalidArg = -1,
        Io = -2,
        Panic = -3,
    }

    /// <summary>C view of one discovered device (<c>EnposeDeviceInfo</c>).</summary>
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Ansi)]
    internal struct NativeDeviceInfo
    {
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 46)]
        public string Ip;
        public uint Serial;
        [MarshalAs(UnmanagedType.I1)]
        public bool Compatible;
    }

    /// <summary>C view of one marker pose (<c>EnposeMarkerPose</c>).</summary>
    [StructLayout(LayoutKind.Sequential)]
    internal struct NativeMarkerPose
    {
        public ulong Timestamp;
        public ushort MarkerId;
        public double X;
        public double Y;
        public double Z;
        [MarshalAs(UnmanagedType.ByValArray, SizeConst = 9)]
        public double[] Rotation;
        public double PositionRmse;
        public double RotationRmse;
        public byte Sensors;
        public byte ObservedEmitters;
    }

    /// <summary>
    /// Owning handle to a native <c>EnposePoseStream</c>. Releases the stream
    /// on disposal or finalization, and keeps it alive across a P/Invoke call
    /// that takes it as an argument.
    /// </summary>
    internal sealed class PoseStreamHandle : SafeHandle
    {
        // Constructed by the interop marshaller when a P/Invoke returns one.
        private PoseStreamHandle() : base(IntPtr.Zero, ownsHandle: true)
        {
        }

        public override bool IsInvalid => handle == IntPtr.Zero;

        protected override bool ReleaseHandle()
        {
            NativeMethods.PoseStreamFree(handle);
            return true;
        }
    }

    internal static class NativeMethods
    {
        /// <summary>Base name of the shared library, without prefix or extension.</summary>
        internal const string LibraryName = "enpose_api";

        static NativeMethods()
        {
            // Both run before the first P/Invoke below: the structs above are
            // checked against the ABI they transcribe (see AbiLayout.cs), and
            // every call then goes through the custom library search order.
            AbiLayout.Verify();
#if NET5_0_OR_GREATER
            NativeLibrary.SetDllImportResolver(typeof(NativeMethods).Assembly, Resolve);
#endif
        }

        [DllImport(LibraryName, EntryPoint = "enpose_discover",
                   CallingConvention = CallingConvention.Cdecl)]
        internal static extern EnposeStatus Discover(out IntPtr devices, out UIntPtr count);

        [DllImport(LibraryName, EntryPoint = "enpose_device_info_array_free",
                   CallingConvention = CallingConvention.Cdecl)]
        internal static extern void DeviceInfoArrayFree(IntPtr devices, UIntPtr count);

        [DllImport(LibraryName, EntryPoint = "enpose_pose_stream_connect",
                   CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
        internal static extern PoseStreamHandle PoseStreamConnect(
            string ip, [MarshalAs(UnmanagedType.I1)] bool createThread);

        [DllImport(LibraryName, EntryPoint = "enpose_pose_stream_receive",
                   CallingConvention = CallingConvention.Cdecl)]
        internal static extern EnposeStatus PoseStreamReceive(
            PoseStreamHandle stream, [MarshalAs(UnmanagedType.I1)] bool block,
            out IntPtr poses, out UIntPtr count);

        [DllImport(LibraryName, EntryPoint = "enpose_marker_pose_array_free",
                   CallingConvention = CallingConvention.Cdecl)]
        internal static extern void MarkerPoseArrayFree(IntPtr poses, UIntPtr count);

        [DllImport(LibraryName, EntryPoint = "enpose_pose_stream_free",
                   CallingConvention = CallingConvention.Cdecl)]
        internal static extern void PoseStreamFree(IntPtr stream);

#if NET5_0_OR_GREATER
        /// <summary>
        /// Resolve the shared library: an explicit <c>ENPOSE_API_LIB</c> path
        /// first, then a <c>lib/</c> directory in any ancestor of the
        /// application directory (the SDK layout). Returning
        /// <see cref="IntPtr.Zero"/> hands the name back to the default
        /// probing logic, which searches the system loader path.
        /// </summary>
        private static IntPtr Resolve(string libraryName, Assembly assembly,
                                      DllImportSearchPath? searchPath)
        {
            if (libraryName != LibraryName)
            {
                return IntPtr.Zero;
            }

            foreach (string candidate in CandidatePaths(FileName()))
            {
                if (NativeLibrary.TryLoad(candidate, out IntPtr handle))
                {
                    return handle;
                }
            }

            return IntPtr.Zero;
        }

        /// <summary>Platform file name of the shared library.</summary>
        private static string FileName()
        {
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                return "enpose_api.dll";
            }
            if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            {
                return "libenpose_api.dylib";
            }
            return "libenpose_api.so";
        }

        /// <summary>Library locations to try, most specific first.</summary>
        private static IEnumerable<string> CandidatePaths(string fileName)
        {
            string? overridePath = Environment.GetEnvironmentVariable("ENPOSE_API_LIB");
            if (!string.IsNullOrEmpty(overridePath))
            {
                yield return overridePath!;
            }

            // In the SDK layout the library sits in a `lib/` directory above
            // the application; check every ancestor for one.
            for (DirectoryInfo? dir = new DirectoryInfo(AppContext.BaseDirectory);
                 dir != null; dir = dir.Parent)
            {
                yield return Path.Combine(dir.FullName, "lib", fileName);
            }
        }
#endif
    }

    /// <summary>Shared helpers for turning C results into .NET results.</summary>
    internal static class Interop
    {
        // Human-readable text for the non-OK status codes.
        private static string StatusText(EnposeStatus status)
        {
            switch (status)
            {
                case EnposeStatus.InvalidArg: return "invalid argument";
                case EnposeStatus.Io: return "I/O error";
                case EnposeStatus.Panic: return "internal error";
                default: return "status " + (int)status;
            }
        }

        /// <summary>Throw an <see cref="EnposeException"/> unless the call succeeded.</summary>
        internal static void ThrowIfFailed(string function, EnposeStatus status)
        {
            if (status != EnposeStatus.Ok)
            {
                throw new EnposeException($"{function} failed: {StatusText(status)}");
            }
        }

        /// <summary>Narrow a C <c>size_t</c> element count to an array length.</summary>
        internal static int Count(UIntPtr count)
        {
            ulong value = count.ToUInt64();
            if (value > int.MaxValue)
            {
                throw new EnposeException($"the library returned an implausible element count ({value})");
            }
            return (int)value;
        }
    }
}
