namespace Enpose
{
    /// <summary>
    /// Pose of one tracked marker in world coordinates.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Units: <see cref="X"/>/<see cref="Y"/>/<see cref="Z"/> and
    /// <see cref="PositionRmse"/> are in meters; <see cref="RotationRmse"/> is
    /// in radians.
    /// </para>
    /// <para>
    /// The world frame is right-handed, and where its origin and axes lie is
    /// decided by the device's extrinsics calibration. By default it is the
    /// reference sensor's optical frame — origin at that sensor, <c>+X</c> to
    /// its right, <c>+Y</c> down and <c>+Z</c> into the scene, so a marker in
    /// front of the device has a positive <c>z</c>. Calibrating against a
    /// measured origin puts the frame there instead, and measuring a ground
    /// plane makes that plane <c>z = 0</c> with <c>+Z</c> pointing up out of
    /// it. Treat the axes as a property of the calibration rather than a fixed
    /// convention: recalibrating a device can move the frame.
    /// </para>
    /// </remarks>
    public readonly struct MarkerPose
    {
        /// <summary>Create a marker pose.</summary>
        /// <param name="timestamp">Microseconds since the device started.</param>
        /// <param name="markerId">Marker identifier.</param>
        /// <param name="x">World-space X position in meters.</param>
        /// <param name="y">World-space Y position in meters.</param>
        /// <param name="z">World-space Z position in meters.</param>
        /// <param name="rotation">Row-major 3x3 rotation (9 values), model frame to world.</param>
        /// <param name="positionRmse">RMS position error in meters.</param>
        /// <param name="rotationRmse">RMS rotation error in radians.</param>
        /// <param name="sensors">Number of sensors that contributed.</param>
        /// <param name="observedEmitters">Number of marker emitters that contributed.</param>
        public MarkerPose(ulong timestamp, ushort markerId, double x, double y, double z,
                          double[] rotation, double positionRmse, double rotationRmse,
                          byte sensors, byte observedEmitters)
        {
            Timestamp = timestamp;
            MarkerId = markerId;
            X = x;
            Y = y;
            Z = z;
            Rotation = rotation;
            PositionRmse = positionRmse;
            RotationRmse = rotationRmse;
            Sensors = sensors;
            ObservedEmitters = observedEmitters;
        }

        /// <summary>Microseconds since the device started.</summary>
        public ulong Timestamp { get; }

        /// <summary>Marker identifier.</summary>
        public ushort MarkerId { get; }

        /// <summary>World-space X position in meters (of the first model emitter).</summary>
        public double X { get; }

        /// <summary>World-space Y position in meters.</summary>
        public double Y { get; }

        /// <summary>World-space Z position in meters.</summary>
        public double Z { get; }

        /// <summary>
        /// Row-major 3x3 rotation (9 values), model frame to world. Each pose
        /// owns its own array.
        /// </summary>
        public double[] Rotation { get; }

        /// <summary>RMS position error in meters (the same units as X/Y/Z).</summary>
        public double PositionRmse { get; }

        /// <summary>RMS rotation error (radians, axis-angle magnitude).</summary>
        public double RotationRmse { get; }

        /// <summary>Number of sensors that contributed to this pose.</summary>
        public byte Sensors { get; }

        /// <summary>
        /// Number of marker emitters (LEDs) that contributed to this pose.
        /// </summary>
        /// <remarks>
        /// The standard marker carries four. A lower count means some were
        /// occluded or could not be decoded, so the pose was fitted from a
        /// reduced set of points and is correspondingly biased; <c>0</c> means
        /// the pose was carried purely by prediction. Filter on this value to
        /// reject poses computed from a partial view.
        /// </remarks>
        public byte ObservedEmitters { get; }
    }
}
