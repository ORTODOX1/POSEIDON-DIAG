<div align="center">

# POSEIDON-DIAG

### Maritime Engine Diagnostics Platform

Open-source diagnostic and condition monitoring platform for marine diesel engines.
Built for engineers who maintain ship power plants.

---

[![Research Preview](https://img.shields.io/badge/status-research_preview-orange?style=flat-square)](https://github.com/ORTODOX1/POSEIDON-DIAG)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.78%2B-000000?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Python](https://img.shields.io/badge/python-3.12%2B-3776AB?style=flat-square&logo=python&logoColor=white)](https://www.python.org/)
[![NMEA 2000](https://img.shields.io/badge/NMEA_2000-CAN_2.0B-005f87?style=flat-square)](https://www.nmea.org/)
[![J1939-76 Marine](https://img.shields.io/badge/SAE_J1939--76-Marine-1a1a2e?style=flat-square)](https://www.sae.org/standards/content/j1939/76_202407/)

</div>

---

## Overview

**POSEIDON-DIAG** is a maritime adaptation of the [DAEDALUS](https://github.com/ORTODOX1/DAEDALUS) ECU diagnostic platform. Where DAEDALUS targets heavy-duty truck ECU tuning, POSEIDON-DIAG is purpose-built for **marine diesel engine diagnostics** -- covering two-stroke and four-stroke engines from MAN B&W, Wartsila, Caterpillar Marine, Rolls-Royce/mtu, and others.

The platform communicates over **J1939-76 Marine CAN** and **NMEA 2000** networks, providing real-time parameter monitoring, fault code diagnostics, condition-based maintenance alerting, and AI-assisted anomaly detection. It is designed for use by chief engineers, marine service technicians, and classification society surveyors.

> **Research Preview** -- This project is under active development. Protocol implementations are validated against laboratory test benches. Deployment on operational vessels requires independent verification against applicable class society rules.

---

## Core Capabilities

### Marine ECU Communication
- **J1939-76 Marine CAN** read/write at 250 kbps with full PGN support
- Bidirectional parameter access for engine control units
- Safe write operations with configurable confirmation gates and rollback
- Multi-node addressing for engines, gearboxes, and auxiliary systems

### NMEA 2000 Integration
- Full decode of engine-related PGN groups (PGN 127488 -- Engine Parameters Rapid Update, PGN 127489 -- Engine Parameters Dynamic, PGN 127493 -- Transmission Parameters)
- Fuel system monitoring (PGN 127497 -- Trip Fuel Consumption)
- Cross-referencing with navigation data (speed over ground, heading, GPS position)

### Live Parameter Monitoring
- Engine RPM, exhaust gas temperature (per cylinder), fuel rack position
- Turbocharger boost pressure and speed
- Lube oil pressure and temperature
- Cooling water inlet/outlet temperatures
- Fuel pressure (common rail and injector-level where available)
- Scavenge air pressure and temperature
- Configurable alarm thresholds and watchdog triggers

### DTC Diagnostics
- J1939 SPN/FMI fault code decoding with marine-specific code tables
- Active and stored DTC enumeration
- Freeze-frame data capture at fault occurrence
- Cross-reference with OEM service bulletins (MAN CEAS, Wartsila UNIC, CAT ET)
- Exportable fault history in PDF and CSV formats

### Condition-Based Maintenance
- Trend analysis on critical parameters (cylinder pressure deviation, exhaust spread, bearing temperatures)
- Automated maintenance interval tracking against running hours
- Alert generation when parameters drift outside class-approved operating envelopes
- Integration with planned maintenance systems via REST API

### AI-Powered Anomaly Detection
- Integration with Claude and GPT models for pattern recognition in engine telemetry
- Deviation scoring against baseline engine performance profiles
- Natural language diagnostic summaries for watch engineers
- Configurable inference pipeline (local models or API-based)

### 3D Map Visualization
- Three.js-based 3D surface rendering of engine parameter maps (fuel injection timing, boost pressure, exhaust temperature)
- Interactive rotation, zoom, and cross-section views
- Side-by-side comparison of current vs. reference maps
- Export to STL for report generation

### Historical Trend Analysis
- Time-series storage with configurable retention (SQLite local, InfluxDB remote)
- Per-cylinder trend overlays for exhaust temperature balancing
- Performance degradation tracking across overhaul cycles
- Voyage-based segmentation for fuel efficiency reporting

---

## Supported Marine Engines

| Manufacturer | Series | Type | Notes |
|---|---|---|---|
| **MAN Energy Solutions** | ME-C, ME-GI, ME-B | Two-stroke | Electronic control via MAN CEAS interface |
| **Wartsila** | W31, W46F, RT-flex | Four-stroke / Two-stroke | UNIC control system integration |
| **Caterpillar Marine** | C32, 3516E | Four-stroke | CAT J1939 dialect support |
| **Rolls-Royce / mtu** | Series 4000 | Four-stroke | MDEC / ADEC electronic governors |
| **Yanmar** | 6EY, 6AYM series | Four-stroke | Standard J1939 interface |
| **Volvo Penta** | D13, IPS series | Four-stroke | EVC/NMEA 2000 gateway |

> Engine support is defined by protocol compatibility. Additional engines can be added by providing the PGN/SPN mapping tables for the target ECU.

---

## Protocol Support

| Protocol | Standard | Transport | Use Case |
|---|---|---|---|
| **J1939-76 Marine** | SAE J1939-76 | CAN 2.0B, 250 kbps | Primary engine ECU communication |
| **NMEA 2000** | IEC 61162-3 | CAN 2.0B + marine application layer | Engine data, navigation, fuel systems |
| **Modbus RTU/TCP** | IEC 61158 | RS-485 / Ethernet | Auxiliary systems (pumps, separators, alarm panels) |
| **OPC UA** | IEC 62541 | TCP/IP | Shore-side fleet management integration |
| **IEC 61162-1/2** | IEC 61162 | RS-422 serial | Legacy navigation instrument data (GPS, gyro, AIS) |

### Hardware Interface

Recommended CAN adapters for onboard use:

- **PEAK PCAN-USB Pro FD** -- dual-channel, rugged housing
- **Kvaser Leaf Light HS v2** -- lightweight, single channel
- **Actisense NGW-1** -- NMEA 2000 gateway with galvanic isolation
- **SocketCAN-compatible** Linux adapters for embedded installations

---

## Technology Stack

| Layer | Technology |
|---|---|
| **Core engine** | Rust 1.78+ (CAN drivers, protocol parsers, safety-critical logic) |
| **Desktop shell** | Tauri 2.x (lightweight, cross-platform) |
| **Frontend** | React 19, TypeScript 5.x |
| **Visualization** | Three.js (3D maps), Recharts (time-series) |
| **CAN interface** | SocketCAN (Linux), PCAN-Basic (Windows) |
| **AI integration** | Python 3.12+ (inference service), Claude API, OpenAI API |
| **Database** | SQLite (local), InfluxDB (time-series), PostgreSQL (fleet) |
| **Communication** | WebSocket (frontend-backend), gRPC (inter-service) |

---

## Architecture

```
poseidon-diag/
|
|-- crates/
|   |-- poseidon-can/          # CAN bus driver abstraction (SocketCAN, PCAN)
|   |-- poseidon-j1939/        # J1939-76 Marine protocol stack
|   |-- poseidon-nmea2k/       # NMEA 2000 PGN encoder/decoder
|   |-- poseidon-modbus/       # Modbus RTU/TCP client
|   |-- poseidon-dtc/          # DTC decoder with SPN/FMI tables
|   |-- poseidon-monitor/      # Real-time parameter aggregation
|   |-- poseidon-cbm/          # Condition-based maintenance engine
|   |-- poseidon-storage/      # Time-series persistence layer
|   |-- poseidon-opcua/        # OPC UA client for shore integration
|   `-- poseidon-safety/       # Write confirmation gates, watchdogs
|
|-- services/
|   |-- ai-inference/          # Python anomaly detection service
|   `-- fleet-sync/            # Shore-to-ship data synchronization
|
|-- src-tauri/                 # Tauri application entry point
|-- src/                       # React frontend
|   |-- components/
|   |   |-- dashboard/         # Main engine overview panels
|   |   |-- dtc/               # Fault code browser
|   |   |-- maps/              # 3D parameter map viewer
|   |   |-- trends/            # Historical trend charts
|   |   `-- shared/            # Reusable UI components
|   |-- hooks/                 # WebSocket and data hooks
|   `-- stores/                # State management
|
|-- proto/                     # gRPC service definitions
|-- config/                    # Engine profile configurations
`-- docs/                      # Technical documentation
```

Each Rust crate follows the single-responsibility principle. The CAN driver crate provides a hardware-agnostic interface; protocol crates (J1939, NMEA 2000, Modbus) consume raw frames and produce typed messages; higher-level crates handle monitoring logic and storage. No crate exceeds its bounded context.

---

## SOLAS/IMO Compliance Notes

This platform is intended as an **engineering diagnostic tool**, not as a certified safety system. The following regulatory context applies:

- **SOLAS Chapter II-1, Regulation 26** -- Steering gear: POSEIDON-DIAG does not interface with or monitor steering gear systems.
- **SOLAS Chapter II-2** -- Fire safety: The platform can monitor exhaust gas temperatures and generate alerts, but it is not a substitute for certified fire detection systems.
- **IMO MSC.1/Circ.1512** -- Guidelines on software quality assurance: The write-confirmation safety gates in `poseidon-safety` are designed with this circular in mind, but the software has not undergone formal type approval.
- **IACS UR E22** -- On-board use of computer-based systems: Operators deploying this tool should ensure it does not interfere with type-approved automation systems.
- **ISM Code** -- Any use of POSEIDON-DIAG should be documented in the vessel Safety Management System as an auxiliary diagnostic tool.

> **Disclaimer**: POSEIDON-DIAG is not type-approved by any classification society. It must not be used as a sole basis for safety-critical decisions. Always cross-reference with certified instrumentation.

---

## Maritime Safety Mechanisms

Marine engine diagnostics carry inherent risks. The following mechanisms are built into the platform:

### Write Protection
- All ECU write operations require a two-stage confirmation (software gate + physical hardware interlock recommendation)
- Write commands are logged with timestamp, operator ID, parameter address, old value, and new value
- A configurable write-lockout mode disables all write operations, limiting the tool to read-only diagnostics
- Automatic write abort if CAN bus error rate exceeds configurable threshold

### Operational Safeguards
- **Dead man switch** -- Active monitoring sessions require periodic operator acknowledgment; sessions auto-pause after configurable timeout
- **Parameter boundary enforcement** -- Write values are validated against OEM-defined min/max ranges before transmission
- **CAN bus isolation** -- Recommended deployment behind a CAN gateway with hardware-level write filtering for critical engine networks
- **Audit trail** -- All diagnostic actions are logged to an append-only local file, exportable for ISM Code documentation

### Network Considerations
- The platform is designed for **air-gapped or isolated network** deployment
- AI inference can operate with locally deployed models when satellite connectivity is unavailable
- Fleet synchronization uses store-and-forward with integrity verification

---

## Quick Start

### Prerequisites

- Rust 1.78 or later
- Node.js 20 LTS
- Python 3.12+ (for AI inference service)
- A supported CAN adapter connected to the J1939/NMEA 2000 network
- Linux: SocketCAN kernel module loaded (`sudo modprobe can`, `sudo modprobe vcan` for testing)
- Windows: PCAN-Basic driver installed

### Build

```bash
# Clone the repository
git clone https://github.com/ORTODOX1/POSEIDON-DIAG.git
cd POSEIDON-DIAG

# Build Rust backend
cargo build --release

# Install frontend dependencies
npm install

# Start the Tauri application
cargo tauri dev
```

### Virtual CAN Testing (Linux)

```bash
# Create a virtual CAN interface for development
sudo ip link add dev vcan0 type vcan
sudo ip link set up vcan0

# Run with virtual CAN
POSEIDON_CAN_INTERFACE=vcan0 cargo tauri dev
```

---

## Configuration

Engine profiles are stored in `config/engines/` as TOML files:

```toml
[engine]
manufacturer = "MAN Energy Solutions"
model = "6S50ME-C9.7"
type = "two-stroke"
cylinders = 6
rated_rpm = 127
rated_power_kw = 8900

[protocol]
bus = "j1939-76"
bitrate = 250000
source_address = 0x00

[parameters.exhaust_temp]
spn = 171
unit = "degC"
alarm_high = 450
alarm_critical = 500

[parameters.turbo_speed]
spn = 103
unit = "rpm"
alarm_high = 28000
```

---

## Roadmap

| Phase | Target | Scope |
|---|---|---|
| **0.1** | Q3 2026 | J1939-76 read-only communication, basic parameter dashboard |
| **0.2** | Q4 2026 | NMEA 2000 PGN decoding, DTC browser, SQLite storage |
| **0.3** | Q1 2027 | 3D map visualization, historical trend analysis |
| **0.4** | Q2 2027 | AI anomaly detection, condition-based maintenance alerts |
| **0.5** | Q3 2027 | Write operations with safety gates, Modbus integration |
| **0.6** | Q4 2027 | OPC UA shore integration, fleet synchronization |
| **1.0** | 2028 | Stable release, extended engine profile library |

---

## Related Projects

- **[DAEDALUS](https://github.com/ORTODOX1/DAEDALUS)** -- The original ECU diagnostic platform for heavy-duty trucks. POSEIDON-DIAG shares its architectural philosophy but is rebuilt from the ground up for maritime protocol requirements.

---

## Contributing

Contributions are welcome. Before submitting a pull request:

1. Ensure all Rust crates compile with `cargo build --release`
2. Run `cargo clippy` with no warnings
3. Run `cargo test` across all crates
4. Frontend changes must pass `npm run lint` and `npm run build`
5. Include engine profile TOML files for any new engine support

For protocol-level contributions (new PGN definitions, SPN mappings), please include a reference to the relevant SAE, IEC, or OEM documentation.

---

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.

---

## About the Author

Marine engineer with 3+ years of experience operating and maintaining ship power plants, including medium-speed and slow-speed diesel engines, auxiliary machinery, and integrated automation systems. Background in both engine room watchkeeping and planned maintenance management. This project bridges the gap between hands-on marine engineering practice and modern diagnostic software tooling.

---

<div align="center">

*Built for the engine room.*

</div>
