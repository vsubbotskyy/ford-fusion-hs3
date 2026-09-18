# Reverse Engineering Methodology & Evidence Standards

> "Never publish or name a signal until empirical evidence supports it; actuation is not the verdict."

This repository documents an evidence-backed protocol for the Ford Fusion Hybrid (CD4) HS-CAN4 bus at the factory TCU connector. Every signal formula, state enum, and command schedule here was derived through structured empirical methods, not by copying third-party DBCs or assuming platform conventions.

---

## 1. The Labelled Capture Rule

For discrete states and boolean flags (doors, hood, trunk, exterior lights):

1. **One action per recording**: Start capture software while the vehicle is stationary and quiet. Trigger exactly one physical action (e.g., open left rear door, toggle front fog lamp), hold for 3–5 seconds, restore to resting state, and terminate capture.
2. **Delta isolation**: Diff the capture against a baseline resting capture. Any candidate bit must:
   - Transition synchronously with the real-world action.
   - Remain stable when unrelated actions occur.
   - Invert reliably across multiple independent test sessions.
3. **Negative tests**: If a signal is suspected to exist on this tap (e.g., window position, sunroof, turn signal direction), run dedicated tests. If zero bits toggle during window movement across multiple logs, the signal is formally cataloged as **Not on this tap** (MS-CAN body network).

---

## 2. Paired CAN + OBD-II Analog Calibration

For continuous physical telemetry (vehicle speed, engine RPM, coolant temperature, fuel tank percentage, high-voltage battery SOC):

1. **Simultaneous capture**: Log HS-CAN4 frames via hardware analyzer while concurrently recording calibrated diagnostic OBD-II PIDs using a dedicated scan tool (e.g., FORScan / Car Scanner ELM327).
2. **Temporal alignment**: Use a high-frequency, unambiguous analog signal (such as vehicle speed from `0x107`) to cross-correlate and eliminate time skew between asynchronous logging clocks.
3. **Statistical fitting**: Apply linear regression over thousands of synchronized sample points:
   $$\text{Physical Value} = \text{Raw Value} \times \text{Scale} + \text{Offset}$$
4. **Acceptance criteria**:
   - Pearson correlation coefficient $r \ge 0.998$.
   - Mean Absolute Error (MAE) within sensor quantization limits.
   - Byte boundaries verified against endianness and bitmask boundaries.

### Correlation Benchmarks Achieved

| Metric | CAN ID | Ground Truth Source | Pearson $r$ | Observed Error / Note |
|---|:---:|---|:---:|---|
| **Vehicle Speed** | `0x107` | OBD-II PID 0x0D | **0.99995** | $\pm 0.1\text{ km/h}$ precision |
| **Engine RPM** | `0x103` | OBD-II PID 0x0C | **0.9980** | Identifies EV mode at raw `0xE000` |
| **Coolant Temp** | `0x104` | OBD-II PID 0x05 | **0.9994** | Linear scale: $\text{raw} - 60^\circ\text{C}$ |
| **Fuel Level %** | `0x174` | OBD-II PID 0x2F | **0.9998** | Rejects clamp values 0 / 1000 |
| **Trip Distance** | `0x113` | Instrument cluster | **1.0000** | $0.1\text{ km/LSB}$ |
| **Distance to Empty** | `0x118` | Instrument cluster | **1.0000** | Exact match to dash display |

---

## 3. The False Friend Pitfall: Bus Topology & Naming Clashes

A common failure in automotive reverse-engineering is copying signal names from public CAN databases (e.g., powertrain HS1 DBCs) onto gatewayed subsidiary buses.

### Case Study: `0x1B3` vs HS1 `BodyInfo_3`
- On Ford HS1, `0x3B3` is frequently named `BodyInfo_3_FD1` and contains turn signal side, ignition switch positions, and dimmer values.
- On this vehicle's HS-CAN4 tap, `0x1B3` carries door ajar and fog light flags in identical bit positions, but the turn signal bits occupy entirely different locations:
  - Stalk activation is present on Byte 1 Bit 0.
  - Flasher bulb phase is on Byte 1 Bit 1.
  - **Turn signal side (Left vs Right) does NOT exist on `0x1B3`**.
- Attempting to force the HS1 bitfield onto `0x1B3` resulted in false turn-signal side detection. The rule: **Each bus tap must be verified independently from ground truth.**

---

## 4. Actuation is Not the Verdict

When commanding vehicle actuators:
- Transmitting a CAN frame without seeing an error frame does not mean the vehicle executed the command.
- Hearing actuator solenoids click does not prove successful protocol state transition (e.g., auto-relock timers can mask failures).
- The **only valid verification** is inspecting the BCM's internal state broadcast:
  - Door status: `0x1B3` Byte 0 Bit 2 (`0` = Locked, `1` = Unlocked).
  - Remote start: `0x147` countdown timer decrementing and `0x142` ICE running state.

---

## 5. Hardware & Electrical Guidelines

When attaching custom hardware to the factory TCU harness:

1. **Bus Termination**:
   - The HS-CAN4 bus is terminated at its two end nodes: the Gateway Module (GWM) and the factory TCU.
   - Any diagnostic tool or transceiver tapping into this link is an **unterminated stub**.
   - **Do not enable a 120 $\Omega$ termination resistor** on your transceiver (e.g., desolder or disable resistor `R2` on common SN65HVD230 breakout boards). Adding an extra termination drops bus impedance to ~40 $\Omega$, degrading signal integrity.

2. **Listen-Only Sniffing**:
   - Always initialize the CAN controller in **Listen-Only mode** during bring-up and capture.
   - Operating in normal mode with an incorrect bitrate or sample-point configuration causes the controller to transmit dominant error frames, corrupting active vehicle communication.
