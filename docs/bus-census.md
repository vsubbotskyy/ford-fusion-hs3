# Ford Fusion Hybrid HS-CAN4 Reverse Engineering Matrix & Master Census

> **Vehicle**: 2018–2019 Ford Fusion Hybrid, **CD4** platform (CD391, HF35 eCVT). 2017+ Fusion has a Gateway Module (GWM). Note: FORScan sometimes lists C1MCA as an electrical architecture enum, but the vehicle platform is CD4; not aftermarket "CGEA 1.2" (older Fords without a separate GWM).  
> **Bus**: HS-CAN4 (also referred to as HS3) at **500 kbit/s**. Topology: two-node link, TCU ↔ GWM only. The GWM bridges a telematics slice onto this link; BCM / PCM / BECM reside on other buses behind the gateway firewall.  
> **Physical Tap**: Factory TCU trunk harness.  
> **Dataset**: 24 captured CAN sessions (>2,500,000 raw frames across city, highway, cold-start, remote start, locking, and diagnostic sessions).

---

## 1. High-Level Bus Census Summary

Across all captures, exactly **63 distinct 11-bit CAN IDs** were discovered on the HS-CAN4 bus:

| Classification Category | Count | Status | Description |
| :--- | :---: | :---: | :--- |
| **Tier 1: Parsed & Verified Core Telemetry** | **31 IDs** | Verified against paired CAN+OBD-II ground truth | Solved scalings, bitfields, and enums |
| **Tier 2: Identified Infrastructure Signals** | **5 IDs** | Exact byte layout & purpose decoded | Clock, token, telematics command path |
| **Tier 3: Observed Static, NM, or Diagnostic Frames** | **27 IDs** | Unvarying in captured sessions | Constant, empty, NM heartbeats, or diagnostics across normal driving/sleep; may activate under unobserved conditions |
| **Total Discovered CAN IDs** | **63 IDs** | 100% Accounted For | Complete Bus Census |

---

## 2. Tier 1: Parsed & Verified Telemetry Signals

These signals are decoded in `rust/hs3-decode` (pure `#![no_std]` functions), `python/hs3_decode` (byte map & parser), and the DBC file `dbc/ford_fusion_hybrid_2018_hs3.dbc`:

| Signal Name | CAN ID (Hex/Dec) | DLC | Bus Rate | Byte / Bit Location | Decoding Formula / Scaling | Verified Observed Range | Ground-Truth Correlation |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **True Total Odometer** | `0x109` (265) | 8 | ~1.6 kHz | Bytes 0–2 (24-bit BE) | `(b0 << 16) \| (b1 << 8) \| b2` | `214,070` $\rightarrow$ `214,937 km` | Dash Total km ($+1\text{ km}$ verified) |
| **Trip Distance** | `0x113` (275) | 8 | 12 Hz | Bytes 4–5 (16-bit BE) | `raw * 0.1` km | `0.0` $\rightarrow$ `16.3 km` | Trip computer ($r=1.0000$) |
| **Gasoline ICE Distance** | `0x118` (280) | 8 | 4.9 Hz | Bytes 0–2 (24-bit BE) | `raw * 0.1` km | `80,600.3` $\rightarrow$ `80,956.0 km` | ICE-only distance ($+18.2\text{ km}$ during trip) |
| **This-drive ICE distance** | `0x118` (280) | 8 | 4.9 Hz | Bytes 3–4 (16-bit BE) | `raw * 0.1` km | `0.0` $\rightarrow$ `40.6 km` | Δ matches lifetime ICE |
| **Distance to Empty (DTE)** | `0x118` (280) | 8 | 4.9 Hz | Bytes 5–6 (16-bit BE) | `raw * 0.1` km | `1021.0` $\rightarrow$ `917.6 km` | Dash DTE display ($r=1.000$) |
| **Vehicle Speed** | `0x107` (263) | 8 | ~1.8 kHz | Bytes 0–1 (16-bit BE) | `raw * 0.01` km/h | `0.0` $\rightarrow$ `130.0 km/h` | OBD Speed ($r=0.99995$) |
| **Park latch** | `0x107` (263) | 8 | ~1.8 kHz | Byte 2 | `0x60`="P", `0xE0`=not-P | `"P"`, `"D"` (D means D/R/N) | Bit 7 only; N and R are not on this tap |
| **High-Res Empower Gauge** | `0x141` (321) | 8 | 2.4 Hz | Bytes 0–1 (16-bit BE) | `raw - 10000` | Center = 10000; drops on regen, rises on power |
| **Digital Brake Switch** | `0x101` (257) | 8 | ~1.8 kHz | Byte 4, Bit 7 | `(b4 & 0x80) != 0` | `true`, `false` | Pedal contact switch |
| **Brake Pedal Demand %** | `0x106` (262) | 8 | ~1.8 kHz | Bytes 0–1 (10-bit BE) | `(((b0 & 0x03) << 8) \| b1) / 10.23` | `0.0%` $\rightarrow$ `38.3%` moving (`50%` stop) | Proportional pedal travel |
| **Accelerator Demand %** | `0x103` (259) | 8 | ~1.5 kHz | Bytes 0–1 (16-bit BE) | `((b0 - 0x80) << 8 \| b1) / 5.3` | `0.0%` $\rightarrow$ `100.0%` | OBD Throttle % ($r=0.998$) |
| **Engine RPM** | `0x103` (259) | 8 | ~1.5 kHz | Bytes 2–3 (16-bit BE) | `(raw - 0xE000) * 2.0` | `0` $\rightarrow$ `5000 RPM` (idle ~`1250`) | OBD Engine RPM ($r=0.998$) |
| **EV Mode Active** | `0x103` (259) | 8 | ~1.5 kHz | Bytes 2–3 | `raw == 0xE000` | `true`, `false` | Electric drive indicator |
| **Coolant Temperature** | `0x104` (260) | 8 | ~1.5 kHz | Byte 2 | `raw - 60.0` °C | `57.0°C` $\rightarrow$ `86.0°C` | OBD Coolant PID ($r=0.9994$) |
| **Fuel Tank Level %** | `0x174` (372) | 8 | 2.4 Hz | Bytes 1–2 (16-bit BE) | `raw * 0.1`; reject `0` / `1000` | `100.0%` $\rightarrow$ `85.9%` | OBD Fuel Level ($r=0.9998$) |
| **Steering Wheel Angle** | `0x175` (373) | 8 | ~1.7 kHz | Bytes 3–4 (16-bit BE) | `(raw - 8192.0) * 0.1` deg | `-900.0°` $\rightarrow$ `+900.0°` | SAS steering sensor |
| **Tire Pressure FL** | `0x1B5` (437) | 8 | ~1.7 kHz | Byte 1 | `raw * 0.01` bar | `2.35` $\rightarrow$ `2.42 bar` | TPMS Sensor (FL) |
| **Tire Pressure FR** | `0x1B5` (437) | 8 | ~1.7 kHz | Byte 3 | `raw * 0.01` bar | `2.35` $\rightarrow$ `2.42 bar` | TPMS Sensor (FR) |
| **Tire Pressure RL** | `0x1B5` (437) | 8 | ~1.7 kHz | Byte 5 | `raw * 0.01` bar | `2.35` $\rightarrow$ `2.42 bar` | TPMS Sensor (RL) |
| **Tire Pressure RR** | `0x1B5` (437) | 8 | ~1.7 kHz | Byte 7 | `raw * 0.01` bar | `2.35` $\rightarrow$ `2.42 bar` | TPMS Sensor (RR) |
| **Door Locks State** | `0x1B3` (435) | 8 | 10 Hz | Byte 0, Bit 2 | `(b0 & 0x04) == 0` | `true` (Locked), `false` (Unlocked) | BCM Door Lock contact (command verdict) |
| **Vehicle In Motion** | `0x1B3` (435) | 8 | 10 Hz | Byte 0, Bit 6 | `(b0 & 0x40) != 0` | `true`, `false` | Motion lockout flag |
| **Headlights On** | `0x1B3` (435) | 8 | 10 Hz | Byte 1, Bit 3 | `(b1 & 0x08) != 0` | `true`, `false` | Exterior lighting |
| **Turn signal (side)** | `0x1B3` (435) | 8 | 10 Hz | LEFT: B1 bit 0 · RIGHT: B7 bit 6 | latch asserted | `"OFF"` / `"LEFT"` / `"RIGHT"` / `"HAZARD"` | Side verified from GPS track heading over 16 turns (0 contradictions) |
| **Turn bulb flash** | `0x1B3` (435) | 8 | 10 Hz | LEFT: B1 bit 1 + B6 bit 6 · RIGHT: B7 bit 7 + B4 bit 3 | ~1.5 Hz | `true` / `false` | The two bits of a side toggle in exact lockstep |
| **Ignition State** | `0x10E` (270) | 8 | ~1.8 kHz | Byte 0 + Byte 7 | `0x17`="ON"; `0x03`="OFF" only if B7=`00` | `"ON"`, `"OFF"` | B0=`03` while B7≠0 is still ON |
| **Remote Start Timer** | `0x147` (327) | 8 | 15.2 Hz | Bytes 1–2 (16-bit BE) | `(b1 << 8) \| b2` | `900` $\rightarrow$ `0` seconds | Remote run countdown |
| **VIN Broadcast** | `0x11A` (282) | 8 | ~1.8 kHz | Multiplexed Chunks 0, 1, 2 | 3 frames concatenated | 17-char ASCII | Mux reassembly (e.g. `3FA6P0XX9YY123456`) |
| **BCM Auth Token** | `0x100` (256) | 8 | ~1.9 kHz | Bytes 0–1 (16-bit BE) | `(b0 << 8) \| b1` | Increments ~800ms | BCM rolling challenge (must ignore zeros) |
| **Ambient light level** | `0x1B3` (435) | 8 | 10 Hz | Byte 5 | raw | `0x00` dark / `0x01` dawn-dusk / `0x05` daylight | Twilight sensor. Formerly mislabelled "TCU presence" — see methodology |
| **Day / night state** | `0x1B3` (435) | 8 | 10 Hz | Byte 1, Bits 7:6 | `1` / `2` | `"DAY"` / `"NIGHT"` | Moves with Byte 5 and the B2/B3 dimming pair |
| **Motor coil temp** | `0x141` (321) | 8 | 2.4 Hz | Byte 2 | `raw - 40` | °C | r=0.999, MAE 0.22 °C vs OBD `Motor Coil Temp` (134 samples, 23→66 °C) |
| **UTC wall clock** | `0x15E` (350) | 8 | 1 Hz | mux `0x01`: B1 h, B2 m, B3 s, B4 day, B5 month | binary | UTC | `0x084` carries the same time in local zone (+2 h CEST) |
| **Hybrid battery SOC** | `0x108` (264) | 8 | ~1 Hz† | Byte 0 | `raw * 0.5` % | 75–84 % | Same 0.5 %/LSB as BECM UDS 0x0101 |
| **Ambient air temp** | `0x142` (322) | 8 | ~1 Hz† | Byte 0 | `raw - 64` °C | ~14 °C | OBD Outside Temp |
| **ICE running (HS3)** | `0x142` (322) | 8 | ~1 Hz† | Byte 2 | `0x64`=EV, `0x68`=ICE | EV vs ICE | Correlates with `0x103` RPM |
| **Gateway GPS lat** | `0x15E` mux `0x16/76/86` | 8 | ~1 Hz | Bytes 1–2 BE + B3 fine | `43.6591 + raw * 0.000256034 + (B3-128)*1e-6` | Regional coordinate | GPS R²=1.000; B3 fine MAE 6.0 m |
| **Gateway GPS lon** | `0x15E` mux `0x16/76/86` | 8 | ~1 Hz | Bytes 5–6 BE | `5.0 + raw * 0.000016` | Regional coordinate | GPS R²=1.000, MAE 2.5 m |
| **This-trip fuel used** | `0x113` (275) | 8 | ~1 Hz† | Byte 1 | `raw * 0.1` L | 0–7.4 L (8-bit max 25.5) | OBD Fuel used MAE 0.056 L, $r=0.999$ |
| **Longitudinal accel** | `0x106` (262) | 8 | ~1.8 kHz | B4 bits1-0 + B5 | `raw * 0.035 − 17.9` m/s² | Ford 10-bit | vs OBD GPS accel $r=0.85$, MAE 0.017 g |
| **Gateway clock MM:SS** | `0x15E` mux `0x01` | 8 | ~1 Hz | B2=min, B3=sec | 0–59 | matches `0x084` | $\rho=0.995$ vs vehicle clock |
| **Hybrid mode** | `0x105` (261) | 8 | ~1 Hz† | Byte 0 | `E0` parked, `E8` city, `F8` highway | discrete | vs speed / EV fraction |
| **Transaxle Park** | `0x10A` (266) | 8 | ~1 Hz† | Byte 2 | `0x39`=P, `0x08`/`0x09`=not-P | Park vs not-Park | `0x08` held through Reverse |
| **Drive-active** | `0x10E` (270) | 8 | 10 Hz | Byte 7 | `0x00` only at speed 0 | parked vs moving | vs `0x107` |
| **Trip average L/100 km** | `0x153` (339) | 8 | ~1.4 Hz | Bytes 4–5 BE | native `raw * 0.1` US mpg → `235.215 / mpg` | ~4–8 L/100 km typical | Cluster trip computer average |
| **HV pack current** | `0x07A` (122) | 8 | ~1–3 Hz | B0 bits6-0 + B1 Motorola 15-bit | `raw * 0.05 − 750` A | −116 → +140 A | pedal discharge / brake regen |
| **HV pack voltage** | `0x07A` (122) | 8 | ~1–3 Hz | B2 bits1-0 + B3 | `raw * 0.5` V | 266–304 V | Pack range (B4=326 V max, B5=152 V min) |
| **TCU motion** | `0x112` (274) | 8 | 10 Hz | Byte 4 | `0x00` low speed, `0x03` moving | 0 vs 3 | vs `0x107` |

† *Note: Frame rates marked with † were recorded at 1 Hz in subsampled logging; native bus transmission rates are higher.*

---

## 3. Tier 2: Identified Infrastructure Signals

| CAN ID | Typical Payload | Bus Rate | Decoded Meaning & Layout | Purpose |
| :---: | :--- | :---: | :--- | :--- |
| **`0x084`** | `[00, 00, day_hi, day_lo, MM, SS, HH, 00]` | 3.3 Hz | **Vehicle Clock & Calendar**<br>• Bytes 2–3: Calendar day<br>• Byte 4: Minute ($0-59$)<br>• Byte 5: Second ($0-59$)<br>• Byte 6: Hour ($0-23$) | Instrument cluster time sync |
| **`0x141`** | `[27, XX, YY, 36, 00, 1F, F0, 28]` | 2.4 Hz | **Analog cluster frame**<br>• B0: Current gauge analog<br>• B1–B2: Intake MAP | Sub-system analog indicators |
| **`0x146`** | `[CMD, 00, 00, SEQ, 00, 00, TOK_H, TOK_L]` | Event | **TCU Remote Command Frame** (Transmitted by TCU)<br>• Byte 0: Command ID (`0x0C`=Lock, `0x04`=Unlock, `0x01`=Start, `0x02`=Stop)<br>• Byte 3 (door) or Byte 5 (power): Sequence counter<br>• Bytes 6–7: Last non-zero rolling token from `0x100` | Remote actuation commands |
| **`0x109`** | `[..., B3, B4, ...]` | ~1.6 kHz | **Fuel Flow Accumulator** (Bytes 3–4)<br>• 16-bit tick counter tracking cumulative injector pulse durations | Cumulative fuel calculation |

---

## 4. Tier 3: Observed Static, Network Management, and Diagnostic Frames

> [!NOTE]
> These 27 IDs exhibited zero dynamic variance or served network/diagnostic functions across our capture dataset (normal city/highway driving, sleep, wake, and remote locking). While treated as non-telemetry for baseline logging, they may carry state under unobserved vehicle conditions (e.g. active safety interventions, DTC fault states, or specific diagnostic test modes).

### A. Static Broadcasts & Unused Slots
- `0x102`, `0x122`, `0x127`, `0x152`, `0x172`, `0x173`, `0x212`, `0x215`, `0x21E`, `0x232`, `0x267`, `0x27B`, `0x27D`, `0x28E`: Constant null frames or static flag slots.
- `0x1B4`, `0x1B8`, `0x27E`, `0x27F`, `0x28A`, `0x28B`, `0x28D`: Fixed configuration and node descriptor constants.

### B. Network Management (AUTOSAR NM)
- `0x590`: TCU network management heartbeat `[90, 00, FF, FF, FF, 00, FF, FF]`.
- `0x59E`: GWM network management sleep readiness broadcast `[9E, 00, FF, FF, FF, FF, FF, FF]`.

### C. Point-to-Point Diagnostic Addressing (UDS)
- `0x720 / 0x728`: Instrument Panel Cluster (IPC) physical request / response.
- `0x760 / 0x768`: APIM (SYNC Display) physical request / response.
- `0x7DF / 0x7E8`: Standard OBD-II functional broadcast & PCM diagnostic response.
- `0x7E2 / 0x7EA`: High-Voltage Hybrid Battery (BECM) diagnostic request / response.
- `0x246 / 0x24E`, `0x2B7 / 0x2BF`: Test capture injection artifacts.
