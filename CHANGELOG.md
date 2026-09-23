# Changelog

## 0.4.2 — 2026-09-23

### Added
- `decode_gear_selector` / `GearSelector`: P/R/N/D on `0x101` B3 bits 5:4. DBC `Gear_Selector`, Python `gear_selector`.
- `decode_restraints` / `Restraints`: driver belt, passenger belt and passenger seat occupied on `0x105`
  (B1 bits 6:5, B1 bits 4:3, B2 bits 7:6; 1 = yes, 2 = no, 3 = init). DBC `Driver_Belt`, `Passenger_Belt`,
  `Passenger_Occupied`, `Restraints_Valid`; Python `driver_belt_buckled`, `passenger_belt_buckled`,
  `passenger_seat_occupied`.

### Changed
- `docs/signals.md`: removed stale "not on this tap" rows for gears and oil life.

## 0.4.1 — 2026-09-23

### Added
- `decode_oil_life`: engine oil life remaining % on `0x104` B5 bits 6:0 (the PCM's `EngOilLife_Pc_Actl`,
  re-packed by the gateway). Matches OBD `Oil Life (%)`; DBC `Engine_Oil_Life`, Python `oil_life_pct`.
- `decode_lamp_mode` / `LampMode`: exterior lamp mode on `0x147` B5 bits 7:5 (off, low beam,
  parking (tentative), DRL, DRL with left/right side off while indicating). DBC `Lamp_Mode` with value table.
- Python parser: `lamp_mode` key on `0x147`.

### Changed
- `BcmStatus::headlights_on` (`0x1B3` B1 bit 3) documented as **not** the headlamps; behaviour unchanged.

## 0.4.0 — 2026-09-22

### Added
- DBC & Byte Map: `0x15E` multiplexer `0x30` GNSS DOP precision signals (`Gnss_PDOP`, `Gnss_HDOP`, `Gnss_VDOP` with scale 0.1) and raw distance pulse counter (`Gnss_Distance_Raw`).
- Master Census: documented Gateway GNSS DOP in `docs/bus-census.md`.

### Changed
- DBC: fixed multiplexing declarations on message `0x15E` (`Mux_Index M`, `m1` for clock, `m22` for GPS, `m48` for GNSS DOP).
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
