//! Client-side discovery of Enpose tracker devices on the local network.
//!
//! Use [`DeviceDiscovery::new`] followed by [`DeviceDiscovery::discover`]
//! to find devices reachable on the current L2 segment.

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use crate::protocol::{
    BROADCAST_PORT, PACKET_SIZE, PKT_TYPE_PEER_INFO, PROTOCOL_VERSION,
    encode_discovery_request, parse_packet,
};

/// Hard cap on how long [`DeviceDiscovery::discover`] is allowed to
/// spend before returning, regardless of how many replies have arrived.
const TOTAL_BUDGET: Duration = Duration::from_millis(500);

/// Time to wait for the very first reply. Sized to cover broadcast
/// propagation plus the responding device's role-negotiation tick
/// latency.
const FIRST_WAIT: Duration = Duration::from_millis(200);

/// Time to wait between subsequent replies. Tighter than [`FIRST_WAIT`]
/// because once at least one device has replied, any additional replies
/// are already in flight and arrive close together.
const NEXT_WAIT: Duration = Duration::from_millis(50);

/// Information about one tracker device discovered on the local network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    /// Address the device replied from. This is the device's primary
    /// IP on the cluster network and the address an Enpose API client
    /// should use to connect to it.
    pub ip: IpAddr,
    /// Factory serial number of the device.
    pub serial: u32,
    /// `true` when the device's wire-protocol version matches the
    /// version this API was built against.
    ///
    /// When `false`, the IP and serial are still populated so the
    /// caller can surface a "found but incompatible" entry to the user
    /// rather than silently hiding the device.
    pub compatible: bool,
}

/// Discovers Enpose tracker devices on the local network.
///
/// On every call to [`Self::discover`] the API sends a discovery
/// request to every directed-broadcast address of every up,
/// non-loopback IPv4 interface on the host (e.g. `192.168.10.255` on
/// a host with `enp1s0 192.168.10.10/24`), plus the limited-broadcast
/// address `255.255.255.255`. This reaches the cluster network on
/// multi-NIC hosts where the kernel's default route would otherwise
/// send the limited broadcast out the wrong interface (typically the
/// wifi default route, leaving the cluster ethernet unreachable).
///
/// A single instance can be reused for many discovery rounds; the API
/// holds no persistent network state between calls.
pub struct DeviceDiscovery {
    /// When `Some`, [`Self::discover`] sends the request only to that
    /// address — used by tests so a fake primary on loopback gets the
    /// request without broadcasting on the LAN. When `None`,
    /// [`Self::discover`] enumerates host interfaces and broadcasts to
    /// every directed-broadcast plus `255.255.255.255`.
    explicit_target: Option<SocketAddr>,
}

impl Default for DeviceDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceDiscovery {
    /// Create a new [`DeviceDiscovery`] that, on each [`Self::discover`]
    /// call, broadcasts to every directed-broadcast on the host plus
    /// `255.255.255.255` — all on [`BROADCAST_PORT`].
    pub fn new() -> Self {
        Self {
            explicit_target: None,
        }
    }

    /// Test constructor that targets a single specific socket address
    /// instead of enumerating broadcasts. Lets unit tests run a fake
    /// primary on loopback without depending on host interfaces.
    #[cfg(test)]
    pub(crate) fn with_target(target: SocketAddr) -> Self {
        Self {
            explicit_target: Some(target),
        }
    }

    /// Send a single discovery request and collect replies.
    ///
    /// Broadcasts one discovery request to [`BROADCAST_PORT`] and
    /// listens on the same socket for [`PKT_TYPE_PEER_INFO`] replies.
    /// Only the elected primary of a cluster replies, so a cluster
    /// contributes one entry; multiple clusters on the same L2 segment
    /// each contribute one entry.
    ///
    /// Timing: waits up to 200 ms for the first reply, then up to 50 ms
    /// for each subsequent reply, with a hard 500 ms total cap.
    /// Returns as soon as a wait window elapses with no reply.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] only for unrecoverable socket failures
    /// (bind, send, or unexpected `recv` errors). A normal "no devices
    /// found" outcome returns `Ok(vec![])`.
    pub fn discover(&self) -> io::Result<Vec<DeviceInfo>> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        socket.set_broadcast(true)?;

        let request = encode_discovery_request();
        for target in self.discovery_targets() {
            // Per-interface send_to may fail (e.g. interface went down
            // between enumeration and send); other targets still get a
            // chance, so swallow the error and keep going.
            let _ = socket.send_to(&request, target);
        }

        let mut devices: Vec<DeviceInfo> = Vec::new();
        let mut wait = FIRST_WAIT;
        let start = Instant::now();
        let mut buf = [0u8; PACKET_SIZE];

        loop {
            let elapsed = start.elapsed();
            if elapsed >= TOTAL_BUDGET {
                break;
            }
            let remaining = TOTAL_BUDGET - elapsed;
            socket.set_read_timeout(Some(wait.min(remaining)))?;

            match socket.recv_from(&mut buf) {
                Ok((n, src)) => {
                    let Some(parsed) = parse_packet(&buf[..n]) else {
                        continue;
                    };
                    if parsed.pkt_type != PKT_TYPE_PEER_INFO {
                        continue;
                    }
                    // Bridged networks occasionally echo our own broadcast
                    // back; drop any duplicate-by-serial reply so the
                    // returned list reflects unique devices.
                    if devices.iter().any(|d| d.serial == parsed.serial) {
                        continue;
                    }
                    devices.push(DeviceInfo {
                        ip: src.ip(),
                        serial: parsed.serial,
                        compatible: parsed.version == PROTOCOL_VERSION,
                    });
                    wait = NEXT_WAIT;
                }
                Err(e)
                    if e.kind() == io::ErrorKind::WouldBlock
                        || e.kind() == io::ErrorKind::TimedOut =>
                {
                    // No reply within the current wait window — we are done.
                    // WouldBlock on Linux, TimedOut on Windows.
                    break;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(devices)
    }

    /// Compute the set of addresses [`Self::discover`] should send the
    /// request to. When the caller supplied an explicit target (tests),
    /// that's the only entry. Otherwise: every directed broadcast for
    /// every up, non-loopback IPv4 interface on the host, plus
    /// `255.255.255.255` as a fallback for setups where directed
    /// broadcasts are filtered or the enumeration finds nothing.
    fn discovery_targets(&self) -> Vec<SocketAddr> {
        if let Some(target) = self.explicit_target {
            return vec![target];
        }
        let mut targets: Vec<SocketAddr> = enumerate_broadcast_addresses()
            .into_iter()
            .map(|ip| SocketAddr::from((ip, BROADCAST_PORT)))
            .collect();
        targets.push(SocketAddr::from((Ipv4Addr::BROADCAST, BROADCAST_PORT)));
        targets
    }
}

/// Enumerate every up, non-loopback, non-link-local IPv4 interface on
/// the host and return its directed broadcast address — e.g. an
/// interface configured as `192.168.10.10/24` yields `192.168.10.255`.
///
/// Using the *directed* broadcast — rather than the limited broadcast
/// `255.255.255.255` — is what makes discovery reliable on multi-NIC
/// hosts. The kernel's route table maps `192.168.10.0/24` to the
/// interface that owns it, so a `send_to(192.168.10.255, ...)` always
/// leaves via that interface. The limited broadcast follows the
/// default route, which on a host with both ethernet and wifi
/// typically points at wifi — the wrong network for a wired cluster.
///
/// Implemented via the `if-addrs` crate, which wraps `getifaddrs(3)`
/// on Unix and `GetAdaptersAddresses` on Windows. Loopback
/// (`127.0.0.0/8`) is skipped because it never reaches a remote
/// device; link-local (`169.254.0.0/16`) is skipped because such
/// addresses are auto-assigned when DHCP fails and broadcasting there
/// reaches nothing useful.
///
/// Returns an empty `Vec` when enumeration itself fails; the caller
/// still falls back to `255.255.255.255`.
fn enumerate_broadcast_addresses() -> Vec<Ipv4Addr> {
    let interfaces = match if_addrs::get_if_addrs() {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    interfaces
        .into_iter()
        .filter_map(|iface| match iface.addr {
            if_addrs::IfAddr::V4(v4) => Some(v4),
            _ => None,
        })
        .filter(|v4| !v4.ip.is_loopback() && !is_link_local(&v4.ip))
        .filter_map(|v4| v4.broadcast)
        .collect()
}

/// Returns `true` for IPv4 addresses in the link-local range
/// `169.254.0.0/16`. `Ipv4Addr::is_link_local` is still unstable in
/// the standard library, so we check the octets directly.
fn is_link_local(ip: &Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 169 && o[1] == 254
}

#[cfg(test)]
#[path = "devicediscovery_tests.rs"]
mod tests;
