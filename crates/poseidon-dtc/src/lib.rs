//! Diagnostic Trouble Code (DTC) decoder for marine engines.
//!
//! Parses J1939 DM1 (active faults) and DM2 (previously active faults)
//! messages, maps SPN/FMI pairs to human-readable descriptions, and
//! classifies fault severity for bridge alarm integration.

use poseidon_can::CanFrame;
use thiserror::Error;

/// DTC decoding errors.
#[derive(Debug, Error)]
pub enum DtcError {
    #[error("DM message payload too short: expected >= {expected}, got {actual}")]
    PayloadTooShort { expected: usize, actual: usize },
    #[error("malformed DTC entry at byte offset {0}")]
    MalformedEntry(usize),
}

/// Severity classification aligned with IMO bridge alarm categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational — logged but no alarm raised.
    Info,
    /// Caution — amber indicator, operator attention recommended.
    Caution,
    /// Warning — audible alarm, corrective action required.
    Warning,
    /// Critical — engine shutdown imminent or occurring.
    Critical,
}

/// Failure Mode Indicator as defined in SAE J1939-73.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fmi(pub u8);

impl Fmi {
    /// Human-readable description of the failure mode.
    pub fn description(&self) -> &'static str {
        match self.0 {
            0 => "Data valid but above normal operational range (most severe)",
            1 => "Data valid but below normal operational range (most severe)",
            2 => "Data erratic, intermittent, or incorrect",
            3 => "Voltage above normal or shorted to high source",
            4 => "Voltage below normal or shorted to low source",
            5 => "Current below normal or open circuit",
            6 => "Current above normal or grounded circuit",
            7 => "Mechanical system not responding or out of adjustment",
            12 => "Bad intelligent device or component",
            31 => "Condition exists",
            _ => "Reserved or manufacturer-specific",
        }
    }
}

/// A single decoded Diagnostic Trouble Code.
#[derive(Debug, Clone)]
pub struct DiagnosticTroubleCode {
    /// Suspect Parameter Number identifying the faulty subsystem.
    pub spn: u32,
    /// Failure Mode Indicator describing the nature of the fault.
    pub fmi: Fmi,
    /// Occurrence count since the DTC was first detected.
    pub occurrence_count: u8,
    /// Source address of the ECU reporting the fault.
    pub source_address: u8,
    /// Computed severity for bridge alarm routing.
    pub severity: Severity,
}

/// Result of parsing a DM1 or DM2 message.
#[derive(Debug, Clone)]
pub struct DmMessage {
    /// Malfunction indicator lamp status.
    pub mil_active: bool,
    /// Red stop lamp status.
    pub red_stop_lamp: bool,
    /// Amber warning lamp status.
    pub amber_warning_lamp: bool,
    /// Individual fault codes contained in the message.
    pub dtcs: Vec<DiagnosticTroubleCode>,
}

/// Classify severity based on the SPN/FMI combination.
///
/// High-criticality SPNs (oil pressure, coolant temp, overspeed) map to
/// Warning or Critical; sensor faults map to Caution.
fn classify_severity(spn: u32, fmi: &Fmi) -> Severity {
    match (spn, fmi.0) {
        (100, 1) => Severity::Critical,   // oil pressure low — most severe
        (100, _) => Severity::Warning,     // oil pressure any other fault
        (110, 0) => Severity::Critical,    // coolant temp high — most severe
        (110, _) => Severity::Warning,
        (190, 0) => Severity::Critical,    // engine overspeed
        (_, 2)   => Severity::Caution,     // erratic data — sensor issue
        (_, 3..=6) => Severity::Caution,   // wiring faults
        _ => Severity::Info,
    }
}

/// Decode a DM1 or DM2 message from a raw CAN frame.
///
/// Each DTC occupies 4 bytes within the DM payload starting at byte offset 2.
/// The first two bytes carry lamp status bits.
pub fn decode_dm_message(frame: &CanFrame, sa: u8) -> Result<DmMessage, DtcError> {
    let d = &frame.data;
    if d.len() < 6 {
        return Err(DtcError::PayloadTooShort { expected: 6, actual: d.len() });
    }

    let mil_active = (d[0] & 0xC0) == 0x40;
    let red_stop_lamp = (d[0] & 0x30) == 0x10;
    let amber_warning_lamp = (d[0] & 0x0C) == 0x04;

    let dtc_bytes = &d[2..];
    let mut dtcs = Vec::new();

    for chunk in dtc_bytes.chunks(4) {
        if chunk.len() < 4 {
            break;
        }
        let spn = ((chunk[2] as u32 & 0xE0) << 11)
            | ((chunk[1] as u32) << 8)
            | (chunk[0] as u32);
        let fmi = Fmi(chunk[2] & 0x1F);
        let occurrence_count = chunk[3] & 0x7F;
        let severity = classify_severity(spn, &fmi);

        dtcs.push(DiagnosticTroubleCode {
            spn,
            fmi,
            occurrence_count,
            source_address: sa,
            severity,
        });
    }

    Ok(DmMessage { mil_active, red_stop_lamp, amber_warning_lamp, dtcs })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_ordering() {
        assert!(Severity::Critical > Severity::Warning);
        assert!(Severity::Warning > Severity::Caution);
        assert!(Severity::Caution > Severity::Info);
    }

    #[test]
    fn fmi_description_known() {
        let fmi = Fmi(3);
        assert_eq!(fmi.description(), "Voltage above normal or shorted to high source");
    }
}
