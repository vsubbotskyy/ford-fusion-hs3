# Changelog

## 0.2.0 — 2026-09-20

Breaking. Corrects two signals on `0x1B3` that were mislabelled, and adds three.

### Breaking
- `BcmStatus::tcu_presence` → **`ambient_light`**. `0x1B3` B5 is the ambient twilight
  level (`0x00` dark / `0x01` dawn-dusk / `0x05` daylight), not a TCU presence ack.
  The old label was a capture-set confound: every TCU-unplugged capture was recorded
  after dark and every franken-modem drive was pre-dawn. `0x590` is the real
  TCU-presence probe.
- `BcmStatus::tcu_ack` removed. It was `B1 bit1 && B4 bit3` — left bulb AND right
  flasher, i.e. hazards; never an acknowledgement.
- `GatewayMux::Clock` now carries `hour`, `minute`, `second`, `day`, `month`
  (was `minute`, `second` only) and is documented as **UTC**.
- `BcmStatus::turn_signal` now returns `"OFF"` / `"LEFT"` / `"RIGHT"` / `"HAZARD"`
  instead of `"OFF"` / `"ON"`.

### Added
- `BcmStatus::turn_left_active`, `turn_right_active`, `hazards_active`.
  Turn side is on `0x1B3` after all: LEFT = B1 bit 0 latch with B1 bit 1 / B6 bit 6
  flashing, RIGHT = B7 bit 6 latch with B7 bit 7 / B4 bit 3 flashing. Verified by
  integrating GPS track heading over turn episodes: 16 real turns, perfect
  separation, zero contradictions. The earlier "side is not on this frame"
  conclusion came from defining *signal active* as B1 bit 0, which only asserts on
  a left turn.
- `BcmStatus::day_night` from `0x1B3` B1 bits[7:6] (`1` day, `2` night).
- `decode_motor_coil_temp()` for `0x141` B2 (`raw - 40` °C). r=0.999, MAE 0.22 °C
  against the paired OBD `Motor Coil Temp` over a 23→66 °C warm-up (134 samples).

### Notes
- `HAZARD` is **inferred**: no frame in any of the 24 captures has both latches set.
- `0x10C` B6 remains unidentified. It is not a climate setpoint and not cabin air
  temperature (r=0.055 vs OBD `Temp Inside Car` over a 43-minute drive).
- `0x174` B6:B7 is one 16-bit monotonic counter, not two signals; it tracks neither
  distance nor fuel.
