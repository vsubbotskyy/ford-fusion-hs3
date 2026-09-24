# Signal notes (HS-CAN4 at the factory TCU tap)

Formulas and rates for verified telemetry live in [bus-census.md](bus-census.md) and `rust/hs3-decode`. Remote lock / unlock / start / stop encoding lives in [tcu-remote.md](tcu-remote.md). This page is the **evidence and negative-result** companion: which bits are closed, which names are traps, and what is not on this tap.

DBC comments that say “see docs/signals.md” point here.

---

## Closed on `0x1B3` — do not re-hunt

Door, lamp, and lock bits were isolated with single-action labelled captures (one physical action per log). Do **not** paste HS1 `BodyInfo_3_FD1` (`0x3B3`) names onto this frame.

| Panel / lamp | Byte.bit | Field | Notes |
|---|---|---|---|
| FL ajar | B7 bit 5 | `door_ajar_fl` | labelled door capture |
| FR ajar | B7 bit 4 | `door_ajar_fr` | same |
| RL ajar | B6 bit 0 | `door_ajar_rl` | matches Ford `DrStatRl` bit 48 after on-car confirm |
| RR ajar | B6 bit 1 | `door_ajar_rr` | matches Ford `DrStatRr` bit 49 |
| Trunk | B0 bit 0 | `trunk_ajar` | labelled trunk capture |
| Hood | B7 bit 3 | `hood_ajar` | labelled hood capture |
| Front fog | B7 bit 0 | `front_fog_on` | labelled lights/fogs |
| Rear fog | B0 bit 1 | `rear_fog_on` | only latches with front fog |
| Lighting domain active | B1 bit 3 | `headlights_on` (legacy name) | Not the headlamps — use `0x147` B5 lamp mode below |
| Unlock | B0 bit 2 | `doors_locked` inverted | command **verdict** — see [tcu-remote.md](tcu-remote.md) §7 |
| Turn LEFT | B1 bit 0 latch; B1 bit 1 + B6 bit 6 flash | `turn_left` | stalk latch and bulb flash |
| Turn RIGHT | B7 bit 6 latch; B7 bit 7 + B4 bit 3 flash | `turn_right` | stalk latch and bulb flash |
| Ambient light | B5 | `ambient_light` | `00` dark / `01` dawn / `05` day |
| Courtesy flash | B1 bit 1 + B7 bit 7 (all four flash bits), no latch | `courtesy_flash` | lock/unlock confirmation flash |
| Day / night | B1 bits 7:6 | `day_night` | `1` day, `2` night |

Windows and sunroof: no bits toggle on this tap. Do not invent UI from HS-CAN4.

---

## Engine oil life — `0x104` B5 bits 6:0

| Bits | Meaning |
|---|---|
| B5 6:0 | Engine oil life remaining, % (PCM `EngOilLife_Pc_Actl`, re-packed by the gateway). Cross-checked against OBD. |
| B5 7 | Live/run flag, not part of the value |

Decoder: `decode_oil_life`.

## Exterior lamp mode — `0x147` B5 bits 7:5

| Raw | B5 | Mode |
|---|---|---|
| 0 | `00` | Off |
| 1 | `20` | Low beam |
| 2 | `40` | Parking (tentative) |
| 3 | `60` | DRL, right side off (right indicator active) |
| 4 | `80` | DRL, left side off (left indicator active) |
| 5 | `A0` | DRL |

Decoder: `decode_lamp_mode` → `LampMode`.

## Temperatures

| Signal | Frame | Formula | Status |
|---|---|---|---|
| HV battery temperature | `0x100` B6 | raw × 0.5 − 50 °C | verified vs OBD |
| Cabin temperature | `0x108` B0 | raw × 0.5 − 57 °C | candidate |

Decoders: `decode_hv_battery_temp`, `decode_cabin_temp`.

## Window positions — `0x108` B1:B2 (candidate)

One nibble per window: B1 high, B1 low, B2 high, B2 low. Nibble bits 3:1 = 1 (closed) … 5 (fully open); bit 0 is a separate flag. Which window each nibble is has not been confirmed. Decoder: `decode_windows` → `[Option<u8>; 4]` percent open.

## Gear selector — `0x101` B3 bits 5:4

| Raw | Gear |
|---|---|
| 0 | P |
| 1 | R |
| 2 | N |
| 3 | D |

A P→D turn of the dial passes R and N within ~0.4 s. Decoder: `decode_gear_selector` → `GearSelector`.

## Seat belts and passenger seat — `0x105`

| Field | Bits | 1 | 2 | 3 |
|---|---|---|---|---|
| Driver belt | B1 6:5 | buckled | unbuckled | init |
| Passenger belt | B1 4:3 | buckled | unbuckled | init |
| Passenger seat occupied | B2 7:6 | occupied | empty | init |

B1 bit 7 = valid (0 briefly after ignition on). Decoder: `decode_restraints` → `Restraints`.

---

## Not on this tap

More TCU logs will not grow these. They live on MS-CAN, HS1, or in the cloud snapshot.

| Want | Where it actually lives |
|---|---|
| HVAC compressor / setpoint | MS-CAN |
| Sunroof | body / MS-CAN |
| Full `BodyInfo_3` (key-in, ignition nibble) | HS1 `0x3B3` |

---

## Open / retracted analogs

These are documented so integrators do not publish them as physics.

| Topic | Status |
|---|---|
| `0x10C` B6 as motor / inverter temp | **Retracted.** Uncorrelated with OBD motor coil or cabin temperature. `decode_motor_temp` returns `None`. |
| `0x174` B3 bit 7 as low-fuel lamp | **Not the lamp.** Anti-slosh / gauge-commit strobe at low speed. Cluster lamp is unmapped on this tap. |
| `0x174` B4–B7 | Open: B6:B7 monotonic counter; tracks neither distance nor fuel. Unknown. |
| `0x141` MAP | Old B1:B2 MAP naming is suspect; B1 rolls 0–255. Do not swap names without a new labelled fit. |
| `0x110` as motor RPM / steering chart | High-rate 16-bit; do not treat raw as a dash angle. |

---

## Publishing rule

A toggling bit is not a signal name. A Ford DBC name from another bus is not evidence. Do not add a decoder field until a labelled capture or paired OBD fit supports it. Command success is `0x1B3` byte0 bit2, not sound.
