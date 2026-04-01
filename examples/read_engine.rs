//! Example: read engine RPM and exhaust temperature from a CAN bus.
//!
//! Demonstrates how the `poseidon-can`, `poseidon-j1939`, and
//! `poseidon-monitor` crates work together to receive CAN frames, decode
//! J1939 parameters, and feed them into the real-time monitor.
//!
//! Run with:
//! ```sh
//! cargo run --example read_engine
//! ```

use std::time::Duration;

use poseidon_can::{parse_extended_id, CanDriver};
use poseidon_can::socketcan::SocketCanDriver;
use poseidon_j1939::{decode_engine_controller, decode_engine_temperature, pgn};
use poseidon_monitor::{Monitor, ParameterReading};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .init();

    // --- 1. Open the CAN interface ------------------------------------------
    let mut driver = SocketCanDriver::new();
    let iface = std::env::var("POSEIDON_CAN_INTERFACE").unwrap_or_else(|_| "vcan0".into());
    if let Err(e) = driver.open(&iface, 250_000) {
        eprintln!("failed to open CAN interface `{iface}`: {e}");
        return;
    }
    println!("listening on {iface} at 250 kbit/s");

    // --- 2. Set up the monitor ----------------------------------------------
    let monitor = Monitor::new(256);
    let mut rx = monitor.subscribe();

    // Subscriber task: print every parameter update.
    let _printer = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            println!("  => {event:?}");
        }
    });

    // --- 3. Frame reception loop --------------------------------------------
    // In a real deployment this loop runs continuously. The stubbed SocketCAN
    // driver always returns None, so we simulate two readings instead.
    println!("attempting to receive frames (stubbed driver returns None)...");

    match driver.recv(Duration::from_millis(100)) {
        Ok(Some(frame)) => {
            let (_pri, pgn_num, sa) = parse_extended_id(frame.id);
            match pgn_num {
                pgn::ENGINE_CONTROLLER_1 => {
                    if let Ok(ec) = decode_engine_controller(&frame, sa) {
                        let reading = ParameterReading {
                            key: "port.rpm".into(),
                            value: ec.engine_rpm,
                            unit: "RPM",
                            timestamp_ms: frame.timestamp_us / 1000,
                            source_address: sa,
                        };
                        let _ = monitor.ingest(reading).await;
                    }
                }
                pgn::ENGINE_TEMPERATURE_1 => {
                    if let Ok(et) = decode_engine_temperature(&frame, sa) {
                        let reading = ParameterReading {
                            key: "port.exhaust_temp".into(),
                            value: et.coolant_temp_c,
                            unit: "degC",
                            timestamp_ms: frame.timestamp_us / 1000,
                            source_address: sa,
                        };
                        let _ = monitor.ingest(reading).await;
                    }
                }
                _ => println!("unhandled PGN {pgn_num}"),
            }
        }
        Ok(None) => println!("no frame received (expected with stubbed driver)"),
        Err(e) => eprintln!("CAN recv error: {e}"),
    }

    // --- 4. Simulate readings for demonstration -----------------------------
    println!("\nsimulating engine readings:");

    let rpm_reading = ParameterReading {
        key: "port.rpm".into(),
        value: 1500.0,
        unit: "RPM",
        timestamp_ms: 1000,
        source_address: 0x00,
    };
    monitor.ingest(rpm_reading).await.ok();

    let temp_reading = ParameterReading {
        key: "port.exhaust_temp".into(),
        value: 385.0,
        unit: "degC",
        timestamp_ms: 1001,
        source_address: 0x00,
    };
    monitor.ingest(temp_reading).await.ok();

    // Give the subscriber task time to print.
    tokio::time::sleep(Duration::from_millis(100)).await;

    // --- 5. Clean up --------------------------------------------------------
    if let Err(e) = driver.close() {
        eprintln!("error closing CAN interface: {e}");
    }
    println!("\ndone.");
}
