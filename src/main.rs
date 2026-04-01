//! POSEIDON-DIAG — Marine Engine Diagnostic Platform
//!
//! Tauri desktop application for real-time monitoring and fault diagnosis
//! of marine propulsion systems over J1939 and NMEA 2000 CAN networks.

use poseidon_monitor::{Monitor, ParameterReading};
use tracing::{info, warn};

/// Default CAN bitrate for J1939 marine networks (250 kbit/s).
const DEFAULT_BITRATE: u32 = 250_000;

/// Channel capacity for the monitor broadcast bus.
const MONITOR_CHANNEL_CAP: usize = 512;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(true)
        .init();

    info!("POSEIDON-DIAG v{} starting", env!("CARGO_PKG_VERSION"));
    info!("default CAN bitrate: {} kbit/s", DEFAULT_BITRATE / 1000);

    let monitor = Monitor::new(MONITOR_CHANNEL_CAP);
    let mut rx = monitor.subscribe();

    // Subscriber task: log every parameter update to the console.
    let _log_handle = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    info!(?event, "monitor event");
                }
                Err(e) => {
                    warn!("subscriber lagged or channel closed: {e}");
                    break;
                }
            }
        }
    });

    // TODO: Initialize CAN driver, start frame ingestion loop, launch
    //       Tauri webview for the bridge UI. For now emit a placeholder
    //       reading so the subscriber task has something to process.
    let reading = ParameterReading {
        key: "port.rpm".into(),
        value: 0.0,
        unit: "RPM",
        timestamp_ms: 0,
        source_address: 0x00,
    };
    if let Err(e) = monitor.ingest(reading).await {
        warn!("failed to ingest initial reading: {e}");
    }

    info!("POSEIDON-DIAG ready — awaiting CAN frames");

    // Keep the runtime alive until terminated.
    tokio::signal::ctrl_c().await.ok();
    info!("shutdown requested — closing CAN interfaces");
}
