# Changelog

## Unreleased

### Changed
- Python parser `0x112`: new keys `ctr_112_init`, `ctr_112_b5`, `ctr_112_b7`. The old
  `tcu_motion`, `tcu_motion_analog`, `tcu_tick` keys are kept as deprecated aliases (removal in 0.5).
  The frame is a vehicle counter block; the B4 flag latches ~2-3 min into a drive and is not motion.
- DBC: `TCU_Presence` → `Ambient_Light` (`0x1B3` B5), `TCU_Motion` → `Ctr_112_Init`,
  `TCU_Motion_Analog` → `Ctr_112_B5`; `0x112` transmitter no longer claimed to be the TCU.
- Byte map `0x15E` mux `0x01`: B1 = hour, B4 = day of month, B5 = month (were "clock companion" /
  "clock mux const"), matching `GatewayMux::Clock`.

### Notes
- `0x1B3` frames with B1 bits[7:6] = 0 (`day_night` = None) are the BCM wake/sleep boundary frame;
  B5 reads `00` there. Consumers should not treat that as "dark".

## 0.3.0 — 2026-09-20

### Added
- `BcmStatus::courtesy_flash`: lock/unlock confirmation flash (flasher bulbs active without stalk latch), gated on `!hazards_active`.

### Fixed
- Replaced misclassified `tcu_ack` with `courtesy_flash` on `0x1B3`.

## 0.2.0 — 2026-09-20

### Breaking
- Renamed `BcmStatus::tcu_presence` to `ambient_light` on `0x1B3` B5 (`0x00` dark, `0x01` dawn/dusk, `0x05` daylight).
- Removed `BcmStatus::tcu_ack`.
- `GatewayMux::Clock` now carries `hour`, `minute`, `second`, `day`, `month` (UTC).
- `BcmStatus::turn_signal` now returns `"OFF"`, `"LEFT"`, `"RIGHT"`, or `"HAZARD"` instead of `"OFF"` / `"ON"`.

### Added
- `BcmStatus::turn_left_active`, `turn_right_active`, and `hazards_active` directional signals on `0x1B3`.
- `BcmStatus::day_night` from `0x1B3` B1 bits[7:6] (`"DAY"` / `"NIGHT"`).
- `decode_motor_coil_temp()` for `0x141` B2 (`raw - 40` °C).
