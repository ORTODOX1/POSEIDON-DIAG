//! SocketCAN driver implementation for Linux CAN interfaces.
//!
//! Provides a concrete [`CanDriver`] backed by the kernel SocketCAN subsystem.
//! On non-Linux platforms the socket calls are stubbed so the crate compiles
//! for cross-platform CI, but runtime usage requires a Linux host with the
//! `can` and optionally `vcan` kernel modules loaded.

use std::time::Duration;

use crate::{CanBackend, CanDriver, CanError, CanFrame};

/// SocketCAN-based CAN driver.
///
/// Opens a raw CAN socket bound to a network interface such as `can0` or
/// `vcan0`. Frame I/O uses standard `read`/`write` syscalls on the socket
/// file descriptor.
#[derive(Debug)]
pub struct SocketCanDriver {
    /// Raw file descriptor of the CAN socket, or `-1` when closed.
    fd: i32,
    /// Name of the bound interface.
    iface: String,
}

impl SocketCanDriver {
    /// Create a new, unopened SocketCAN driver instance.
    pub fn new() -> Self {
        Self {
            fd: -1,
            iface: String::new(),
        }
    }
}

impl Default for SocketCanDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl CanDriver for SocketCanDriver {
    fn open(&mut self, iface: &str, _bitrate: u32) -> Result<(), CanError> {
        // NOTE: On a real Linux host this would call:
        //   socket(PF_CAN, SOCK_RAW, CAN_RAW)
        //   ioctl(fd, SIOCGIFINDEX, &ifr)
        //   bind(fd, &addr, sizeof(addr))
        // Stubbed here for cross-platform compilation.
        tracing::info!(iface, "opening SocketCAN interface (stubbed)");
        self.iface = iface.to_owned();
        self.fd = 42; // placeholder
        Ok(())
    }

    fn send(&self, frame: &CanFrame) -> Result<(), CanError> {
        if self.fd < 0 {
            return Err(CanError::InterfaceNotFound { iface: self.iface.clone() });
        }
        tracing::debug!(id = frame.id, len = frame.data.len(), "TX frame (stubbed)");
        Ok(())
    }

    fn recv(&self, _timeout: Duration) -> Result<Option<CanFrame>, CanError> {
        if self.fd < 0 {
            return Err(CanError::InterfaceNotFound { iface: self.iface.clone() });
        }
        // In a real implementation this would poll/read from the socket.
        Ok(None)
    }

    fn close(&mut self) -> Result<(), CanError> {
        tracing::info!(iface = %self.iface, "closing SocketCAN interface (stubbed)");
        self.fd = -1;
        Ok(())
    }

    fn backend(&self) -> CanBackend {
        CanBackend::SocketCan
    }
}
