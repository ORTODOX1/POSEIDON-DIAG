//! Write protection, audit logging, and operational safeguards for marine ECU
//! diagnostics.
//!
//! Every ECU write operation on a vessel carries risk. This crate provides
//! layered safety mechanisms aligned with IMO MSC.1/Circ.1512 guidance on
//! software quality assurance for shipboard systems.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Errors raised by the safety subsystem.
#[derive(Debug, Error)]
pub enum SafetyError {
    #[error("write rejected: operator has not confirmed stage {stage}")]
    ConfirmationMissing { stage: u8 },
    #[error("parameter 0x{address:04X} value {value} outside OEM bounds [{min}, {max}]")]
    OutOfBounds { address: u16, value: f64, min: f64, max: f64 },
    #[error("dead-man switch expired: no acknowledgement for {elapsed:?}")]
    DeadManExpired { elapsed: Duration },
    #[error("write operations are globally locked")]
    WriteLocked,
}

/// Two-stage confirmation gate for ECU write operations.
///
/// Stage 1: software confirmation from the operator via the UI.
/// Stage 2: secondary confirmation (physical interlock or explicit re-confirm).
/// Both stages must be set before a write is permitted.
#[derive(Debug)]
pub struct WriteGuard {
    stage_1_confirmed: bool,
    stage_2_confirmed: bool,
    globally_locked: bool,
}

impl WriteGuard {
    /// Create a new guard with all stages unconfirmed and writes unlocked.
    pub fn new() -> Self {
        Self {
            stage_1_confirmed: false,
            stage_2_confirmed: false,
            globally_locked: false,
        }
    }

    /// Confirm the specified stage (1 or 2).
    pub fn confirm(&mut self, stage: u8) {
        match stage {
            1 => self.stage_1_confirmed = true,
            2 => self.stage_2_confirmed = true,
            _ => tracing::warn!(stage, "ignoring unknown confirmation stage"),
        }
    }

    /// Reset both confirmation stages after a write completes or is aborted.
    pub fn reset(&mut self) {
        self.stage_1_confirmed = false;
        self.stage_2_confirmed = false;
    }

    /// Enable the global write lock, preventing all write operations.
    pub fn lock_writes(&mut self) {
        self.globally_locked = true;
    }

    /// Disable the global write lock.
    pub fn unlock_writes(&mut self) {
        self.globally_locked = false;
    }

    /// Check whether a write may proceed. Returns `Ok(())` when both stages
    /// are confirmed and the global lock is not engaged.
    pub fn authorize(&self) -> Result<(), SafetyError> {
        if self.globally_locked {
            return Err(SafetyError::WriteLocked);
        }
        if !self.stage_1_confirmed {
            return Err(SafetyError::ConfirmationMissing { stage: 1 });
        }
        if !self.stage_2_confirmed {
            return Err(SafetyError::ConfirmationMissing { stage: 2 });
        }
        Ok(())
    }
}

impl Default for WriteGuard {
    fn default() -> Self {
        Self::new()
    }
}

/// A single entry in the append-only audit log.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// ISO 8601 timestamp string.
    pub timestamp: String,
    /// Operator identifier.
    pub operator: String,
    /// ECU parameter address that was modified.
    pub address: u16,
    /// Value before the write.
    pub old_value: f64,
    /// Value after the write.
    pub new_value: f64,
}

/// Append-only log of all parameter modifications for ISM Code compliance.
#[derive(Debug, Default)]
pub struct AuditLog {
    entries: Vec<AuditEntry>,
}

impl AuditLog {
    /// Create an empty audit log.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Record a parameter modification. Entries cannot be removed.
    pub fn record(&mut self, entry: AuditEntry) {
        tracing::info!(
            address = entry.address,
            old = entry.old_value,
            new = entry.new_value,
            operator = %entry.operator,
            "audit: parameter write recorded"
        );
        self.entries.push(entry);
    }

    /// Return all recorded entries (read-only).
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }
}

/// OEM-defined min/max bounds for a single parameter.
#[derive(Debug, Clone)]
pub struct Bounds {
    pub min: f64,
    pub max: f64,
}

/// Validates proposed write values against OEM-defined parameter ranges.
#[derive(Debug, Default)]
pub struct ParameterBounds {
    limits: HashMap<u16, Bounds>,
}

impl ParameterBounds {
    /// Create an empty bounds registry.
    pub fn new() -> Self {
        Self { limits: HashMap::new() }
    }

    /// Register bounds for a parameter address.
    pub fn register(&mut self, address: u16, min: f64, max: f64) {
        self.limits.insert(address, Bounds { min, max });
    }

    /// Validate a proposed value against the registered bounds. If no bounds
    /// are registered for the address the value is accepted.
    pub fn validate(&self, address: u16, value: f64) -> Result<(), SafetyError> {
        if let Some(b) = self.limits.get(&address) {
            if value < b.min || value > b.max {
                return Err(SafetyError::OutOfBounds {
                    address,
                    value,
                    min: b.min,
                    max: b.max,
                });
            }
        }
        Ok(())
    }
}

/// Requires periodic operator acknowledgement during active monitoring.
///
/// If the operator does not call [`DeadManSwitch::acknowledge`] within the
/// configured timeout, the switch expires and write operations should be
/// suspended.
#[derive(Debug)]
pub struct DeadManSwitch {
    timeout: Duration,
    last_ack: Instant,
}

impl DeadManSwitch {
    /// Create a switch with the given timeout duration.
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            last_ack: Instant::now(),
        }
    }

    /// Record an operator acknowledgement, resetting the timer.
    pub fn acknowledge(&mut self) {
        self.last_ack = Instant::now();
    }

    /// Check whether the switch has expired. Returns `Ok(())` while the
    /// operator has acknowledged within the timeout window.
    pub fn check(&self) -> Result<(), SafetyError> {
        let elapsed = self.last_ack.elapsed();
        if elapsed > self.timeout {
            Err(SafetyError::DeadManExpired { elapsed })
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_guard_requires_both_stages() {
        let mut guard = WriteGuard::new();
        assert!(guard.authorize().is_err());

        guard.confirm(1);
        assert!(guard.authorize().is_err());

        guard.confirm(2);
        assert!(guard.authorize().is_ok());

        guard.reset();
        assert!(guard.authorize().is_err());
    }

    #[test]
    fn global_lock_overrides_confirmations() {
        let mut guard = WriteGuard::new();
        guard.confirm(1);
        guard.confirm(2);
        guard.lock_writes();
        assert!(guard.authorize().is_err());
    }

    #[test]
    fn bounds_reject_out_of_range() {
        let mut bounds = ParameterBounds::new();
        bounds.register(0x1000, 0.0, 100.0);

        assert!(bounds.validate(0x1000, 50.0).is_ok());
        assert!(bounds.validate(0x1000, 150.0).is_err());
        // Unknown address passes through.
        assert!(bounds.validate(0xFFFF, 999.0).is_ok());
    }

    #[test]
    fn audit_log_is_append_only() {
        let mut log = AuditLog::new();
        assert!(log.entries().is_empty());

        log.record(AuditEntry {
            timestamp: "2026-04-01T12:00:00Z".into(),
            operator: "chief-eng".into(),
            address: 0x1000,
            old_value: 80.0,
            new_value: 85.0,
        });
        assert_eq!(log.entries().len(), 1);
    }
}
