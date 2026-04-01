//! PGN registry for J1939-76 Marine parameter groups.
//!
//! Provides a lookup table of well-known Parameter Group Numbers used in
//! marine diesel engine diagnostics. Each entry carries the PGN number,
//! human-readable name, expected data length, and default transmission rate.

use std::collections::HashMap;

/// Metadata for a single Parameter Group Number.
#[derive(Debug, Clone)]
pub struct PgnInfo {
    /// Numeric PGN identifier.
    pub pgn: u32,
    /// Human-readable name of the parameter group.
    pub name: &'static str,
    /// Expected payload length in bytes.
    pub data_length: usize,
    /// Typical transmission rate in milliseconds (0 = on-request only).
    pub transmission_rate_ms: u32,
}

/// Build the default registry of marine-relevant PGNs.
///
/// The returned map is keyed by PGN number for O(1) lookup during frame
/// dispatching.
pub fn default_registry() -> HashMap<u32, PgnInfo> {
    let entries: Vec<PgnInfo> = vec![
        PgnInfo { pgn: 61444, name: "Electronic Engine Controller 1",          data_length: 8, transmission_rate_ms: 10  },
        PgnInfo { pgn: 65262, name: "Engine Temperature 1",                    data_length: 8, transmission_rate_ms: 1000 },
        PgnInfo { pgn: 65263, name: "Engine Fluid Level/Pressure",             data_length: 8, transmission_rate_ms: 500  },
        PgnInfo { pgn: 65226, name: "DM1 — Active Diagnostic Trouble Codes",   data_length: 8, transmission_rate_ms: 1000 },
        PgnInfo { pgn: 65227, name: "DM2 — Previously Active DTCs",            data_length: 8, transmission_rate_ms: 0    },
        PgnInfo { pgn: 65270, name: "Inlet/Exhaust Conditions 1",              data_length: 8, transmission_rate_ms: 500  },
        PgnInfo { pgn: 65271, name: "Vehicle Electrical Power 1",              data_length: 8, transmission_rate_ms: 1000 },
        PgnInfo { pgn: 65253, name: "Engine Hours / Revolutions",              data_length: 8, transmission_rate_ms: 1000 },
        PgnInfo { pgn: 65276, name: "Dash Display",                            data_length: 8, transmission_rate_ms: 1000 },
        PgnInfo { pgn: 65030, name: "Marine Control Information",              data_length: 8, transmission_rate_ms: 100  },
        PgnInfo { pgn: 65028, name: "Marine Propulsion Drive Status",          data_length: 8, transmission_rate_ms: 100  },
        PgnInfo { pgn: 65031, name: "Marine Generator Set Status",             data_length: 8, transmission_rate_ms: 500  },
    ];

    entries.into_iter().map(|info| (info.pgn, info)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_engine_controller() {
        let reg = default_registry();
        let info = reg.get(&61444).expect("EEC1 must be in registry");
        assert_eq!(info.data_length, 8);
        assert_eq!(info.transmission_rate_ms, 10);
    }

    #[test]
    fn registry_contains_marine_pgns() {
        let reg = default_registry();
        assert!(reg.contains_key(&65030), "Marine Control Information missing");
        assert!(reg.contains_key(&65028), "Marine Propulsion Drive Status missing");
    }
}
