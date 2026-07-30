<div align="center">

# POSEIDON-DIAG

### Maritime Engine Diagnostics Platform

Rust workspace for decoding marine diesel engine data off J1939 and NMEA 2000
CAN networks. Written by a marine engineer, for engine-room diagnostics work.

---

[![Research Preview](https://img.shields.io/badge/status-research_preview-orange?style=flat-square)](https://github.com/hermandoronin/POSEIDON-DIAG)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021_edition-000000?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![NMEA 2000](https://img.shields.io/badge/NMEA_2000-CAN_2.0B-005f87?style=flat-square)](https://www.nmea.org/)
[![SAE J1939](https://img.shields.io/badge/SAE_J1939-CAN_2.0B-1a1a2e?style=flat-square)](https://www.sae.org/standards/content/j1939_202208/)

</div>

---

## Overview

**POSEIDON-DIAG** is a set of Rust crates for reading and decoding marine diesel
engine telemetry from J1939 and NMEA 2000 CAN networks. The intended users are
chief engineers, marine service technicians, and anyone who needs to see what an
engine ECU is actually reporting.

> **Research Preview** — This is an early-stage codebase. It is a protocol
> decoding library plus a skeleton binary, not a finished diagnostic product.
> **The SocketCAN driver is currently a stub**: it does not open a real socket
> and never returns frames. Everything below describes what is in the repository
> today; planned work is listed separately under
> [Not implemented yet](#not-implemented-yet).

---

## What is implemented

### `poseidon-can` — CAN abstraction

- `CanFrame` / `CanError` types and a `CanDriver` trait for pluggable backends
- `parse_extended_id` — decodes the 29-bit extended identifier into
  `(priority, PGN, source address)` per SAE J1939-21, handling PDU1 vs PDU2 and
  the EDP/DP bits, so PGNs above `0xFFFF` (e.g. NMEA 2000 127488 = `0x1F200`)
  decode correctly
- `SocketCanDriver` — a **stub** implementation. The structure and call sequence
  are in place; the actual `socket(PF_CAN, ...)` / `bind` / `read` syscalls are
  not wired up, so `recv` always returns `None`.

### `poseidon-j1939` — J1939 decoding

Decoders, with the byte layout documented in the source against SAE J1939-71:

| PGN | Name | Decoded fields |
|---|---|---|
| 61444 | Electronic Engine Controller 1 | engine speed, actual torque %, demand torque % |
| 65262 | Engine Temperature 1 | coolant temp, fuel temp, oil temp |
| 65263 | Engine Fluid Level/Pressure 1 | fuel delivery pressure, oil pressure, coolant pressure |

Plus a PGN registry (`pgn_registry::default_registry`) carrying name, payload
length, and typical transmission rate for 12 PGNs, including the marine groups
65028 / 65030 / 65031. **The registry is metadata only** — entries without a
decoder above are not parsed into typed values.

### `poseidon-nmea2k` — NMEA 2000 decoding

| PGN | Name | Decoded fields |
|---|---|---|
| 127488 | Engine Parameters, Rapid Update | engine instance, RPM, boost pressure, tilt/trim |
| 130312 | Temperature | SID, instance, source, actual temperature, set temperature |

An `EngineDynamic` type for PGN 127489 is declared but has no decoder yet.

### `poseidon-dtc` — fault codes

- `decode_dm_message` — parses DM1 (active) and DM2 (previously active) payloads
  into SPN / FMI / occurrence count
- FMI descriptions per SAE J1939-73
- `Severity` classification (Info / Caution / Warning / Critical) for a small set
  of high-criticality SPNs (oil pressure, coolant temperature, overspeed)

### `poseidon-monitor` — live parameter aggregation

- `EngineSnapshot` — async `RwLock` map of the latest reading per parameter key
- `Monitor` — ingests readings and fans events out to subscribers over a Tokio
  broadcast channel

### `poseidon-safety` — write safeguards

These are the guard rails for a future write path. **No ECU write path exists
yet**, so nothing currently calls them in anger:

- `WriteGuard` — two-stage confirmation plus a global write lock
- `ParameterBounds` — min/max validation of a proposed value per parameter address
- `DeadManSwitch` — expires unless the operator acknowledges within a timeout
- `AuditLog` — append-only record of parameter modifications, held **in memory**
  (no file persistence yet)

---

## Architecture

```
POSEIDON-DIAG/
|
|-- Cargo.toml                     # workspace manifest (6 crates + binary)
|-- Cargo.lock
|
|-- src/
|   `-- main.rs                    # binary: starts the monitor, logs events
|
|-- crates/
|   |-- poseidon-can/              # frame types, driver trait, 29-bit ID parsing
|   |   `-- src/socketcan.rs       #   SocketCAN backend (stub)
|   |-- poseidon-j1939/            # J1939 decoders
|   |   `-- src/pgn.rs             #   PGN metadata registry
|   |-- poseidon-nmea2k/           # NMEA 2000 decoders
|   |-- poseidon-dtc/              # DM1/DM2, SPN/FMI, severity
|   |-- poseidon-monitor/          # parameter snapshot + pub/sub
|   `-- poseidon-safety/           # write gates, bounds, dead-man switch, audit log
|
|-- examples/
|   `-- read_engine.rs             # end-to-end wiring demo
|
`-- .github/workflows/ci.yml       # build, test, clippy, rustfmt
```

Each crate has one responsibility. `poseidon-can` is hardware-agnostic; the
protocol crates consume raw frames and produce typed values; `poseidon-monitor`
holds state and distributes updates; `poseidon-safety` is independent of the
transport entirely.

---

## Technology Stack

| Layer | Technology |
|---|---|
| **Language** | Rust, 2021 edition |
| **Async runtime** | Tokio (`poseidon-monitor`, binary) |
| **Errors** | `thiserror` |
| **Logging** | `tracing` / `tracing-subscriber` |
| **CAN backend** | SocketCAN (Linux) — interface defined, implementation stubbed |

There is no frontend, no database, and no external service in this repository.

---

## Quick Start

### Prerequisites

- A stable Rust toolchain (install via [rustup](https://rustup.rs/))

That is the whole list. No CAN hardware is needed to build, test, or run the
example, because the driver is stubbed.

### Build and test

```bash
git clone https://github.com/hermandoronin/POSEIDON-DIAG.git
cd POSEIDON-DIAG

cargo build --workspace
cargo test --workspace
```

### Run the example

Shows how the CAN, J1939, and monitor crates fit together. The driver returns no
frames, so it falls back to two simulated readings:

```bash
cargo run --example read_engine
```

Override the interface name it tries to open with `POSEIDON_CAN_INTERFACE`:

```bash
POSEIDON_CAN_INTERFACE=can0 cargo run --example read_engine
```

### Run the binary

Starts the Tokio runtime and the monitor, emits one placeholder reading, then
waits for Ctrl-C:

```bash
cargo run
```

---

## Not implemented yet

Everything in this section is a plan, not a feature. None of it is in the code.

**Transport and hardware**

- Real SocketCAN I/O (raw socket, `SIOCGIFINDEX`, bind, read/write)
- PCAN backend for Windows — the `CanBackend::Pcan` enum variant exists, the
  driver does not
- Multi-packet transport (J1939-21 TP.CM / TP.DT) for payloads over 8 bytes
- Modbus RTU/TCP for auxiliary systems
- OPC UA client for shore-side integration
- IEC 61162-1/2 serial navigation instrument data

**Decoding**

- NMEA 2000 PGN 127489 (Engine Parameters, Dynamic) and 127493, 127497
- Decoders for the marine J1939 PGNs currently present in the registry as
  metadata only
- Freeze-frame capture at fault occurrence
- Engine-specific profiles (per-manufacturer PGN/SPN mapping tables)

**Application**

- Any user interface
- ECU write operations — the `poseidon-safety` gates exist, the write path does not
- Persistence of the audit log and of time-series history
- Export of fault history to PDF or CSV
- Trend analysis and condition-based maintenance alerting
- 3D parameter map visualisation
- AI-assisted anomaly detection

---

## SOLAS/IMO Compliance Notes

This platform is intended as an **engineering diagnostic tool**, not as a
certified safety system. The following regulatory context applies:

- **SOLAS Chapter II-1, Regulation 26** — Steering gear: POSEIDON-DIAG does not
  interface with or monitor steering gear systems.
- **SOLAS Chapter II-2** — Fire safety: the platform can decode exhaust and
  coolant temperatures, but it is not a substitute for certified fire detection
  systems.
- **IMO MSC.1/Circ.1512** — Guidelines on software quality assurance: the
  write-confirmation gates in `poseidon-safety` are designed with this circular
  in mind, but the software has not undergone formal type approval.
- **IACS UR E22** — On-board use of computer-based systems: operators deploying
  this tool should ensure it does not interfere with type-approved automation
  systems.
- **ISM Code** — Any use of POSEIDON-DIAG should be documented in the vessel
  Safety Management System as an auxiliary diagnostic tool.

> **Disclaimer**: POSEIDON-DIAG is not type-approved by any classification
> society. It must not be used as a sole basis for safety-critical decisions.
> Always cross-reference with certified instrumentation.

---

## Design intent for write operations

Marine engine diagnostics carry inherent risk, so the safety layer was written
before the write path. The intended model, as encoded in `poseidon-safety`:

- A write requires **two independent confirmations**; a global lock can disable
  writes outright, reducing the tool to read-only diagnostics.
- Proposed values are validated against registered min/max bounds before any
  transmission.
- An active session requires periodic operator acknowledgement; the dead-man
  switch expires otherwise.
- Every modification is recorded with operator, address, old value, and new value.

Deployment guidance that the code cannot enforce: run behind a CAN gateway with
hardware-level write filtering on critical engine networks, and treat an isolated
network as the default.

---

## Contributing

Contributions are welcome. Before submitting a pull request:

1. `cargo build --workspace` succeeds
2. `cargo test --workspace` passes
3. `cargo clippy --workspace -- -D warnings` is clean
4. `cargo fmt --all -- --check` is clean

CI runs exactly these four steps.

For protocol-level contributions (new PGN definitions, SPN mappings), please
include a reference to the relevant SAE, IEC, or OEM documentation, and document
the byte layout in a doc comment on the decoder as the existing ones do.

---

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.

---

## About the Author

Marine engineer with 3+ years of experience operating and maintaining ship power
plants, including medium-speed and slow-speed diesel engines, auxiliary
machinery, and integrated automation systems. Background in both engine room
watchkeeping and planned maintenance management. This project bridges the gap
between hands-on marine engineering practice and modern diagnostic software
tooling.

---

<div align="center">

*Built for the engine room.*

</div>
