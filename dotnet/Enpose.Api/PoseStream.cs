using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

namespace Enpose
{
    /// <summary>
    /// A live pose stream from one device.
    /// </summary>
    /// <remarks>
    /// Disconnects on <see cref="Dispose"/> — use it with <c>using</c> — and,
    /// as a backstop, when the handle is finalized.
    /// </remarks>
    public sealed class PoseStream : IDisposable
    {
        private readonly PoseStreamHandle _handle;

        /// <summary>
        /// Connect to the device at <paramref name="ip"/>.
        /// </summary>
        /// <param name="ip">
        /// Device IPv4 address; the Enpose API is IPv4-only.
        /// </param>
        /// <param name="createThread">
        /// When true (the default and preferred mode), a background thread
        /// receives and buffers poses; otherwise poses are collected when
        /// <see cref="Receive"/> is called.
        /// </param>
        /// <exception cref="EnposeException">The stream could not be connected.</exception>
        public PoseStream(string ip, bool createThread = true)
        {
            if (ip == null)
            {
                throw new ArgumentNullException(nameof(ip));
            }
            _handle = NativeMethods.PoseStreamConnect(ip, createThread);
            if (_handle.IsInvalid)
            {
                throw new EnposeException($"failed to connect pose stream to {ip}");
            }
        }

        /// <summary>Connect to a discovered device.</summary>
        /// <param name="device">A device returned by <see cref="EnposeApi.Discover"/>.</param>
        /// <param name="createThread">
        /// When true (the default and preferred mode), a background thread
        /// receives and buffers poses.
        /// </param>
        /// <exception cref="EnposeException">The stream could not be connected.</exception>
        public PoseStream(DeviceInfo device, bool createThread = true)
            : this((device ?? throw new ArgumentNullException(nameof(device))).Ip, createThread)
        {
        }

        /// <summary>Return poses received from the stream.</summary>
        /// <param name="block">
        /// When false (the default), returns the poses that have arrived since
        /// the previous call (empty if none) without waiting. When true, waits
        /// for at least one pose update, up to a 3-second timeout — so the
        /// result is still empty if none arrives in that window.
        /// </param>
        /// <returns>The poses received, in arrival order.</returns>
        /// <exception cref="EnposeException">An unrecoverable failure occurred.</exception>
        /// <exception cref="ObjectDisposedException">The stream is already disposed.</exception>
        public IReadOnlyList<MarkerPose> Receive(bool block = false)
        {
            if (_handle.IsClosed)
            {
                throw new ObjectDisposedException(nameof(PoseStream));
            }

            Interop.ThrowIfFailed("enpose_pose_stream_receive",
                NativeMethods.PoseStreamReceive(_handle, block, out IntPtr poses, out UIntPtr count));

            int n = Interop.Count(count);
            var result = new List<MarkerPose>(n);
            try
            {
                int size = Marshal.SizeOf<NativeMarkerPose>();
                for (int i = 0; i < n; i++)
                {
                    var native = Marshal.PtrToStructure<NativeMarkerPose>(poses + i * size);
                    result.Add(new MarkerPose(
                        native.Timestamp, native.MarkerId,
                        native.X, native.Y, native.Z,
                        native.Rotation,
                        native.PositionRmse, native.RotationRmse,
                        native.Sensors, native.ObservedEmitters));
                }
            }
            finally
            {
                // The array is owned by the library; hand it straight back.
                NativeMethods.MarkerPoseArrayFree(poses, count);
            }
            return result;
        }

        /// <summary>Disconnect and release the stream. Idempotent.</summary>
        public void Dispose()
        {
            _handle.Dispose();
        }
    }
}
