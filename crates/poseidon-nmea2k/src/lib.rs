//! NMEA 2000 PGN decoder for marine engine and environmental sensors.
//!
//! NMEA 2000 shares the CAN 2.0B physical layer with J1939 but defines its
//! own set of Parameter Group Numbers oriented toward navigation and vessel
//! monitoring.

use poseidon_can::CanFrame;
use thiserror::Error;

/// Well-known NMEA 2000 PGNs relevant to engine diagnostics.
pub mod pgn {
    /// Engine Parameters, Rapid Update — RPM, trim, tilt.
    pub const ENGINE_PARAMS_RAPID: u32 = 127488;
    /// Engine Parameters, Dynamic — oil pressure, temps, hours.
    pub const ENGINE_PARAMS_DYNAMIC: u32 = 127489;
    /// Temperature — generic temperature source instances.
    pub const TEMPERATURE: u32 = 130312;
}

/// Errors from NMEA 2000 message decoding.
#[derive(Debug, Error)]
pub enum Nmea2kError {
    #[error("payload too short for PGN {pgn}: need {expected} bytes, got {actual}")]
    PayloadTooShort { pgn: u32, expected: usize, actual: usize },
    #[error("reserved field has unexpected value in PGN {0}")]
    ReservedField(u32),
    #[error("unknown temperature source instance {0}")]
    UnknownTempSource(u8),
}

/// Engine instance identifier (port, starboard, or auxiliary).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineInstance {
    /// Port-side main engine (instance 0).
    Port,
    /// Starboard-side main engine (instance 1).
    Starboard,
    /// Auxiliary / generator engine.
    Auxiliary(u8),
}

impl From<u8> for EngineInstance {
    fn from(val: u8) -> Self {
        match val {
            0 => Self::Port,
            1 => Self::Starboard,
            n => Self::Auxiliary(n),
        }
    }
}

/// Decoded PGN 127488 — Engine Parameters, Rapid Update.
#[derive(Debug, Clone)]
pub struct EngineRapid {
    pub instance: EngineInstance,
    /// Engine speed in RPM.
    pub rpm: f64,
    /// Engine boost pressure in kPa.
    pub boost_pressure_kpa: f64,
    /// Engine tilt/trim in percent.
    pub tilt_trim_pct: i8,
}

/// Decoded PGN 127489 — Engine Parameters, Dynamic.
#[derive(Debug, Clone)]
pub struct EngineDynamic {
    pub instance: EngineInstance,
    /// Oil pressure in kPa.
    pub oil_pressure_kpa: f64,
    /// Oil temperature in Kelvin.
    pub oil_temp_k: f64,
    /// Coolant temperature in Kelvin.
    pub coolant_temp_k: f64,
    /// Total engine hours.
    pub engine_hours: f64,
}

/// Decoded PGN 130312 — Temperature.
#[derive(Debug, Clone)]
pub struct Temperature {
    /// Sequence ID for correlating with other PGNs.
    pub sid: u8,
    /// Temperature source instance (0 = sea water, 1 = outside, etc.).
    pub source_instance: u8,
    /// Actual temperature in Kelvin.
    pub actual_temp_k: f64,
    /// Set (requested) temperature in Kelvin, if applicable.
    pub set_temp_k: Option<f64>,
}

/// Decode PGN 127488 — Engine Parameters, Rapid Update.
pub fn decode_engine_rapid(frame: &CanFrame) -> Result<EngineRapid, Nmea2kError> {
    let d = &frame.data;
    if d.len() < 8 {
        return Err(Nmea2kError::PayloadTooShort { pgn: 127488, expected: 8, actual: d.len() });
    }
    let instance = EngineInstance::from(d[0]);
    let rpm_raw = u16::from_le_bytes([d[1], d[2]]);
    let rpm = rpm_raw as f64 * 0.25;
    let boost_raw = u16::from_le_bytes([d[3], d[4]]);
    let boost_pressure_kpa = boost_raw as f64 * 0.1;
    let tilt_trim_pct = d[5] as i8;

    Ok(EngineRapid { instance, rpm, boost_pressure_kpa, tilt_trim_pct })
}

/// Decode PGN 130312 — Temperature.
pub fn decode_temperature(frame: &CanFrame) -> Result<Temperature, Nmea2kError> {
    let d = &frame.data;
    if d.len() < 8 {
        return Err(Nmea2kError::PayloadTooShort { pgn: 130312, expected: 8, actual: d.len() });
    }
    let sid = d[0];
    let source_instance = d[1];
    let actual_raw = u16::from_le_bytes([d[2], d[3]]);
    let actual_temp_k = actual_raw as f64 * 0.01;
    let set_raw = u16::from_le_bytes([d[4], d[5]]);
    let set_temp_k = if set_raw == 0xFFFF { None } else { Some(set_raw as f64 * 0.01) };

    Ok(Temperature { sid, source_instance, actual_temp_k, set_temp_k })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_instance_mapping() {
        assert_eq!(EngineInstance::from(0), EngineInstance::Port);
        assert_eq!(EngineInstance::from(1), EngineInstance::Starboard);
        assert!(matches!(EngineInstance::from(5), EngineInstance::Auxiliary(5)));
    }
}
