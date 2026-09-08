namespace Enpose
{
    /// <summary>One device discovered on the local network.</summary>
    public sealed class DeviceInfo
    {
        /// <summary>Create a device description.</summary>
        /// <param name="ip">Device IPv4 address.</param>
        /// <param name="serial">Factory serial number.</param>
        /// <param name="compatible">Whether the device's protocol version matches.</param>
        public DeviceInfo(string ip, uint serial, bool compatible)
        {
            Ip = ip;
            Serial = serial;
            Compatible = compatible;
        }

        /// <summary>Device IPv4 address.</summary>
        public string Ip { get; }

        /// <summary>Factory serial number.</summary>
        public uint Serial { get; }

        /// <summary>True if the device's protocol version matches this binding.</summary>
        public bool Compatible { get; }
    }
}
