//! Minimal example: discover Enpose tracker devices on the local
//! network and log each one to the terminal.
//!
//! Run with: `cargo run --example example`

use enpose_api::DeviceDiscovery;

fn main() -> std::io::Result<()> {
    let devices = DeviceDiscovery::new().discover()?;

    if devices.is_empty() {
        println!("No Enpose devices found on the local network.");
        return Ok(());
    }

    println!("Found {} device(s):", devices.len());
    for device in &devices {
        let status = if device.compatible {
            "compatible"
        } else {
            "INCOMPATIBLE (protocol version mismatch)"
        };
        println!(
            "  - {} (serial {}): {}",
            device.ip, device.serial, status,
        );
    }
    Ok(())
}
