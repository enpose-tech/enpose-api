//! Client-side pose streaming from a single Enpose tracker device.
//!
//! A [`PoseStream`] delivers a live stream of [`MarkerPose`] updates from
//! one device. Construct it from a [`DeviceInfo`] returned by
//! [`crate::DeviceDiscovery`] or directly from an [`IpAddr`], then poll it
//! with [`PoseStream::receive_pose_updates`] to get the poses that have
//! arrived since your last call.
//!
//! The `create_thread` constructor argument selects between two modes:
//!
//! * **Threaded** (`true`) — a background thread continuously receives and
//!   buffers incoming poses, so none are missed regardless of how often you
//!   poll. [`PoseStream::receive_pose_updates`] returns the buffered poses.
//!   Use this when you cannot guarantee a regular polling cadence.
//!
//! * **Single-threaded** (`false`) — no thread is spawned; poses are
//!   collected when you call [`PoseStream::receive_pose_updates`]. Use this
//!   when you poll on your own regular cadence and want the stream to use
//!   only your thread. Poll often enough that updates are not missed between
//!   calls.
//!
//! In either mode, [`PoseStream::receive_pose_updates`] takes a `block` flag:
//! pass `false` to return whatever has arrived (possibly nothing) without
//! waiting, or `true` to wait for at least one pose update. A blocking call
//! waits at most 3 seconds, then returns an empty result if none arrived.
//!
//! The connection is closed automatically when the [`PoseStream`] is
//! dropped.

use std::io;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::devicediscovery::DeviceInfo;
use crate::protocol::{
    PACKET_SIZE, PKT_TYPE_POSE_DATA, POSE_KEEPALIVE_INTERVAL_SECS, POSE_PORT,
    encode_pose_subscribe, encode_pose_unsubscribe, parse_packet,
};
use crate::marker_pose::MarkerPose;

/// Read timeout used by the background receive thread. Bounds how long the
/// thread blocks on `recv` so it can wake up to send keep-alives and to
/// notice a stop request promptly.
const THREAD_RECV_TIMEOUT: Duration = Duration::from_millis(200);

/// Maximum time a blocking [`PoseStream::receive_pose_updates`] call (one made
/// with `block = true`) waits for a pose update before giving up and returning
/// an empty result.
const BLOCK_TIMEOUT: Duration = Duration::from_secs(3);

/// Receive buffer size. Large enough for a full pose datagram (fixed
/// header plus the MessagePack-encoded poses of one frame).
const RECV_BUF_SIZE: usize = 65536;

/// A live pose stream from one Enpose tracker device.
///
/// Dropping a `PoseStream` automatically disconnects it (see
/// [`PoseStream::disconnect`]).
pub struct PoseStream {
    /// Socket connected to the device's [`POSE_PORT`]. Shared with the
    /// background thread in threaded mode.
    socket: Arc<UdpSocket>,
    /// `false` once [`Self::disconnect`] has run, so `Drop` does not send a
    /// second disconnect packet or join an already-joined thread.
    connected: bool,
    /// Background receiver state, present only in threaded mode.
    thread: Option<ThreadState>,
    /// When the last keep-alive was sent, used only in single-threaded mode
    /// to decide when the next one is due.
    last_keepalive: Instant,
}

/// State owned by the background receive thread in threaded mode.
struct ThreadState {
    /// Poses received since the last [`PoseStream::receive_pose_updates`]
    /// call, in arrival order, paired with a condvar the receiver signals
    /// whenever it appends — so a blocking receive can wait for data.
    buffered: Arc<(Mutex<Vec<MarkerPose>>, Condvar)>,
    /// Set to request the thread to exit.
    stop: Arc<AtomicBool>,
    /// Handle to join the thread on disconnect.
    handle: Option<JoinHandle<()>>,
}

impl PoseStream {
    /// Connect a pose stream to the device at `ip`.
    ///
    /// `ip` must be IPv4 — the Enpose API is IPv4-only. An IPv6 address is
    /// rejected with [`io::ErrorKind::Unsupported`].
    ///
    /// When `create_thread` is `true`, spawns the background receiver thread
    /// described in the [module docs](crate::posestream).
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] if `ip` is not IPv4, or if the connection to
    /// the device cannot be established.
    pub fn from_ip(ip: IpAddr, create_thread: bool) -> io::Result<Self> {
        if ip.is_ipv6() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "the Enpose API supports IPv4 only",
            ));
        }
        Self::connect_to(SocketAddr::new(ip, POSE_PORT), create_thread)
    }

    /// Connect a pose stream to a device discovered via
    /// [`crate::DeviceDiscovery`].
    ///
    /// Convenience wrapper around [`Self::from_ip`] using [`DeviceInfo::ip`].
    pub fn from_device(device: &DeviceInfo, create_thread: bool) -> io::Result<Self> {
        Self::from_ip(device.ip, create_thread)
    }

    /// Test constructor that targets a full socket address instead of the
    /// fixed [`POSE_PORT`], so unit tests can run a fake device on an
    /// ephemeral loopback port.
    #[cfg(test)]
    pub(crate) fn with_target(addr: SocketAddr, create_thread: bool) -> io::Result<Self> {
        Self::connect_to(addr, create_thread)
    }

    /// Bind an ephemeral local socket, connect it to `addr`, send the
    /// initial subscribe packet, and optionally spawn the receiver thread.
    fn connect_to(addr: SocketAddr, create_thread: bool) -> io::Result<Self> {
        let socket = UdpSocket::bind((IpAddr::from([0, 0, 0, 0]), 0))?;
        socket.connect(addr)?;
        let socket = Arc::new(socket);

        // Subscribe immediately so poses start flowing without waiting for
        // the first keep-alive interval.
        socket.send(&encode_pose_subscribe())?;

        let thread = if create_thread {
            Some(Self::spawn_receiver(socket.clone()))
        } else {
            None
        };

        Ok(Self {
            socket,
            connected: true,
            thread,
            last_keepalive: Instant::now(),
        })
    }

    /// Return the marker poses received from the stream.
    ///
    /// When `block` is `false`, returns all poses that have arrived since the
    /// previous call, or an empty vector if none have — it never waits. When
    /// `block` is `true`, it waits for a pose update and returns the poses
    /// gathered so far; the wait is bounded by a 3-second timeout, so a
    /// blocking call still returns an empty vector if no update arrives within
    /// that window.
    ///
    /// Call it repeatedly to keep receiving updates. In threaded mode the
    /// poses are those gathered by the background thread.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] only for an unrecoverable communication
    /// failure. In threaded mode this never returns an error.
    pub fn receive_pose_updates(&mut self, block: bool) -> io::Result<Vec<MarkerPose>> {
        if let Some(thread) = &self.thread {
            let (buffer, available) = &*thread.buffered;
            let mut buffer = buffer.lock().unwrap();
            if block {
                // Wait until the background thread buffers a pose, or until the
                // timeout elapses (handles spurious wakeups internally).
                (buffer, _) = available
                    .wait_timeout_while(buffer, BLOCK_TIMEOUT, |b| b.is_empty())
                    .unwrap();
            }
            return Ok(std::mem::take(&mut *buffer));
        }

        // Single-threaded: send keep-alives and drain the socket ourselves.
        self.send_keepalive_if_due()?;

        let mut poses = Vec::new();
        self.drain_available(&mut poses)?;
        if block && poses.is_empty() {
            self.block_for_pose(&mut poses)?;
            self.drain_available(&mut poses)?;
        }
        Ok(poses)
    }

    /// Send a keep-alive subscribe packet if the keep-alive interval has
    /// elapsed since the previous one. Single-threaded mode only.
    fn send_keepalive_if_due(&mut self) -> io::Result<()> {
        if self.last_keepalive.elapsed() >= Duration::from_secs(POSE_KEEPALIVE_INTERVAL_SECS) {
            self.socket.send(&encode_pose_subscribe())?;
            self.last_keepalive = Instant::now();
        }
        Ok(())
    }

    /// Drain every pose datagram already waiting on the socket into `poses`,
    /// without blocking.
    fn drain_available(&self, poses: &mut Vec<MarkerPose>) -> io::Result<()> {
        self.socket.set_nonblocking(true)?;
        let mut buf = [0u8; RECV_BUF_SIZE];
        loop {
            match self.socket.recv(&mut buf) {
                Ok(n) => {
                    if let Some(batch) = parse_pose_packet(&buf[..n]) {
                        poses.extend(batch);
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Block until at least one pose datagram arrives or [`BLOCK_TIMEOUT`]
    /// elapses, appending any poses to `poses` while keeping the connection
    /// alive. Returns with `poses` still empty if the timeout is reached.
    fn block_for_pose(&mut self, poses: &mut Vec<MarkerPose>) -> io::Result<()> {
        self.socket.set_nonblocking(false)?;
        let deadline = Instant::now() + BLOCK_TIMEOUT;
        let mut buf = [0u8; RECV_BUF_SIZE];
        while poses.is_empty() {
            // Stop once the deadline passes. Otherwise wake at least once per
            // keep-alive tick, but never wait past the deadline.
            let remaining = match deadline.checked_duration_since(Instant::now()) {
                Some(d) if !d.is_zero() => d,
                _ => break,
            };
            self.socket.set_read_timeout(Some(remaining.min(THREAD_RECV_TIMEOUT)))?;
            match self.socket.recv(&mut buf) {
                Ok(n) => {
                    if let Some(batch) = parse_pose_packet(&buf[..n]) {
                        poses.extend(batch);
                    }
                }
                // The read timeout fired (reported as WouldBlock or TimedOut
                // depending on the platform): no data yet, so refresh the
                // keep-alive and keep waiting until the deadline.
                Err(e)
                    if e.kind() == io::ErrorKind::WouldBlock
                        || e.kind() == io::ErrorKind::TimedOut =>
                {
                    self.send_keepalive_if_due()?;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Disconnect the stream, closing the connection to the device.
    ///
    /// Idempotent, and called automatically when the [`PoseStream`] is
    /// dropped, so you only need to call it explicitly to disconnect early.
    pub fn disconnect(&mut self) {
        if !self.connected {
            return;
        }
        self.connected = false;

        if let Some(thread) = &mut self.thread {
            thread.stop.store(true, Ordering::Relaxed);
            if let Some(handle) = thread.handle.take() {
                let _ = handle.join();
            }
        }

        // Best-effort: the device times the connection out regardless.
        let _ = self.socket.send(&encode_pose_unsubscribe());
    }

    /// Spawn the background receiver thread and return its shared state.
    fn spawn_receiver(socket: Arc<UdpSocket>) -> ThreadState {
        let buffered = Arc::new((Mutex::new(Vec::new()), Condvar::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let thread_buffered = buffered.clone();
        let thread_stop = stop.clone();
        let handle =
            thread::spawn(move || Self::receiver_thread(socket, thread_buffered, thread_stop));
        ThreadState {
            buffered,
            stop,
            handle: Some(handle),
        }
    }

    /// Background thread body: periodically sends keep-alives and buffers
    /// incoming pose datagrams until asked to stop, signalling the condvar so
    /// a blocking receive wakes when new poses land.
    fn receiver_thread(
        socket: Arc<UdpSocket>,
        buffered: Arc<(Mutex<Vec<MarkerPose>>, Condvar)>,
        stop: Arc<AtomicBool>,
    ) {
        let _ = socket.set_read_timeout(Some(THREAD_RECV_TIMEOUT));
        let mut last_keepalive = Instant::now();
        let mut buf = [0u8; RECV_BUF_SIZE];
        let (buffer, available) = &*buffered;

        while !stop.load(Ordering::Relaxed) {
            if last_keepalive.elapsed() >= Duration::from_secs(POSE_KEEPALIVE_INTERVAL_SECS) {
                // Best-effort: a transient send error should not kill the
                // stream; the next tick retries.
                let _ = socket.send(&encode_pose_subscribe());
                last_keepalive = Instant::now();
            }

            // A recv error is either the expected read timeout (which bounds
            // the loop) or a transient failure retried next iteration; only a
            // successful read carries poses. Append them and wake any blocking
            // receive waiting on the condvar.
            if let Ok(n) = socket.recv(&mut buf)
                && let Some(batch) = parse_pose_packet(&buf[..n])
                && !batch.is_empty()
            {
                buffer.lock().unwrap().extend(batch);
                available.notify_one();
            }
        }
    }
}

impl Drop for PoseStream {
    fn drop(&mut self) {
        self.disconnect();
    }
}

/// Parse a [`PKT_TYPE_POSE_DATA`] datagram into its poses, or return
/// `None` if the buffer is not a valid pose-data packet (wrong magic,
/// wrong type, or undecodable payload).
fn parse_pose_packet(data: &[u8]) -> Option<Vec<MarkerPose>> {
    let parsed = parse_packet(data)?;
    if parsed.pkt_type != PKT_TYPE_POSE_DATA {
        return None;
    }
    rmp_serde::from_slice::<Vec<MarkerPose>>(&data[PACKET_SIZE..]).ok()
}

#[cfg(test)]
#[path = "posestream_tests.rs"]
mod tests;
