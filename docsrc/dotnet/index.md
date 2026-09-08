# Enpose API — .NET

Managed P/Invoke wrapper over the `enpose_api` C ABI: it loads the prebuilt
shared library at run time, so no native build step is needed.

It covers the full client workflow:

* <xref:Enpose.EnposeApi.Discover> finds Enpose devices on the local network.
* <xref:Enpose.PoseStream> connects to a device and yields live
  <xref:Enpose.MarkerPose> updates.

```csharp
using Enpose;

foreach (DeviceInfo device in EnposeApi.Discover())
{
    if (!device.Compatible) continue;
    using var stream = new PoseStream(device);   // threaded by default
    foreach (MarkerPose pose in stream.Receive(block: true))
    {
        Console.WriteLine($"{pose.MarkerId} {pose.X} {pose.Y} {pose.Z}");
    }
}
```

Failures throw <xref:Enpose.EnposeException>. See the
[API reference](api/Enpose.yml) for every class, method, and property.
