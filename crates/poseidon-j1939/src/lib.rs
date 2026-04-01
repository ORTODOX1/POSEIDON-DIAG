//! J1939-76 Marine protocol implementation.
//!
//! Decodes Parameter Group Numbers (PGNs) and extracts Suspect Parameter
//! Numbers (SPNs) from CAN frames originating on marine propulsion and
//! auxiliary engine networks.

use poseidon_can::CanFrame;
use thiserror::Error;

/// Well-known PGN constants for marine diesel engine diagnostics.
pub mod pgn {
    /// Electronic Engine Controller 1 — RPM, torque, demand.
    pub const ENGINE_CONTROLLER_1: u32 = 61444;
    /// Engine Temperature 1 — coolant temp, fuel temp, intercooler.
    pub const ENGINE_TEMPERATURE_1: u32 = 65262;
    /// Engine Fluid Level/Pressure — oil pressure, coolant level, fuel pressure.
    pub const ENGINE_FLUID_PRESSURE: u32 = 65263;
    /// DM1 — Active Diagnostic Trouble Codes.
    pub const DM1_ACTIVE_DTC: u32 = 65226;
    /// DM2 — Previously Active DTCs.
    pub const DM2_PREVIOUS_DTC: u32 = 65227;
}

/// Extended PGN registry with metadata (data length, transmission rate).
#[path = "pgn.rs"]
pub mod pgn_registry;

/// Errors raised during J1939 message decoding.
#[derive(Debug, Error)]
pub enum J1939Error {
    #[error("frame too short: expected >= {expected} bytes, got {actual}")]
    FrameTooShort { expected: usize, actual: usize },
    #[error("unknown PGN {0}")]
    UnknownPgn(u32),
    #[error("SPN {spn} value out of valid range")]
    SpnOutOfRange { spn: u32 },
}

/// Decoded engine controller parameters (PGN 61444).
#[derive(Debug, Clone)]
pub struct EngineController {
    /// Source address of the ECU that produced this message.
    pub source_address: u8,
    /// Engine speed in revolutions per minute.
    pub engine_rpm: f64,
    /// Actual engine percent torque (0..125%).
    pub actual_torque_pct: f64,
    /// Driver demand engine percent torque.
    pub demand_torque_pct: f64,
}

/// Decoded engine temperature parameters (PGN 65262).
#[derive(Debug, Clone)]
pub struct EngineTemperature {
    pub source_address: u8,
    /// Engine coolant temperature in degrees Celsius.
    pub coolant_temp_c: f64,
    /// Fuel temperature in degrees Celsius.
    pub fuel_temp_c: f64,
    /// Engine oil temperature in degrees Celsius.
    pub oil_temp_c: f64,
}

/// Decoded engine fluid level/pressure parameters (PGN 65263).
#[derive(Debug, Clone)]
pub struct EngineFluid {
    pub source_address: u8,
    /// Engine oil pressure in kPa.
    pub oil_pressure_kpa: f64,
    /// Coolant pressure in kPa.
    pub coolant_pressure_kpa: f64,
    /// Fuel delivery pressure in kPa.
    pub fuel_pressure_kpa: f64,
}

/// Decode PGN 61444 — Electronic Engine Controller 1.
///
/// Byte layout per SAE J1939-71:
///   Byte 3-4: Engine Speed (0.125 RPM/bit, 0 offset)
///   Byte 1:   Actual Engine Torque (1%/bit, -125% offset)
///   Byte 2:   Driver Demand Torque (1%/bit, -125% offset)
pub fn decode_engine_controller(frame: &CanFrame, sa: u8) -> Result<EngineController, J1939Error> {
    if frame.data.len() < 8 {
        return Err(J1939Error::FrameTooShort { expected: 8, actual: frame.data.len() });
    }
    let rpm_raw = u16::from_le_bytes([frame.data[3], frame.data[4]]);
    let engine_rpm = rpm_raw as f64 * 0.125;
    let actual_torque_pct = frame.data[2] as f64 - 125.0;
    let demand_torque_pct = frame.data[1] as f64 - 125.0;

    Ok(EngineController { source_address: sa, engine_rpm, actual_torque_pct, demand_torque_pct })
}

/// Decode PGN 65262 — Engine Temperature 1.
pub fn decode_engine_temperature(frame: &CanFrame, sa: u8) -> Result<EngineTemperature, J1939Error> {
    if frame.data.len() < 8 {
        return Err(J1939Error::FrameTooShort { expected: 8, actual: frame.data.len() });
    }
    let coolant_temp_c = frame.data[0] as f64 - 40.0;
    let fuel_temp_c = frame.data[1] as f64 - 40.0;
    let oil_temp_c = u16::from_le_bytes([frame.data[2], frame.data[3]]) as f64 * 0.03125 - 273.0;

    Ok(EngineTemperature { source_address: sa, coolant_temp_c, fuel_temp_c, oil_temp_c })
}

/// Decode PGN 65263 — Engine Fluid Level/Pressure.
pub fn decode_engine_fluid(frame: &CanFrame, sa: u8) -> Result<EngineFluid, J1939Error> {
    if frame.data.len() < 8 {
        return Err(J1939Error::FrameTooShort { expected: 8, actual: frame.data.len() });
    }
    let oil_pressure_kpa = frame.data[3] as f64 * 4.0;
    let coolant_pressure_kpa = frame.data[4] as f64 * 2.0;
    let fuel_pressure_kpa = frame.data[5] as f64 * 4.0;

    Ok(EngineFluid { source_address: sa, oil_pressure_kpa, coolant_pressure_kpa, fuel_pressure_kpa })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_rpm_1500() {
        let frame = CanFrame {
            id: 0x0CF00400,
            data: vec![0x00, 0xFD, 0xFD, 0xE0, 0x2E, 0x00, 0xFF, 0xFF],
            is_extended: true,
            timestamp_us: 0,
        };
        let ec = decode_engine_controller(&frame, 0x00).unwrap();
        assert!((ec.engine_rpm - 1500.0).abs() < 1.0);
    }
}
