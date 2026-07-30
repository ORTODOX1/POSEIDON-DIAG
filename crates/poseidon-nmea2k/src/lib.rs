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
    PayloadTooShort {
        pgn: u32,
        expected: usize,
        actual: usize,
    },
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
    /// Temperature instance — distinguishes multiple sensors of the same
    /// source (e.g. two engine room probes).
    pub instance: u8,
    /// Temperature source enumeration (0 = sea, 1 = outside, 2 = inside,
    /// 3 = engine room, 4 = main cabin, ...).
    pub source: u8,
    /// Actual temperature in Kelvin.
    pub actual_temp_k: f64,
    /// Set (requested) temperature in Kelvin, if applicable.
    pub set_temp_k: Option<f64>,
}

/// Decode PGN 127488 — Engine Parameters, Rapid Update.
pub fn decode_engine_rapid(frame: &CanFrame) -> Result<EngineRapid, Nmea2kError> {
    let d = &frame.data;
    if d.len() < 8 {
        return Err(Nmea2kError::PayloadTooShort {
            pgn: 127488,
            expected: 8,
            actual: d.len(),
        });
    }
    let instance = EngineInstance::from(d[0]);
    let rpm_raw = u16::from_le_bytes([d[1], d[2]]);
    let rpm = rpm_raw as f64 * 0.25;
    let boost_raw = u16::from_le_bytes([d[3], d[4]]);
    let boost_pressure_kpa = boost_raw as f64 * 0.1;
    let tilt_trim_pct = d[5] as i8;

    Ok(EngineRapid {
        instance,
        rpm,
        boost_pressure_kpa,
        tilt_trim_pct,
    })
}

/// Decode PGN 130312 — Temperature.
///
/// Field layout (single-frame, 8 bytes):
///   Field 1 (d[0]):     SID
///   Field 2 (d[1]):     Temperature Instance
///   Field 3 (d[2]):     Temperature Source
///   Field 4 (d[3-4]):   Actual Temperature (0.01 K/bit, unsigned)
///   Field 5 (d[5-6]):   Set Temperature (0.01 K/bit, unsigned)
///   Field 6 (d[7]):     Reserved
pub fn decode_temperature(frame: &CanFrame) -> Result<Temperature, Nmea2kError> {
    let d = &frame.data;
    if d.len() < 8 {
        return Err(Nmea2kError::PayloadTooShort {
            pgn: 130312,
            expected: 8,
            actual: d.len(),
        });
    }
    let sid = d[0];
    let instance = d[1];
    let source = d[2];
    let actual_raw = u16::from_le_bytes([d[3], d[4]]);
    let actual_temp_k = actual_raw as f64 * 0.01;
    let set_raw = u16::from_le_bytes([d[5], d[6]]);
    let set_temp_k = if set_raw == 0xFFFF {
        None
    } else {
        Some(set_raw as f64 * 0.01)
    };

    Ok(Temperature {
        sid,
        instance,
        source,
        actual_temp_k,
        set_temp_k,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_instance_mapping() {
        assert_eq!(EngineInstance::from(0), EngineInstance::Port);
        assert_eq!(EngineInstance::from(1), EngineInstance::Starboard);
        assert!(matches!(
            EngineInstance::from(5),
            EngineInstance::Auxiliary(5)
        ));
    }

    #[test]
    fn decode_temperature_field_layout() {
        // SID 0x2A, instance 1, source 3 (engine room),
        // actual 300.15 K (raw 30015 = 0x753F), set temperature not available.
        let frame = CanFrame {
            id: 0x09F1_1200,
            data: vec![0x2A, 0x01, 0x03, 0x3F, 0x75, 0xFF, 0xFF, 0xFF],
            is_extended: true,
            timestamp_us: 0,
        };
        let t = decode_temperature(&frame).unwrap();
        assert_eq!(t.sid, 0x2A);
        assert_eq!(t.instance, 1);
        assert_eq!(t.source, 3);
        assert!((t.actual_temp_k - 300.15).abs() < 1e-9);
        assert_eq!(t.set_temp_k, None);
    }

    #[test]
    fn decode_temperature_with_set_point() {
        // Set temperature 295.15 K (raw 29515 = 0x734B) in bytes 6-7.
        let frame = CanFrame {
            id: 0x09F1_1200,
            data: vec![0x2A, 0x01, 0x03, 0x3F, 0x75, 0x4B, 0x73, 0xFF],
            is_extended: true,
            timestamp_us: 0,
        };
        let t = decode_temperature(&frame).unwrap();
        let set = t.set_temp_k.expect("set temperature must be present");
        assert!((set - 295.15).abs() < 1e-9);
    }
}
