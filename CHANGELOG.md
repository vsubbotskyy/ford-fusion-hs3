# Changelog

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
