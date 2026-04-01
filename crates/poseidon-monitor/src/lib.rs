//! Real-time engine parameter aggregator.
//!
//! Consumes decoded CAN frames from J1939 and NMEA 2000 protocol stacks,
//! maintains a live snapshot of propulsion parameters, and distributes
//! updates to registered subscribers (UI, logging, alarm subsystems).

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};

/// Monitoring subsystem errors.
#[derive(Debug, Error)]
pub enum MonitorError {
    #[error("engine instance `{0}` not registered")]
    UnknownEngine(String),
    #[error("subscriber channel closed")]
    ChannelClosed,
    #[error("parameter `{key}` stale — last update {age_ms} ms ago")]
    StaleParameter { key: String, age_ms: u64 },
}

/// A single engine parameter reading with metadata.
#[derive(Debug, Clone)]
pub struct ParameterReading {
    /// Machine-readable key (e.g. "port.rpm", "stbd.oil_pressure_kpa").
    pub key: String,
    /// Decoded value in the parameter's native unit.
    pub value: f64,
    /// Unit label for display.
    pub unit: &'static str,
    /// Monotonic timestamp (milliseconds since monitor start).
    pub timestamp_ms: u64,
    /// Source address of the ECU that produced this reading.
    pub source_address: u8,
}

/// Events pushed to subscribers.
#[derive(Debug, Clone)]
pub enum MonitorEvent {
    /// A parameter was updated with a new reading.
    ParameterUpdate(ParameterReading),
    /// An active DTC was detected or cleared.
    FaultChange { spn: u32, fmi: u8, active: bool },
    /// Communication with an ECU was lost (no frames for timeout period).
    EcuTimeout { source_address: u8, silent_ms: u64 },
}

/// Live snapshot of all monitored engine parameters.
///
/// Thread-safe; multiple readers can query the latest values while the
/// ingestion loop updates them concurrently.
#[derive(Debug, Clone)]
pub struct EngineSnapshot {
    values: Arc<RwLock<HashMap<String, ParameterReading>>>,
}

impl EngineSnapshot {
    /// Create an empty snapshot store.
    pub fn new() -> Self {
        Self { values: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// Insert or update a parameter reading.
    pub async fn update(&self, reading: ParameterReading) {
        let mut map = self.values.write().await;
        map.insert(reading.key.clone(), reading);
    }

    /// Retrieve the latest reading for a parameter key.
    pub async fn get(&self, key: &str) -> Option<ParameterReading> {
        let map = self.values.read().await;
        map.get(key).cloned()
    }

    /// Return all current parameter readings.
    pub async fn all(&self) -> Vec<ParameterReading> {
        let map = self.values.read().await;
        map.values().cloned().collect()
    }
}

impl Default for EngineSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

/// Central monitor that aggregates readings and fans out events.
pub struct Monitor {
    snapshot: EngineSnapshot,
    tx: broadcast::Sender<MonitorEvent>,
}

impl Monitor {
    /// Create a monitor with the specified subscriber channel capacity.
    pub fn new(channel_capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(channel_capacity);
        Self { snapshot: EngineSnapshot::new(), tx }
    }

    /// Get a reference to the live snapshot for read access.
    pub fn snapshot(&self) -> &EngineSnapshot {
        &self.snapshot
    }

    /// Subscribe to real-time monitor events.
    pub fn subscribe(&self) -> broadcast::Receiver<MonitorEvent> {
        self.tx.subscribe()
    }

    /// Ingest a new parameter reading: update the snapshot and notify
    /// all active subscribers.
    pub async fn ingest(&self, reading: ParameterReading) -> Result<(), MonitorError> {
        self.snapshot.update(reading.clone()).await;
        // Silently drop if no subscribers are listening — this is expected during startup and shutdown
        let _ = self.tx.send(MonitorEvent::ParameterUpdate(reading));
        Ok(())
    }

    /// Report a fault status change to all subscribers.
    pub fn report_fault(&self, spn: u32, fmi: u8, active: bool) {
        // Silently drop if no subscribers are listening — this is expected during startup and shutdown
        let _ = self.tx.send(MonitorEvent::FaultChange { spn, fmi, active });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn snapshot_round_trip() {
        let snap = EngineSnapshot::new();
        let reading = ParameterReading {
            key: "port.rpm".into(),
            value: 1800.0,
            unit: "RPM",
            timestamp_ms: 42,
            source_address: 0x00,
        };
        snap.update(reading).await;
        let got = snap.get("port.rpm").await.unwrap();
        assert!((got.value - 1800.0).abs() < f64::EPSILON);
    }
}
