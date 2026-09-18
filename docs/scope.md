# HS-CAN4 Tap Scope & Platform Boundaries

> **Vehicle**: 2018–2019 Ford Fusion Hybrid (CD4.1 / CD391, HF35 eCVT)  
> **Bus**: HS-CAN4 (HS3) @ 500 kbit/s  
> **Physical Location**: Factory TCU connector (trunk harness)

---

## 1. Network Topology

The telematics bus on this vehicle is a dedicated, point-to-point physical CAN link connecting only two electronic control units:

1. **TCU (Telematics Control Unit)**: Located in the trunk.
2. **GWM (Gateway Module)**: Located behind the instrument cluster.

```text
┌────────────────────────────────────────────────────────┐
│               Vehicle Internal Networks                │
│                                                        │
│  HS1 / Powertrain      MS-CAN / Body      Infotainment │
│       │                      │                 │       │
│       └──────────────┬───────┴─────────────────┘       │
│                      │                                 │
│             [Gateway Module (GWM)]                     │
│                      │                                 │
│                      │  HS-CAN4 @ 500 kbit/s           │
│                      │  (Two-Node Point-to-Point)      │
│                      ▼                                 │
│             [Factory TCU Connector]                    │
└────────────────────────────────────────────────────────┘
```

The GWM bridges a curated subset of powertrain and body telemetry onto HS-CAN4 to service the telematics module, and accepts remote command requests (`0x146`) from the TCU to forward to the appropriate body control or powertrain controllers.

---

## 2. Platform Disambiguation: CD4 vs CGEA 1.2 vs C1MCA

Clear architectural naming prevents substantial reverse-engineering errors:

- **CD4 Platform**: The 2017+ Ford Fusion Hybrid is built on Ford's global CD4 platform (CD391). It features a centralized Gateway Module (GWM) that isolates external interfaces and routes inter-bus traffic.
- **Not CGEA 1.2**: Earlier Ford platforms (e.g., pre-2017 vehicles without a dedicated GWM) utilized CGEA 1.2 electrical architectures where modules shared common body/powertrain buses without a central firewall. Assumptions from CGEA 1.2 do not apply to this harness.
- **Not C1MCA / C1CMA**: Diagnostic tools such as FORScan frequently categorize electrical variants under legacy compact-car enums (Focus/C-Max/Escape). The vehicle platform is CD4.

---

## 3. In Scope (What Lives on This Tap)

The 63 distinct 11-bit CAN IDs present on this bus provide:

- **Powertrain Dynamics**: High-frequency vehicle speed (`0x107`), engine RPM and EV drive state (`0x103`), coolant temperature (`0x104`), accelerator demand (`0x103`), brake pedal demand and switch (`0x106`, `0x101`), longitudinal acceleration (`0x106`).
- **High-Voltage Battery Telemetry**: Pack current and voltage (`0x07A`), state of charge (`0x108`, `0x10F`), hybrid drive mode (`0x105`), motor angle resolver (`0x110`).
- **Fuel & Distance Metrics**: Total odometer (`0x109`), trip distance and trip fuel used (`0x113`), lifetime and trip ICE distance (`0x118`), distance to empty (`0x118`), fuel tank level percentage (`0x174`), cluster average economy (`0x153`).
- **Body & Chassis States**: Individual tire pressures (`0x1B5`), steering wheel angle (`0x175`), door ajar status for FL, FR, RL, RR, hood, and trunk (`0x1B3`), door lock state (`0x1B3`), exterior lighting master (`0x1B3`), front and rear fog lights (`0x1B3`).
- **Gateway & Telematics Infrastructure**: Real-time vehicle clock and calendar (`0x084`), rolling BCM authentication token (`0x100`), multiplexed VIN broadcast (`0x11A`), multiplexed gateway GPS (`0x15E`), remote start timer countdown (`0x147`).
- **Remote Actuation**: Telematics command execution (`0x146`, `0x22A`, `0x27C`).

---

## 4. Out of Scope (What is NOT on This Tap)

Do not search this bus for the following signals—they reside on other vehicle buses and are **not bridged** by the GWM onto HS-CAN4:

1. **MS-CAN Body Actuators**:
   - Window position, window movement commands, and anti-pinch status (reside on MS-CAN door modules).
   - Sunroof control and shade status.
   - Climate control setpoints, cabin temperature sensors, blower fan speed, and A/C compressor clutch duty cycle.
2. **Infotainment & Audio**:
   - SYNC screen menus, volume, media tracks, radio presets (reside on dedicated infotainment buses).
3. **Discrete Transmission Gears**:
   - Park vs Not-Park is available (`0x107` / `0x10A`), but distinct Reverse vs Neutral gear position is not bridged onto this link.
4. **Lighting Directional Nuances**:
   - Turn signal stalk activation and flasher bulb phase are present on `0x1B3`, but Left vs Right turn side is not available on this bus.
5. **Raw Powertrain Diagnostic Polling**:
   - UDS physical request `0x7E0` (PCM) is filtered by the GWM and cannot be polled from the TCU harness.
