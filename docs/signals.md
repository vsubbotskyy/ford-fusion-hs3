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
| DRL / lighting master | B1 bit 3 | `headlights_on` | AUTO / dipped / high are **not** separate on this tap |
| Unlock | B0 bit 2 | `doors_locked` inverted | command **verdict** — see [tcu-remote.md](tcu-remote.md) §7 |
| Turn active / bulb | B1 bit 0 / bit 1 | `turn_signal_active` / `flasher_on` | **side is not here** |

Windows and sunroof: no bits toggle on this tap. Do not invent UI from HS-CAN4.

---

## Not on this tap

More TCU logs will not grow these. They live on MS-CAN, HS1, or in the cloud snapshot.

| Want | Where it actually lives |
|---|---|
| Window position / motion | MS-CAN door-status frames |
| HVAC compressor / setpoint | MS-CAN |
| Sunroof | body / MS-CAN |
| Reverse vs Neutral as distinct gears | not this bus (Park vs not-Park only: `0x107` / `0x10A`) |
| Oil life | FordPass / TCU snapshot — do not invent from HS-CAN4 |
| Full `BodyInfo_3` (key-in, ignition nibble, dimming, turn *side*) | HS1 `0x3B3` |

---

## Open / retracted analogs

These are documented so integrators do not publish them as physics.

| Topic | Status |
|---|---|
| `0x10C` B6 as motor / inverter temp | **Retracted.** Live band ~64–77; uncorrelated with OBD motor coil on a cold start. `decode_motor_temp` returns `None`. DBC comment points here. |
| `0x174` B3 bit 7 as low-fuel lamp | **Not the lamp.** Anti-slosh / gauge-commit strobe at low speed. Cluster lamp is unmapped on this tap. |
| `0x174` B4–B7 | Open: checksum / life counter / second sender. Needs fill and long-park captures, not more highway. |
| `0x141` MAP | Old B1:B2 MAP naming is suspect; B1 rolls 0–255. Do not swap names without a new labelled fit. |
| Turn *side* (L vs R) | Closed as **not** on `0x1B3`. Remaining hunt is the rest of HS-CAN4, not more `0x1B3` bits. |
| `0x110` as motor RPM / steering chart | High-rate 16-bit; do not treat raw as a dash angle. |

---

## Publishing rule

A toggling bit is not a signal name. A Ford DBC name from another bus is not evidence. Do not add a decoder field until a labelled capture or paired OBD fit supports it. Command success is `0x1B3` byte0 bit2, not sound.
