// Load-time check that the structs in NativeMethods.cs still describe the
// Enpose C ABI.
//
// The ABI is defined by rust/src/ffi.rs, which carries the layout table and
// lists every binding that mirrors it by hand. Nothing enforces those mirrors
// mechanically across languages, and a struct that drifted does not fail
// loudly: every call would keep succeeding and return plausible-looking
// nonsense. So the transcription is verified once, before the first P/Invoke.
//
// Two complementary checks, because either alone has a blind spot:
//
//   * Size and field offsets, from the marshaller itself. These catch a
//     reordered, inserted or removed field.
//   * A round trip of a known byte pattern over a poisoned buffer. This
//     catches a field declared wider or narrower than the ABI says — which
//     can consume the padding beside it and leave every offset and the total
//     size unchanged (ushort -> uint for MarkerId, say), so the offset check
//     alone would pass while the value read is wrong.
//
// 64-bit platforms only, matching the other bindings' checks: a 32-bit x86 ABI
// aligns doubles to 4 and produces a different, equally valid layout.

using System;
using System.Runtime.InteropServices;
using System.Text;

namespace Enpose
{
    internal static class AbiLayout
    {
        /// <summary>Byte written into every position the ABI does not define.</summary>
        private const byte Poison = 0xCD;

        /// <summary>
        /// Verify both native structs, or throw <see cref="EnposeException"/>.
        /// Called from the <see cref="NativeMethods"/> type initializer, so a
        /// failure surfaces as a <see cref="TypeInitializationException"/>
        /// wrapping that exception.
        /// </summary>
        internal static void Verify()
        {
            if (IntPtr.Size != 8)
            {
                return;
            }
            VerifyDeviceInfo();
            VerifyMarkerPose();
        }

        private static void VerifyDeviceInfo()
        {
            Check("EnposeDeviceInfo size", Marshal.SizeOf<NativeDeviceInfo>(), 56);
            CheckOffset<NativeDeviceInfo>("Ip", 0);
            CheckOffset<NativeDeviceInfo>("Serial", 48);
            CheckOffset<NativeDeviceInfo>("Compatible", 52);

            IntPtr buffer = Poisoned(56);
            try
            {
                byte[] ip = Encoding.ASCII.GetBytes("192.168.0.42\0");
                Marshal.Copy(ip, 0, buffer, ip.Length);
                Marshal.WriteInt32(buffer, 48, unchecked((int)0xDEADBEEF));
                Marshal.WriteByte(buffer, 52, 1);

                var device = Marshal.PtrToStructure<NativeDeviceInfo>(buffer);
                Check("EnposeDeviceInfo.Ip", device.Ip, "192.168.0.42");
                Check("EnposeDeviceInfo.Serial", device.Serial, 0xDEADBEEFu);
                Check("EnposeDeviceInfo.Compatible", device.Compatible, true);
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
            }
        }

        private static void VerifyMarkerPose()
        {
            Check("EnposeMarkerPose size", Marshal.SizeOf<NativeMarkerPose>(), 136);
            CheckOffset<NativeMarkerPose>("Timestamp", 0);
            CheckOffset<NativeMarkerPose>("MarkerId", 8);
            CheckOffset<NativeMarkerPose>("X", 16);
            CheckOffset<NativeMarkerPose>("Y", 24);
            CheckOffset<NativeMarkerPose>("Z", 32);
            CheckOffset<NativeMarkerPose>("Rotation", 40);
            CheckOffset<NativeMarkerPose>("PositionRmse", 112);
            CheckOffset<NativeMarkerPose>("RotationRmse", 120);
            CheckOffset<NativeMarkerPose>("Sensors", 128);
            CheckOffset<NativeMarkerPose>("ObservedEmitters", 129);

            IntPtr buffer = Poisoned(136);
            try
            {
                var position = new[] { 1.5, -2.5, 3.5 };
                var rotation = new[] { 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0 };
                var rmse = new[] { 0.125, 0.0625 };
                Marshal.WriteInt64(buffer, 0, 0x0102030405060708L);
                Marshal.WriteInt16(buffer, 8, 0x1234);
                Marshal.Copy(position, 0, buffer + 16, position.Length);
                Marshal.Copy(rotation, 0, buffer + 40, rotation.Length);
                Marshal.Copy(rmse, 0, buffer + 112, rmse.Length);
                Marshal.WriteByte(buffer, 128, 5);
                Marshal.WriteByte(buffer, 129, 4);

                var pose = Marshal.PtrToStructure<NativeMarkerPose>(buffer);
                Check("EnposeMarkerPose.Timestamp", pose.Timestamp, 0x0102030405060708UL);
                Check("EnposeMarkerPose.MarkerId", pose.MarkerId, (ushort)0x1234);
                Check("EnposeMarkerPose.X", pose.X, position[0]);
                Check("EnposeMarkerPose.Y", pose.Y, position[1]);
                Check("EnposeMarkerPose.Z", pose.Z, position[2]);
                Check("EnposeMarkerPose.Rotation length", pose.Rotation.Length, rotation.Length);
                for (int i = 0; i < rotation.Length; i++)
                {
                    Check($"EnposeMarkerPose.Rotation[{i}]", pose.Rotation[i], rotation[i]);
                }
                Check("EnposeMarkerPose.PositionRmse", pose.PositionRmse, rmse[0]);
                Check("EnposeMarkerPose.RotationRmse", pose.RotationRmse, rmse[1]);
                Check("EnposeMarkerPose.Sensors", pose.Sensors, (byte)5);
                Check("EnposeMarkerPose.ObservedEmitters", pose.ObservedEmitters, (byte)4);
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
            }
        }

        /// <summary>Allocate <paramref name="size"/> bytes of poison.</summary>
        private static IntPtr Poisoned(int size)
        {
            IntPtr buffer = Marshal.AllocHGlobal(size);
            for (int i = 0; i < size; i++)
            {
                Marshal.WriteByte(buffer, i, Poison);
            }
            return buffer;
        }

        private static void CheckOffset<T>(string field, int expected)
        {
            Check($"{typeof(T).Name}.{field} offset",
                  Marshal.OffsetOf<T>(field).ToInt64(), (long)expected);
        }

        private static void Check(string what, object actual, object expected)
        {
            if (!actual.Equals(expected))
            {
                throw new EnposeException(
                    $"the .NET binding does not match the Enpose ABI, so it would misread " +
                    $"every result ({what} is {actual}, expected {expected}). The ABI is " +
                    "defined by the Rust FFI layer; see rust/src/ffi.rs.");
            }
        }
    }
}
