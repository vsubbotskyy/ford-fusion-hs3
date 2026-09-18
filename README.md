# HS-CAN4 at the Factory TCU Connector
## 2018–2019 Ford Fusion Hybrid (CD4 Platform)

Passive decode of the gatewayed telematics bus, plus the factory TCU’s remote-function command sequences as empirically measured on that bus.

This repository provides an **evidence-backed signal database and protocol library**, not a complete telematics product. It is intended for engineers and vehicle owners researching or replacing a Telematics Control Unit (TCU) **on hardware they own**, and for anyone requiring a verified DBC, Python parser, or embedded Rust decoder for this specific tap.

```text
                    ┌─────────────────────────────────────────┐
                    │       Vehicle (CD4 Fusion Hybrid)       │
                    │  HS1 / Powertrain     MS-CAN / Body     │
                    │         ▲                   ▲           │
                    │         │                   │           │
                    │            Gateway (GWM)                │
                    │                   │                     │
                    │                   ▼                     │
                    │         HS-CAN4 @ 500 kbit/s            │
                    │         two-node: TCU ↔ GWM             │
                    │                   │                     │
                    │            Factory TCU connector        │
                    └─────────────────────────────────────────┘
                                        │
                                        ▼
                              this project’s scope
```

- **In scope:** All 63 CAN arbitration IDs observed at the trunk TCU harness; verified physical formulas; TCU-owned transmit frames; and `0x146` remote lock / unlock / start / stop sequence schedules with factory millisecond timing.
- **Out of scope:** MS-CAN body actuators (windows, sunroof, HVAC), HS1 powertrain frames filtered by the gateway, FordPass cloud APIs, and diagnostic UDS polling of `0x7E0` (PCM) from this harness.
- **Platform Disambiguation:** This vehicle is built on Ford's **CD4** platform (CD391). It is **not** aftermarket CGEA 1.2 (which lacks a central Gateway Module) and **not** C1MCA (a FORScan compact-car electrical enumeration).

---

## Why This Exists

A factory telematics module operates on HS-CAN4 as a point-to-point leaf connected directly to the central Gateway Module (GWM). If you replace or interface with it, public powertrain DBCs fall short in two critical ways:

1. **What the gateway bridges onto this tap**: The GWM filters traffic. Infotainment and powertrain DBCs contain hundreds of IDs that never appear on this harness, while multiplexed signals like gateway GPS (`0x15E`) and VIN chunks (`0x11A`) only exist here.
2. **What the TCU transmits during remote operations**: Remote functions are not single magic frames. They are multi-second, phased command-and-hold schedules (`0x146`, `0x22A`, `0x27C`) requiring rolling challenge tokens from the BCM.

This repository provides the empirical, ground-truth answer for this vehicle family, structured so another embedded firmware (any MCU), a Python analytics script, or SavvyCAN can reuse it cleanly.

---

## Root Cause Analysis (RCA) Case Studies

Three engineering challenges encountered and resolved during reverse-engineering:

### 1. The Sleep-Token Trap (`0x100`)
- **Symptom:** Remote unlock worked immediately after driving, but failed consistently after the car sat parked overnight.
- **Root Cause:** The Body Control Module (BCM) broadcasts a 16-bit rolling token on `0x100` Bytes 0–1. When the car enters bus sleep, `0x100` continues broadcasting low-frequency sleep heartbeats where Bytes 0–1 are `00 00`. A naive decoder that saves the latest received token overwrote the valid rolling token with zeros. When woken, the BCM rejected the command frame containing token `00 00`.
- **Resolution:** Persist only the **last non-zero token**. Sleep heartbeats are explicitly filtered out.

### 2. The Command Verdict Rule (`0x1B3`)
- **Symptom:** Inconsistent lock test results during validation.
- **Root Cause:** Hearing actuators click is not protocol evidence. The CD4 BCM auto-relocks in ~25–30 seconds if no door is opened. A lock command on an already-locked car produces false positives.
- **Resolution:** The sole verdict on this tap is **`0x1B3` Byte 0 Bit 2** (`0` = locked, `1` = unlocked). Read it before TX, run the schedule, and watch the bit across the hold window. See [tcu-remote.md](docs/tcu-remote.md) §7.

### 3. Bus Identity & False-Friend Naming (`0x1B3` vs HS1 `BodyInfo_3`)
- **Symptom:** False detection of turn signal direction (left vs right).
- **Root Cause:** Public Ford HS1 DBCs document `0x3B3` (`BodyInfo_3_FD1`) carrying turn side, ignition, and lighting. On HS-CAN4, `0x1B3` shares identical bit locations for door-ajar and fog lamps, but turn signal side is absent (Byte 1 Bit 0 is stalk on/off, Byte 1 Bit 1 is flasher bulb phase).
- **Resolution:** Rejected the HS1 DBC template. Every bit was isolated via single-action labelled captures.

---

## Verified Signal Highlights

Exactly **63 distinct 11-bit CAN IDs** exist on this bus. A representative sample of verified signals:

| Signal | CAN ID | Layout | Ground Truth Correlation | Notes |
|---|:---:|---|:---:|---|
| **Vehicle Speed** | `0x107` | Bytes 0–1 BE $\times 0.01\text{ km/h}$ | $r = 0.99995$ vs OBD-II | High-rate dynamic broadcast |
| **Engine RPM / EV** | `0x103` | Bytes 2–3 BE `(raw - 0xE000) * 2` | $r = 0.9980$ vs OBD-II | `0xE000` indicates EV drive mode |
| **Coolant Temp** | `0x104` | Byte 2 `raw - 60` °C | $r = 0.9994$ vs OBD-II | Engine coolant temperature |
| **Fuel Tank Level %** | `0x174` | Bytes 1–2 BE $\times 0.1\text{ \%}$ | $r = 0.9998$ vs OBD-II | Raw 0 and 1000 rejected as invalid |
| **Total Odometer** | `0x109` | Bytes 0–2 (24-bit BE) 1 km/LSB | Verified vs cluster | Lifetime vehicle odometer |
| **Hybrid Battery SOC** | `0x108` | Byte 0 $\times 0.5\text{ \%}$ | Verified vs BECM UDS | Pack State of Charge |
| **HV Pack Current** | `0x07A` | Motorola 15-bit $\times 0.05 - 750\text{ A}$ | Empirical discharge/regen | Pack load range (−116 A to +140 A) |
| **HV Pack Voltage** | `0x07A` | 10-bit $\times 0.5\text{ V}$ | Empirical pack voltage | Fusion Hybrid pack (266 V to 304 V) |
| **Door Lock State** | `0x1B3` | Byte 0 Bit 2 (`0`=Locked, `1`=Unlocked) | BCM state broadcast | Command actuation verdict |
| **Door Ajar (FL/FR/RL/RR)** | `0x1B3` | Byte 6 & Byte 7 individual bits | Labelled captures | Single-door physical confirmation |
| **BCM Rolling Token** | `0x100` | Bytes 0–1 (16-bit BE) | Live BCM challenge | Updates every ~800 ms (ignore zeros) |
| **Vehicle VIN** | `0x11A` | Chunks 0, 1, 2 multiplexed ASCII | Exact 17-char string | Reassembled in `hs3-decode` |

### Detailed Documentation
- [Master Bus Census (`docs/bus-census.md`)](docs/bus-census.md): Complete 63-ID breakdown with OBD-II correlation data
- [Signal notes (`docs/signals.md`)](docs/signals.md): Closed `0x1B3` bits, negative results, retracted analogs
- [TCU Remote Commands (`docs/tcu-remote.md`)](docs/tcu-remote.md): Remote sequence timing schedules, token handling, and replay protection
- [Methodology (`docs/methodology.md`)](docs/methodology.md): Paired logging, statistical validation criteria, and hardware guidelines
- [Harness Scope (`docs/scope.md`)](docs/scope.md): Physical tap boundaries and CD4 platform definition

---

## Quick Start

### 1. DBC Database
Load [`dbc/ford_fusion_hybrid_2018_hs3.dbc`](dbc/ford_fusion_hybrid_2018_hs3.dbc) directly into SavvyCAN, python-can, or cantools.

### 2. Python Package & CLI
Install locally:
```bash
cd python
pip install .
```

Parse and validate a capture:
```bash
hs3-parse ../testdata/synthetic_sample.csv --dump
```

Python library usage:
```python
from hs3_decode import parse_frame

# Decode vehicle speed (0x107)
data = [0x15, 0x7C, 0xE0, 0x00, 0x00, 0x00, 0x00, 0x00]
decoded = parse_frame(0x107, data)
print(decoded["speed_kmh"])  # 55.0 km/h
```

### 3. Embedded Rust (`no_std`)
The Rust crates in `rust/` are `#![no_std]` and have zero external dependencies.

#### Passive Decoding (`hs3-decode`)
```rust
use hs3_decode::{decode_vehicle_speed, decode_bcm_status};

if let Some(kmh) = decode_vehicle_speed(&frame_data) {
    // Process vehicle speed
}
if let Some(bcm) = decode_bcm_status(&frame_data) {
    // Process door locks and lamps
}
```

#### Command Sequencing (`hs3-tcu`)
```rust
use hs3_tcu::{factory_door_sequence, Auth, DoorCmd};

let auth = Auth::new(sequence_counter, last_nonzero_token);
let schedule = factory_door_sequence(DoorCmd::Unlock, auth);

for step in schedule {
    hardware_sleep_until(start_time + step.t_ms);
    can_transceiver.transmit(step.frame.id, &step.frame.data);
}
```

Run host tests:
```bash
cd rust
cargo test --workspace
```

---

## Security & Ethical Disclosure

- **Physical Access Tap**: This protocol operates exclusively over the physical HS-CAN4 wiring at the factory TCU connector. It does not provide remote wireless access or bypass physical network boundaries.
- **Replay Protection**: The BCM enforces strict monotonic sequence counter increments. Capturing and replaying static historical frames will be rejected by the vehicle.
- **Separate Crates**: Passive signal decoding (`hs3-decode`) is strictly decoupled from command transmission (`hs3-tcu`). Integrators who only require telemetry logging can depend on `hs3-decode` with zero risk of pulling transmission logic.
- **Responsible Use**: This documentation is provided for owners repairing, replacing, or maintaining their own equipment.

---

## Engineering Context

- **Language Choice**: Rust was selected for embedded decoders to enforce memory safety and eliminate buffer overflow risks on a safety-relevant vehicle bus. Python was used for log parsing, statistical correlation, and exploratory analysis.
- **Validation & WIP Status**: Core physical analogs (speed, RPM, coolant, fuel, odometer) are statistically verified against synchronized OBD-II and cluster ground truth ($r \ge 0.998$). Discrete body states (door ajar, locks, fogs) were confirmed through isolated single-action captures. Other signals (such as auxiliary component temperatures, high-rate raw values, and anti-slosh fuel filtering) are empirical best-fits or active work-in-progress documented in [bus-census.md](docs/bus-census.md).

---

## License

This project is licensed under the [MIT License](LICENSE).
