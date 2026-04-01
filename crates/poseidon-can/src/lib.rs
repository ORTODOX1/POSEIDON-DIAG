//! CAN bus driver abstraction layer for marine engine diagnostics.
//!
//! Provides a unified interface over SocketCAN (Linux) and PCAN (Windows)
//! hardware adapters commonly found in shipboard diagnostic equipment.

pub mod socketcan;

use std::time::Duration;
use thiserror::Error;

/// Errors originating from CAN bus operations.
#[derive(Debug, Error)]
pub enum CanError {
    #[error("CAN interface `{iface}` not found")]
    InterfaceNotFound { iface: String },
    #[error("bus-off condition detected on `{iface}`")]
    BusOff { iface: String },
    #[error("transmit timeout after {elapsed:?}")]
    TxTimeout { elapsed: Duration },
    #[error("receive buffer overflow — {dropped} frames lost")]
    RxOverflow { dropped: u32 },
    #[error("driver I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// A single CAN 2.0B / CAN-FD frame on the bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanFrame {
    /// 29-bit extended identifier (J1939 / NMEA 2000 always use extended).
    pub id: u32,
    /// Payload bytes (up to 8 for CAN 2.0B, up to 64 for CAN-FD).
    pub data: Vec<u8>,
    /// True when the frame uses the 29-bit extended identifier format.
    pub is_extended: bool,
    /// Monotonic timestamp of reception in microseconds.
    pub timestamp_us: u64,
}

/// Supported CAN adapter backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanBackend {
    /// Linux SocketCAN (vcan, can0, etc.).
    SocketCan,
    /// Peak PCAN-USB adapter via the PCAN-Basic API.
    Pcan,
}

/// Abstraction over physical CAN adapters used aboard vessels.
///
/// Implementors handle platform-specific initialization, bitrate
/// configuration, and frame I/O for a single CAN channel.
pub trait CanDriver: Send + Sync {
    /// Open the interface at the specified bitrate (typically 250 kbit/s for
    /// J1939 marine networks).
    fn open(&mut self, iface: &str, bitrate: u32) -> Result<(), CanError>;

    /// Transmit a single CAN frame. Blocks until the frame is acknowledged
    /// or the hardware timeout expires.
    fn send(&self, frame: &CanFrame) -> Result<(), CanError>;

    /// Receive the next available CAN frame. Returns `None` when the
    /// specified timeout elapses without a frame being received.
    fn recv(&self, timeout: Duration) -> Result<Option<CanFrame>, CanError>;

    /// Close the interface and release hardware resources.
    fn close(&mut self) -> Result<(), CanError>;

    /// Return the backend type of this driver instance.
    fn backend(&self) -> CanBackend;
}

/// Convenience helper: extract the 29-bit extended CAN identifier fields
/// used by J1939 and NMEA 2000.
///
/// Returns `(priority, pgn, source_address)`.
pub fn parse_extended_id(id: u32) -> (u8, u32, u8) {
    let source_address = (id & 0xFF) as u8;
    let pdu_format = ((id >> 8) & 0xFF) as u8;
    let pdu_specific = ((id >> 16) & 0xFF) as u8;
    let priority = ((id >> 26) & 0x07) as u8;

    let pgn = if pdu_format < 240 {
        // PDU1 — peer-to-peer, destination in PS field
        (pdu_format as u32) << 8
    } else {
        // PDU2 — broadcast
        ((pdu_format as u32) << 8) | (pdu_specific as u32)
    };

    (priority, pgn, source_address)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_engine_controller_id() {
        // Priority 3, PGN 61444 (0xF004), SA 0x00
        let id: u32 = 0x0CF00400;
        let (pri, pgn, sa) = parse_extended_id(id);
        assert_eq!(pri, 3);
        assert_eq!(pgn, 61444);
        assert_eq!(sa, 0x00);
    }
}
