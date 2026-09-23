#![no_std]

// ============================================================================
// Verified Ford Fusion Hybrid (2018/2019) Passive CAN Decoders
// Derived from HS3-CAN broadcast traffic correlated against OBD-II reference
// ============================================================================

#[inline]
fn round_f32(val: f32) -> f32 {
    if val >= 0.0 {
        ((val + 0.5) as i32) as f32
    } else {
        ((val - 0.5) as i32) as f32
    }
}

#[inline]
fn round_f64(val: f64) -> f64 {
    if val >= 0.0 {
        ((val + 0.5) as i64) as f64
    } else {
        ((val - 0.5) as i64) as f64
    }
}

/// State extracted from BCM frame 0x1B3
#[derive(Debug, Clone, PartialEq)]
pub struct BcmStatus {
    pub doors_locked: bool,
    pub vehicle_in_motion: bool,
    pub turn_signal: Option<&'static str>, // "OFF" | "LEFT" | "RIGHT" | "HAZARD"
    pub turn_signal_active: bool,
    pub turn_left_active: bool,
    pub turn_right_active: bool,
    pub hazards_active: bool,
    /// Lock/unlock confirmation courtesy flash (all flasher bulbs active without stalk latch).
    pub courtesy_flash: bool,
    pub flasher_bulb_on: bool,
    /// B1 bit 3. **Not the headlamps** despite the name (it stays set with only the DRLs lit).
    /// Kept for compatibility; use [`decode_lamp_mode`] (0x147) for the exterior lamp state.
    pub headlights_on: bool,
    pub front_fog_on: bool,
    pub rear_fog_on: bool,
    pub door_ajar_fl: bool,
    pub door_ajar_fr: bool,
    pub door_ajar_rl: bool,
    pub door_ajar_rr: bool,
    pub trunk_ajar: bool,
    pub hood_ajar: bool,
    /// Ambient twilight level from B5: `0x00` dark, `0x01` dawn/dusk, `0x05` daylight.
    pub ambient_light: u8,
    /// B1 bits[7:6]: `1` = day, `2` = night.
    pub day_night: Option<&'static str>,
}

/// 0x107 Bytes 0-1 (Big-Endian, 0.01 km/h / LSB) — Verified r=0.99995
pub fn decode_vehicle_speed(data: &[u8]) -> Option<f32> {
    if data.len() < 2 { return None; }
    let raw = ((data[0] as u16) << 8) | (data[1] as u16);
    let speed = (raw as f32) * 0.01;
    if speed < 260.0 { Some(speed) } else { None }
}

/// 0x107 Byte 2: Park latch, not PRND.
/// `0x60` = Park. `0xE0` = not Park (Drive, Neutral, and Reverse all use 0xE0).
/// Across every HS3 log, B2 is only these two values; bit 7 is the only difference.
pub fn decode_gear_position(data: &[u8]) -> Option<&'static str> {
    if data.len() < 3 { return None; }
    match data[2] {
        0x60 => Some("P"),
        0xE0 => Some("D"),
        _ => None,
    }
}

/// 0x103 Bytes 2-3 (Engine RPM & EV mode / running indicator)
/// - >0xE000: ICE Running, RPM = (raw - 0xE000) * 2.0 (idle ~1250-1300 RPM)
/// - ==0xE000: EV Mode Active, ICE stopped, RPM = 0.0
/// - <0xE000: Standby / unpowered, RPM = 0.0
///
/// Returns: (rpm, is_running, ev_mode_active)
pub fn decode_engine_status(data: &[u8]) -> Option<(f32, bool, bool)> {
    if data.len() < 4 { return None; }
    let b2 = data[2];
    let b3 = data[3];
    if b2 > 0xE0 || (b2 == 0xE0 && b3 > 0) {
        let delta = (((b2 - 0xE0) as u32) << 8) | (b3 as u32);
        let rpm = (delta as f32) * 2.0;
        Some((rpm, true, false))
    } else if b2 == 0xE0 && b3 == 0 {
        Some((0.0, false, true))
    } else {
        Some((0.0, false, false))
    }
}

/// 0x101 Byte 4 Bit 7 (Driver Brake Pedal Switch: true = pressed, false = released)
pub fn decode_brake_pressed(data: &[u8]) -> Option<bool> {
    if data.len() < 5 { return None; }
    Some((data[4] & 0x80) != 0)
}

/// 0x106 Bytes 0-1 (Driver Brake Pedal Demand / Travel: 10-bit Big-Endian, 0 to 1023)
/// - Byte 0 Bits 1:0 = High bits 9:8
/// - Byte 1 = Low bits 7:0
/// - When uninitialized / sleeping: Byte 0 is 0x9F and Byte 1 is 0xFF (ignored)
///
/// Scaled as percentage (0.0% to 100.0%): raw / 10.23
pub fn decode_brake_pedal_pct(data: &[u8]) -> Option<f32> {
    if data.len() < 2 { return None; }
    let b0 = data[0];
    let b1 = data[1];
    if b0 == 0x9F && b1 == 0xFF {
        return None;
    }
    let raw = (((b0 & 0x03) as u16) << 8) | (b1 as u16);
    let pct = ((raw as f32) / 10.23).min(100.0);
    Some(round_f32(pct * 10.0) / 10.0)
}

/// 0x106 Bytes 4-5 (longitudinal acceleration, Ford 10-bit)
/// raw = ((B4 & 0x03) << 8) | B5; m/s² = raw * 0.035 - 17.9
/// Live frames have B4 bits7-2 = 0x3E (B4=F9/FA/FB). B4=0x03 is sleep/sentinel companion.
pub fn decode_longitudinal_accel(data: &[u8]) -> Option<f32> {
    if data.len() < 6 { return None; }
    let b4 = data[4];
    if (b4 >> 2) != 0x3E {
        return None;
    }
    let raw = (((b4 & 0x03) as u16) << 8) | (data[5] as u16);
    if raw >= 1020 {
        return None;
    }
    let ms2 = (raw as f32) * 0.035 - 17.9;
    Some(round_f32(ms2 * 100.0) / 100.0)
}

/// 0x106 Bytes 6-7 (12-bit analog). B6 high nibble D=live / 0=sleep (B6=0x0F iff B4=0x03).
/// raw = ((B6 & 0x0F) << 8) | B7. 0xFFE is the rest/uninit sentinel.
pub fn decode_106_raw12(data: &[u8]) -> Option<u16> {
    if data.len() < 8 { return None; }
    if data[6] == 0x0F {
        return None;
    }
    let raw = (((data[6] & 0x0F) as u16) << 8) | (data[7] as u16);
    if raw == 0xFFE {
        return None;
    }
    Some(raw)
}

/// 0x103 Bytes 0-1 (Driver Propulsion Demand Torque / Accelerator Pedal Demand)
/// - Baseline / Coasting / Stopped: 0x8000
/// - Accelerating: > 0x8000 (up to ~530 counts above baseline)
///
/// Returns estimated driver pedal demand % (0.0% to 100.0%)
pub fn decode_pedal_position(data: &[u8]) -> Option<f32> {
    if data.len() < 2 { return None; }
    let b0 = data[0];
    let b1 = data[1];
    if b0 > 0x80 || (b0 == 0x80 && b1 > 0) {
        let delta = (((b0 - 0x80) as u32) << 8) | (b1 as u32);
        // 530 counts corresponds to full observed acceleration demand (~100%)
        let pct = ((delta as f32) / 5.3).min(100.0);
        Some(round_f32(pct * 10.0) / 10.0)
    } else {
        Some(0.0)
    }
}

/// 0x104 Byte 5 bits 6:0 — engine oil life remaining, percent (the PCM's `EngOilLife_Pc_Actl`,
/// re-packed by the gateway). Bit 7 of B5 is a separate live/run flag and is masked off.
pub fn decode_oil_life(data: &[u8]) -> Option<u8> {
    if data.len() < 6 { return None; }
    let pct = data[5] & 0x7F;
    if pct <= 100 { Some(pct) } else { None }
}

/// 0x104 Byte 2 (Engine Coolant Temperature: raw - 60.0 °C) — Verified r=0.9994
pub fn decode_coolant_temp(data: &[u8]) -> Option<f32> {
    if data.len() < 3 { return None; }
    let temp_c = (data[2] as f32) - 60.0;
    if (-40.0..=140.0).contains(&temp_c) {
        Some(temp_c)
    } else {
        None
    }
}

/// 0x109 Bytes 0-2 (Total Vehicle Odometer: Big-Endian 24-bit, 1 km / LSB)
/// Verified against factory dash: 214,070 km -> 214,937 km.
pub fn decode_odometer(data: &[u8]) -> Option<f64> {
    if data.len() < 3 { return None; }
    let raw = ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | (data[2] as u32);
    if (50_000..=2_000_000).contains(&raw) {
        Some(raw as f64)
    } else {
        None
    }
}

/// 0x113 Bytes 4-5 (Trip Distance: Big-Endian 16-bit, 0.1 km / LSB) — Verified r=1.0000
/// Resets to 0.0 at trip start, increments up to 6553.5 km.
pub fn decode_trip_distance(data: &[u8]) -> Option<f32> {
    if data.len() < 6 { return None; }
    let raw = ((data[4] as u16) << 8) | (data[5] as u16);
    let dist = (raw as f32) * 0.1;
    Some(round_f32(dist * 10.0) / 10.0)
}

/// 0x113 Byte 1 (this-trip fuel used: 0.1 L / LSB, 8-bit so max 25.5 L)
/// Morning OBD Fuel used (L): MAE 0.056 L, r=0.999, max err 0.12 L.
pub fn decode_trip_fuel_l(data: &[u8]) -> Option<f32> {
    if data.len() < 2 { return None; }
    let liters = (data[1] as f32) * 0.1;
    Some(round_f32(liters * 10.0) / 10.0)
}

/// Backwards-compatible alias for decode_trip_distance as f64
pub fn decode_odometer_trip(data: &[u8]) -> Option<f64> {
    decode_trip_distance(data).map(|d| d as f64)
}

/// 0x118 Bytes 0-2 (Lifetime Gasoline Engine ICE Distance: Big-Endian 24-bit, 0.1 km / LSB)
/// Observed: 80,600.3 km -> 80,956.0 km.
pub fn decode_hybrid_ice_km(data: &[u8]) -> Option<f64> {
    if data.len() < 3 { return None; }
    let raw = ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | (data[2] as u32);
    if raw > 0 {
        let km = (raw as f64) * 0.1;
        Some(round_f64(km * 10.0) / 10.0)
    } else {
        None
    }
}

/// 0x118 Bytes 5-6 (Distance to Empty DTE: Big-Endian 16-bit, 0.1 km / LSB)
/// Observed: 1021.0 km -> 917.6 km.
pub fn decode_dte(data: &[u8]) -> Option<f32> {
    if data.len() < 7 { return None; }
    let raw_dte = ((data[5] as u16) << 8) | (data[6] as u16);
    let dte = (raw_dte as f32) * 0.1;
    Some(round_f32(dte * 10.0) / 10.0)
}

/// 0x118 Bytes 3-4 (this-drive ICE distance: Big-Endian 16-bit, 0.1 km / LSB)
/// Δ matches lifetime ICE on 0x118 B0-B2. Distinct from 0x113 trip km.
pub fn decode_trip_ice_km(data: &[u8]) -> Option<f32> {
    if data.len() < 5 { return None; }
    let raw = ((data[3] as u16) << 8) | (data[4] as u16);
    let km = (raw as f32) * 0.1;
    Some(round_f32(km * 10.0) / 10.0)
}

/// 0x118 Bytes 3-4 (this-drive ICE km) and Bytes 5-6 (Distance to Empty DTE km)
pub fn decode_trip_and_dte(data: &[u8]) -> Option<(f32, f32)> {
    let ice = decode_trip_ice_km(data)?;
    let dte = decode_dte(data)?;
    Some((ice, dte))
}

/// 0x174 Bytes 1-2 (Fuel Tank Level %: Big-Endian, 0.1% / LSB)
/// 0x035B = 859 -> 85.9%. 0x03E8 (100.0%) is an init/invalid sentinel on this car.
pub fn decode_fuel_level(data: &[u8]) -> Option<f32> {
    if data.len() < 3 { return None; }
    let raw = ((data[1] as u16) << 8) | (data[2] as u16);
    if raw == 0 || raw == 1000 {
        return None;
    }
    let pct = (raw as f32) * 0.1;
    if (0.0..=100.0).contains(&pct) {
        Some(pct)
    } else {
        None
    }
}

/// 0x174 B3 bit 7 is IPC anti-slosh / low-speed fuel-gauge commit, not the lamp.
/// See docs/can-findings.md § Open RE — 0x174. Empty-tank capture still needed.
pub fn decode_low_fuel_lamp(_data: &[u8]) -> Option<bool> {
    None
}

/// 0x109 Bytes 3-4 (Fuel flow consumption accumulator ticks)
pub fn decode_fuel_accumulator(data: &[u8]) -> Option<u32> {
    if data.len() < 5 { return None; }
    let count = ((data[3] as u32) << 8) | (data[4] as u32);
    Some(count)
}

/// 0x10E Byte 0 + Byte 7 (Vehicle Ignition State)
/// 0x17 = ON. 0x03 is OFF only when B7=0 (drive inactive).
/// Mid-drive frames can have B0=0x03 while B7 is still 0x20/0xA0.
pub fn decode_ignition_state(data: &[u8]) -> Option<&'static str> {
    let b0 = *data.first()?;
    match b0 {
        0x17 => Some("ON"),
        0x03 => {
            let b7 = *data.get(7)?;
            if b7 == 0 {
                Some("OFF")
            } else {
                Some("ON")
            }
        }
        _ => Some("STANDBY"),
    }
}

/// 0x175 Bytes 3-4 (Steering Wheel Angle: (raw - 8192) * 0.1 deg)
pub fn decode_steering_angle(data: &[u8]) -> Option<f32> {
    if data.len() < 5 { return None; }
    let raw16 = ((data[3] as u16) << 8) | (data[4] as u16);
    let angle = ((raw16 as f32) - 8192.0) * 0.1;
    if (-900.0..=900.0).contains(&angle) {
        Some(angle)
    } else {
        None
    }
}

/// 0x1B5 Bytes 1, 3, 5, 7 (Tire Pressures in kPa / 100 = bar)
/// Returns: [FL, FR, RL, RR]
pub fn decode_tire_pressures(data: &[u8]) -> Option<[f32; 4]> {
    if data.len() < 8 { return None; }
    let fl = (data[1] as f32) * 0.01;
    let fr = (data[3] as f32) * 0.01;
    let rl = (data[5] as f32) * 0.01;
    let rr = (data[7] as f32) * 0.01;
    if (0.5..5.0).contains(&fl) && (0.5..5.0).contains(&fr) && (0.5..5.0).contains(&rl) && (0.5..5.0).contains(&rr) {
        Some([fl, fr, rl, rr])
    } else {
        None
    }
}

/// 0x1B3 BCM Broadcast: locks, lighting, door ajar, turn signals, ambient light.
///
/// Turn signals (verified against GPS track heading over 16 turns, 0 contradictions):
///   LEFT  = B1 bit0 active latch; B1 bit1 and B6 bit6 flash in lockstep.
///   RIGHT = B7 bit6 active latch; B7 bit7 and B4 bit3 flash in lockstep.
///   COURTESY FLASH = all four flash bits, neither latch - the BCM lock/unlock
///     confirmation blink, and the observable ack for a door command.
///   HAZARD = both latches asserted. INFERRED, NOT OBSERVED: no frame in any
///     of the 24 captures has both latches set, so this branch is untested
///     against the vehicle. Capture a hazard-light session before relying on it.
/// Ambient light (B5) is NOT TCU presence: it tracks time of day (00 dark /
/// 01 dawn / 05 daylight), stepping 00->01 at vehicle clock 07:05:25 on the
/// 2026-09-09 dawn drive. The old label was an artefact of every TCU-unplugged
/// capture happening after dark. B1 bits[7:6] carry day/night state.
/// Front fog = B7 bit0. Rear fog = B0 bit1 (only latches with front fog).
/// Door ajar (user-confirmed; matches Ford DrStatRl/Rr on BodyInfo_3):
/// FL = B7 bit5, FR = B7 bit4, RL = B6 bit0, RR = B6 bit1.
/// Trunk ajar (trunc.csv): B0 bit0. Independent of the four door bits.
/// Hood ajar (hood.csv): B7 bit3. Independent of FL/trunk.
pub fn decode_bcm_status(data: &[u8]) -> Option<BcmStatus> {
    if data.is_empty() { return None; }
    let unlocked = (data[0] >> 2) & 1 == 1;
    let doors_locked = !unlocked;
    let vehicle_in_motion = (data[0] & 0x40) != 0;

    if data.len() < 8 {
        return Some(BcmStatus {
            doors_locked,
            vehicle_in_motion,
            turn_signal: None,
            turn_signal_active: false,
            turn_left_active: false,
            turn_right_active: false,
            hazards_active: false,
            courtesy_flash: false,
            flasher_bulb_on: false,
            headlights_on: false,
            front_fog_on: false,
            rear_fog_on: false,
            door_ajar_fl: false,
            door_ajar_fr: false,
            door_ajar_rl: false,
            door_ajar_rr: false,
            trunk_ajar: false,
            hood_ajar: false,
            ambient_light: 0xFF,
            day_night: None,
        });
    }

    let turn_left_active = data[1] & 0x01 != 0;
    let turn_right_active = (data[7] >> 6) & 0x01 != 0;
    let hazards_active = turn_left_active && turn_right_active;
    let ts_active = turn_left_active || turn_right_active;
    let left_bulb_on = (data[1] >> 1) & 0x01 != 0;
    let right_bulb_on = (data[7] >> 7) & 0x01 != 0;
    let flasher_bulb_on = left_bulb_on || right_bulb_on;
    let courtesy_flash = left_bulb_on && right_bulb_on && !hazards_active;
    let headlights_on = (data[1] >> 3) & 0x01 != 0;
    let front_fog_on = data[7] & 0x01 != 0;
    let rear_fog_on = (data[0] >> 1) & 0x01 != 0;
    let door_ajar_fl = (data[7] >> 5) & 0x01 != 0;
    let door_ajar_fr = (data[7] >> 4) & 0x01 != 0;
    let door_ajar_rl = data[6] & 0x01 != 0;
    let door_ajar_rr = (data[6] >> 1) & 0x01 != 0;
    let trunk_ajar = data[0] & 0x01 != 0;
    let hood_ajar = (data[7] >> 3) & 0x01 != 0;
    let ambient_light = data[5];
    let day_night = match (data[1] >> 6) & 0x03 {
        1 => Some("DAY"),
        2 => Some("NIGHT"),
        _ => None,
    };

    let turn_signal = Some(match (turn_left_active, turn_right_active) {
        (true, true) => "HAZARD",
        (true, false) => "LEFT",
        (false, true) => "RIGHT",
        (false, false) => "OFF",
    });

    Some(BcmStatus {
        doors_locked,
        vehicle_in_motion,
        turn_signal,
        turn_signal_active: ts_active,
        turn_left_active,
        turn_right_active,
        hazards_active,
        courtesy_flash,
        flasher_bulb_on,
        headlights_on,
        front_fog_on,
        rear_fog_on,
        door_ajar_fl,
        door_ajar_fr,
        door_ajar_rl,
        door_ajar_rr,
        trunk_ajar,
        hood_ajar,
        ambient_light,
        day_night,
    })
}

/// 0x147 Bytes 1-2 (Remote Start Countdown Timer, Big-Endian seconds)
/// 900 -> 0 seconds during remote start active run
pub fn decode_remote_start_timer(data: &[u8]) -> Option<u16> {
    if data.len() < 3 { return None; }
    let secs = ((data[1] as u16) << 8) | (data[2] as u16);
    Some(secs)
}

/// Exterior lamp mode, `0x147` B5 bits 7:5 (3-bit value).
///
/// | raw | mode |
/// |---|---|
/// | 0 | `Off` |
/// | 1 | `LowBeam` |
/// | 2 | `Parking` (tentative) |
/// | 3 | `DrlRightOff` — DRL with the right side off while the right indicator is active |
/// | 4 | `DrlLeftOff` — DRL with the left side off while the left indicator is active |
/// | 5 | `Drl` |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LampMode {
    Off,
    LowBeam,
    /// Tentative.
    Parking,
    DrlRightOff,
    DrlLeftOff,
    Drl,
    /// 6 or 7 — never observed.
    Unknown(u8),
}

impl LampMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            LampMode::Off => "OFF",
            LampMode::LowBeam => "LOW_BEAM",
            LampMode::Parking => "PARKING",
            LampMode::DrlRightOff => "DRL_RIGHT_OFF",
            LampMode::DrlLeftOff => "DRL_LEFT_OFF",
            LampMode::Drl => "DRL",
            LampMode::Unknown(_) => "UNKNOWN",
        }
    }

    /// True while the low-beam headlamps are lit.
    pub fn low_beam(&self) -> bool {
        matches!(self, LampMode::LowBeam)
    }

    /// True while daytime running lamps are active (either or both sides).
    pub fn drl(&self) -> bool {
        matches!(self, LampMode::Drl | LampMode::DrlLeftOff | LampMode::DrlRightOff)
    }
}

/// 0x147 B5 bits 7:5 — exterior lamp mode. See [`LampMode`].
pub fn decode_lamp_mode(data: &[u8]) -> Option<LampMode> {
    if data.len() < 6 { return None; }
    Some(match data[5] >> 5 {
        0 => LampMode::Off,
        1 => LampMode::LowBeam,
        2 => LampMode::Parking,
        3 => LampMode::DrlRightOff,
        4 => LampMode::DrlLeftOff,
        5 => LampMode::Drl,
        n => LampMode::Unknown(n),
    })
}

/// 0x100 Bytes 0-1 (BCM Rolling Authentication Token)
pub fn decode_bcm_token(data: &[u8]) -> Option<u16> {
    if data.len() < 2 { return None; }
    let tok = ((data[0] as u16) << 8) | (data[1] as u16);
    if tok != 0 { Some(tok) } else { None }
}

/// Multiplexed VIN assembler state
#[derive(Debug, Clone, Default)]
pub struct VinAssembler {
    chunks: [u8; 17],
    mask: u8,
}

impl VinAssembler {
    pub const fn new() -> Self {
        Self {
            chunks: [0u8; 17],
            mask: 0,
        }
    }

    /// Process a 0x11A CAN frame.
    /// Format: data[0] == 0xC1, data[1] == chunk_idx (0, 1, 2)
    /// Returns Some(vin) when all 17 characters have been assembled.
    pub fn process_frame(&mut self, data: &[u8]) -> Option<&str> {
        if data.len() < 8 || data[0] != 0xC1 {
            return None;
        }
        match data[1] {
            0x00 => {
                self.chunks[0..6].copy_from_slice(&data[2..8]);
                self.mask |= 0x01;
            }
            0x01 => {
                self.chunks[6..12].copy_from_slice(&data[2..8]);
                self.mask |= 0x02;
            }
            0x02 => {
                self.chunks[12..17].copy_from_slice(&data[2..7]);
                self.mask |= 0x04;
            }
            _ => {}
        }
        if self.mask == 0x07 {
            if let Ok(s) = core::str::from_utf8(&self.chunks) {
                if s.chars().all(|c| c.is_ascii_alphanumeric()) {
                    return Some(s);
                }
            }
        }
        None
    }

    pub fn vin(&self) -> Option<&str> {
        if self.mask == 0x07 {
            if let Ok(s) = core::str::from_utf8(&self.chunks) {
                if s.chars().all(|c| c.is_ascii_alphanumeric()) {
                    return Some(s);
                }
            }
        }
        None
    }
}

/// 0x084 Ford GlobalClock: B6 hour, B4 minute, B5 second (B2:B3 = calendar day).
pub fn decode_vehicle_clock(data: &[u8]) -> Option<(u8, u8, u8)> {
    if data.len() < 7 {
        return None;
    }
    let hh = data[6];
    let mm = data[4];
    let ss = data[5];
    if hh <= 23 && mm <= 59 && ss <= 59 {
        Some((hh, mm, ss))
    } else {
        None
    }
}

/// 0x108 Byte 0 * 0.5. SUPERSEDED — this is NOT traction SOC (r=0.26 vs OBD,
/// pinned ~78-83%). Kept for reference only; real SOC is `decode_hybrid_soc_10f`.
pub fn decode_hybrid_soc(data: &[u8]) -> Option<f32> {
    let raw = *data.first()?;
    let pct = (raw as f32) * 0.5;
    if (40.0..=100.0).contains(&pct) {
        Some(pct)
    } else {
        None
    }
}

/// 0x142 Byte 2: ICE vs EV. 0x64 = EV (ICE stopped), 0x68 = ICE running.
pub fn decode_ice_running_142(data: &[u8]) -> Option<bool> {
    if data.len() < 3 {
        return None;
    }
    match data[2] {
        0x64 => Some(false),
        0x68 => Some(true),
        _ => None,
    }
}

/// 0x142 Byte 0: ambient air temperature, °C = raw − 64.
/// Mean 13.8 °C vs OBD outside 13 °C on the 2026-09-09 morning drive.
pub fn decode_ambient_temp(data: &[u8]) -> Option<f32> {
    let raw = *data.first()?;
    let temp = (raw as f32) - 64.0;
    if (-20.0..=50.0).contains(&temp) {
        Some(temp)
    } else {
        None
    }
}

/// 0x10C Byte 6 is NOT motor / inverter temp. Live values sit at 64–77 °C
/// even with ICE coolant at 15–18 °C. OBD Motor Coil starts ~23 °C on the
/// same cold drive (r≈0). See docs/can-findings.md § Open RE — 0x10C.
pub fn decode_motor_temp(_data: &[u8]) -> Option<f32> {
    None
}

/// 0x10A Byte 2: transaxle Park vs not-Park. `0x39` = Park, `0x08`/`0x09` = not Park.
/// `0x08` stays set through Reverse. `0x38` is a rare stopped third state (not PRND).
pub fn decode_transaxle_park(data: &[u8]) -> Option<bool> {
    if data.len() < 3 {
        return None;
    }
    match data[2] {
        0x39 => Some(true),
        0x08 | 0x09 => Some(false),
        _ => None,
    }
}

/// 0x141 Byte 2: Motor Coil Temperature, `raw - 40` °C.
pub fn decode_motor_coil_temp(data: &[u8]) -> Option<f32> {
    if data.len() < 3 {
        return None;
    }
    let raw = data[2];
    if raw < 40 {
        return None;
    }
    Some(raw as f32 - 40.0)
}

/// 0x10E Byte 7: 0x00 only while speed is 0 (parked / not in active drive).
pub fn decode_drive_active(data: &[u8]) -> Option<bool> {
    if data.len() < 8 {
        return None;
    }
    Some(data[7] != 0x00)
}

/// 0x10E B4 bit5 and B5 bit2 are 1 iff B7 == 0xA0 (all captures).
pub fn decode_drive_alt(data: &[u8]) -> Option<bool> {
    if data.len() < 8 {
        return None;
    }
    Some(data[7] == 0xA0)
}

/// 0x07A Ford Battery_Traction_1: 15-bit Motorola current starting at bit 6.
/// Amps = raw * 0.05 − 750. Verified: pedal>200 → ~+45 A, brake>100 → ~−27 A.
pub fn decode_hv_current(data: &[u8]) -> Option<f32> {
    if data.len() < 2 {
        return None;
    }
    let raw = (((data[0] & 0x7F) as u16) << 8) | (data[1] as u16);
    Some((raw as f32) * 0.05 - 750.0)
}

/// 0x141 Bytes 0-1 BE: High-Resolution Empower/Charge Gauge.
/// 16-bit value where 10000 is the zero point (0 kW / 0 Amps).
/// Drops below 10000 during regen, rises above 10000 during acceleration/power.
/// Output is scaled to approximately match Amps (multiplier 0.09375).
pub fn decode_hv_current_gauge(data: &[u8]) -> Option<f32> {
    if data.len() < 2 {
        return None;
    }
    let raw = ((data[0] as u16) << 8) | (data[1] as u16);
    Some((raw as f32 - 10000.0) * 0.09375)
}

/// 0x07A Bytes 2–3: pack voltage. V = ((B2 & 3) << 8 | B3) * 0.5
/// Morning drive 266–304 V; B4*2=326 V high limit, B5*2=152 V low limit.
pub fn decode_hv_voltage(data: &[u8]) -> Option<f32> {
    if data.len() < 4 {
        return None;
    }
    let raw = (((data[2] & 0x03) as u16) << 8) | (data[3] as u16);
    let volts = (raw as f32) * 0.5;
    if (100.0..=450.0).contains(&volts) {
        Some(volts)
    } else {
        None
    }
}

/// 0x105 Byte 0: hybrid operating mode.
pub fn decode_hybrid_mode(data: &[u8]) -> Option<&'static str> {
    match data.first()? {
        0xE0 => Some("OFF"),
        0xE8 => Some("CITY"),
        0xF8 => Some("HIGHWAY"),
        _ => None,
    }
}

/// US gallons/mile → L/100 km: 235.214583 / mpg.
const US_MPG_TO_L100KM: f32 = 235.214_58;

fn trip_avg_mpg_raw(data: &[u8]) -> Option<f32> {
    if data.len() < 6 {
        return None;
    }
    let raw = ((data[4] as u16) << 8) | (data[5] as u16);
    if raw == 0 || raw >= 1000 {
        return None;
    }
    Some(raw as f32 * 0.1)
}

/// 0x153 Bytes 4–5: this-trip average, native 0.1 US mpg / LSB.
pub fn decode_trip_mpg(data: &[u8]) -> Option<f32> {
    let mpg = trip_avg_mpg_raw(data)?;
    Some(round_f32(mpg * 10.0) / 10.0)
}

/// Same `0x153` average as L/100 km. Gated to 1.5–30.
pub fn decode_trip_l100km(data: &[u8]) -> Option<f32> {
    let mpg = trip_avg_mpg_raw(data)?;
    let l100 = US_MPG_TO_L100KM / mpg;
    if !(1.5..=30.0).contains(&l100) {
        return None;
    }
    Some(round_f32(l100 * 10.0) / 10.0)
}

/// 0x10F Bytes 0–1: high-rate 16-bit BE (aliased at ~1 Hz in captures).
pub fn decode_10f_raw(data: &[u8]) -> Option<u16> {
    if data.len() < 2 {
        return None;
    }
    Some(((data[0] as u16) << 8) | (data[1] as u16))
}

/// 0x10F Bytes 0-1 (Big-Endian): HV traction battery State of Charge.
/// `SOC% = raw16 * 0.0025` (i.e. raw / 400).
/// VERIFIED against the OBD-II "Traction Battery State-of-Charge" PID over three
/// logged drives: r = 0.999, RMS 0.06 %, raw 16766..25046 -> 41.9..62.6 %.
/// This SUPERSEDES `decode_hybrid_soc` (0x108 byte0 * 0.5), which was pinned near
/// 78-83 % and uncorrelated with real SOC (r = 0.26). Cross-checked by 0x10C byte0.
pub fn decode_hybrid_soc_10f(data: &[u8]) -> Option<f32> {
    if data.len() < 2 {
        return None;
    }
    let raw = ((data[0] as u16) << 8) | (data[1] as u16);
    let pct = (raw as f32) * 0.0025;
    if (1.0..=99.0).contains(&pct) {
        Some(round_f32(pct * 10.0) / 10.0)
    } else {
        None
    }
}

/// 0x110 Bytes 1–2: electrical / rotor angle, 360° / 65536.
/// B3 bit 7 is resolver-valid (0x80). B0=B1=B2=0xFF is the asleep sentinel.
pub fn decode_motor_angle(data: &[u8]) -> Option<f32> {
    if data.len() < 3 {
        return None;
    }
    if data.len() >= 4 && (data[3] & 0x80) == 0 {
        return None;
    }
    if data[0] == 0xFF && data[1] == 0xFF && data[2] == 0xFF {
        return None;
    }
    let raw = ((data[1] as u16) << 8) | (data[2] as u16);
    Some(round_f32((raw as f32) * 360.0 / 65536.0 * 10.0) / 10.0)
}

/// 0x112 Byte 4 — NOT motion. Renamed 2026-09-12 after re-analysis.
///
/// 0x112 is a **vehicle module** broadcasting at ~10 Hz (it is still live with the
/// factory TCU removed, so Capture 0's "TCU-owned" classification was wrong).
/// Bytes 4-5 are a 16-bit counter block that resets to exactly 1000 (0x03E8) at the
/// start of each drive leg and then drifts in the 900-1000 band; byte 7 resets to 0
/// and climbs monotonically. Reading byte 4 alone only answers "is that 16-bit value
/// \>= 768", i.e. "has this counter block been initialised" — which latches on ~2-3 min
/// into a drive and never drops through stops.
///
/// Evidence it is not motion (drive_home 09-11): byte4=0x00 occurs at speeds up to
/// 38 km/h, and 22 % of byte4=0x03 samples are at speed < 0.5 km/h.
/// Use `decode_vehicle_speed` (0x107) for motion. Bytes 5 and 7 are unidentified.
pub fn decode_0x112_initialised(data: &[u8]) -> Option<bool> {
    if data.len() < 5 {
        return None;
    }
    match data[4] {
        0x00 => Some(false),
        0x03 => Some(true),
        _ => None,
    }
}

/// 0x15E multiplexed gateway bridge.
/// mux 0x01 = UTC wall clock at 1 Hz: B1 hour, B2 minute, B3 second,
///   B4 day-of-month, B5 month. B6 is a constant descriptor (0x0C), B7 pad.
/// mux 0x16 / 0x76 / 0x86 = GPS: B1:B2 latitude, B5:B6 longitude.
///   lat = 43.6591 + raw16 * 0.000256034 + (B3-128)*1e-6
///   lon = 5.0 + raw16 * 0.000016          (R²=1.000, MAE 2.5 m)
#[derive(Debug, Clone, PartialEq)]
pub enum GatewayMux {
    /// UTC wall clock.
    Clock { hour: u8, minute: u8, second: u8, day: u8, month: u8 },
    Gps { lat: f64, lon: f64 },
}

pub fn decode_gateway_mux(data: &[u8]) -> Option<GatewayMux> {
    if data.is_empty() {
        return None;
    }
    match data[0] {
        0x01 => {
            if data.len() < 4 {
                return None;
            }
            if data.len() < 6 {
                return None;
            }
            let hour = data[1];
            let minute = data[2];
            let second = data[3];
            let day = data[4];
            let month = data[5];
            if hour <= 23 && minute <= 59 && second <= 59 {
                Some(GatewayMux::Clock { hour, minute, second, day, month })
            } else {
                None
            }
        }
        0x16 | 0x76 | 0x86 => {
            if data.len() < 7 {
                return None;
            }
            let lat_raw = ((data[1] as u16) << 8) | (data[2] as u16);
            let lon_raw = ((data[5] as u16) << 8) | (data[6] as u16);
            let lat_fine = if data.len() >= 4 {
                (data[3] as f64 - 128.0) * 1e-6
            } else {
                0.0
            };
            let lat = 43.6591 + (lat_raw as f64) * 0.000256034 + lat_fine;
            let lon = 5.0 + (lon_raw as f64) * 0.000016;
            if (40.0..=60.0).contains(&lat) && (0.0..=15.0).contains(&lon) {
                Some(GatewayMux::Gps { lat, lon })
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vehicle_speed() {
        // Raw: 0x00 0x00 -> 0.0 km/h
        assert_eq!(decode_vehicle_speed(&[0x00, 0x00]), Some(0.0));
        // Raw: 0x14 0x50 = 5200 -> 52.0 km/h
        assert_eq!(decode_vehicle_speed(&[0x14, 0x50]), Some(52.0));
    }

    #[test]
    fn test_gear_position() {
        assert_eq!(decode_gear_position(&[0x00, 0x00, 0x60]), Some("P"));
        assert_eq!(decode_gear_position(&[0x00, 0x00, 0xE0]), Some("D"));
        assert_eq!(decode_gear_position(&[0x00, 0x00, 0x00]), None);
    }

    #[test]
    fn test_engine_rpm_and_ev_mode() {
        // 0xE000: EV Mode active, engine stopped
        let (rpm, running, ev) = decode_engine_status(&[0x80, 0x00, 0xE0, 0x00]).unwrap();
        assert_eq!(rpm, 0.0);
        assert!(!running);
        assert!(ev);

        // 0xE28C: ICE Running -> diff = 0x28C = 652 -> 1304 RPM
        let (rpm, running, ev) = decode_engine_status(&[0x7F, 0xA5, 0xE2, 0x8C]).unwrap();
        assert_eq!(rpm, 1304.0);
        assert!(running);
        assert!(!ev);

        // 0x0000: Vehicle unpowered
        let (rpm, running, ev) = decode_engine_status(&[0x80, 0x02, 0x00, 0x00]).unwrap();
        assert_eq!(rpm, 0.0);
        assert!(!running);
        assert!(!ev);
    }

    #[test]
    fn test_coolant_temp() {
        // 0x6E = 110 -> 110 - 60 = 50.0 deg C
        assert_eq!(decode_coolant_temp(&[0x00, 0x00, 0x6E]), Some(50.0));
    }

    #[test]
    fn test_fuel_level() {
        assert_eq!(decode_fuel_level(&[0x00, 0x03, 0xE8]), None);
        assert_eq!(decode_fuel_level(&[0x00, 0x03, 0x5B]), Some(85.9));
    }

    #[test]
    fn test_trip_and_dte() {
        // 0x118 this-drive ICE: 0x00 0x0B = 11 -> 1.1 km; DTE: 0x27 0xE2 = 10210 -> 1021.0 km
        let (trip_ice, dte) = decode_trip_and_dte(&[0x0C, 0x58, 0xC8, 0x00, 0x0B, 0x27, 0xE2, 0x00]).unwrap();
        assert!((trip_ice - 1.1).abs() < 0.01);
        assert!((dte - 1021.0).abs() < 0.01);
        assert_eq!(decode_trip_ice_km(&[0x0C, 0x58, 0xC8, 0x01, 0x76]), Some(37.4));
    }

    #[test]
    fn test_ignition_state() {
        assert_eq!(decode_ignition_state(&[0x17]), Some("ON"));
        assert_eq!(decode_ignition_state(&[0x03]), None);
        assert_eq!(decode_ignition_state(&[0x03, 0, 0, 0, 0, 0, 0, 0x00]), Some("OFF"));
        assert_eq!(decode_ignition_state(&[0x03, 0, 0, 0, 0, 0, 0, 0x20]), Some("ON"));
        assert_eq!(decode_ignition_state(&[0x03, 0, 0, 0, 0, 0, 0, 0xA0]), Some("ON"));
    }

    #[test]
    fn test_remote_start_timer() {
        // 0x03 0x84 = 900 seconds (15:00)
        assert_eq!(decode_remote_start_timer(&[0x22, 0x03, 0x84]), Some(900));
        // 0x00 0x00 = 0 seconds (idle / drive)
        assert_eq!(decode_remote_start_timer(&[0x22, 0x00, 0x00]), Some(0));
    }

    #[test]
    fn test_oil_life() {
        // B5 values seen in the corpus: 0x3A (58 %, flag clear), 0xBD (61 %, flag set).
        assert_eq!(decode_oil_life(&[0x00, 0x00, 0x58, 0x02, 0x4C, 0x3A, 0x00, 0x00]), Some(58));
        assert_eq!(decode_oil_life(&[0x02, 0x9A, 0x8F, 0xF6, 0x40, 0xBD, 0x20, 0x00]), Some(61));
        assert_eq!(decode_oil_life(&[0, 0, 0, 0, 0, 0x7F, 0, 0]), None); // 127 > 100
        assert_eq!(decode_oil_life(&[0, 0, 0]), None);
    }

    #[test]
    fn test_lamp_mode() {
        // Real frames from the corpus (B5 is the 6th byte).
        let f = |b5: u8| [0x22, 0x00, 0x00, 0x00, 0x04, b5, 0x00, 0x00];
        assert_eq!(decode_lamp_mode(&f(0x00)), Some(LampMode::Off));
        assert_eq!(decode_lamp_mode(&f(0x20)), Some(LampMode::LowBeam));
        assert_eq!(decode_lamp_mode(&f(0x40)), Some(LampMode::Parking));
        assert_eq!(decode_lamp_mode(&f(0x60)), Some(LampMode::DrlRightOff));
        assert_eq!(decode_lamp_mode(&f(0x80)), Some(LampMode::DrlLeftOff));
        assert_eq!(decode_lamp_mode(&f(0xA0)), Some(LampMode::Drl));
        assert_eq!(decode_lamp_mode(&f(0xE0)), Some(LampMode::Unknown(7)));
        // Low bits of B5 are not part of the field.
        assert_eq!(decode_lamp_mode(&f(0xA3)), Some(LampMode::Drl));
        assert!(LampMode::LowBeam.low_beam() && !LampMode::Drl.low_beam());
        assert!(LampMode::DrlLeftOff.drl() && !LampMode::Off.drl());
        assert_eq!(decode_lamp_mode(&[0x22, 0x00]), None);
    }

    #[test]
    fn test_bcm_status() {
        // Parked, locked, idle lights, TCU present:
        // [0x10, 0x48, 0x04, 0x11, 0x10, 0x05, 0x00, 0x02]
        let status = decode_bcm_status(&[0x10, 0x48, 0x04, 0x11, 0x10, 0x05, 0x00, 0x02]).unwrap();
        assert!(status.doors_locked);
        assert_eq!(status.turn_signal, Some("OFF"));
        assert!(!status.turn_signal_active);
        assert!(status.headlights_on);
        assert!(!status.front_fog_on);
        assert!(!status.rear_fog_on);
        assert!(!status.door_ajar_fl);
        assert!(!status.door_ajar_fr);
        assert!(!status.door_ajar_rl);
        assert!(!status.door_ajar_rr);
        assert!(!status.trunk_ajar);
        assert!(!status.hood_ajar);
        assert_eq!(status.ambient_light, 0x05);
        assert_eq!(status.day_night, Some("DAY"));

        let left = decode_bcm_status(&[0x40, 0x4B, 0x04, 0x11, 0x10, 0x05, 0x40, 0x02]).unwrap();
        assert!(left.turn_signal_active);
        assert!(left.turn_left_active && !left.turn_right_active);
        assert!(left.flasher_bulb_on);
        assert!(!left.hazards_active);
        assert_eq!(left.turn_signal, Some("LEFT"));

        let right = decode_bcm_status(&[0x40, 0x48, 0x04, 0x11, 0x18, 0x05, 0x00, 0xC2]).unwrap();
        assert!(right.turn_signal_active);
        assert!(right.turn_right_active && !right.turn_left_active);
        assert!(right.flasher_bulb_on);
        assert_eq!(right.turn_signal, Some("RIGHT"));

        let haz = decode_bcm_status(&[0x40, 0x4B, 0x04, 0x11, 0x18, 0x00, 0x40, 0xC2]).unwrap();
        assert!(haz.hazards_active);
        assert_eq!(haz.turn_signal, Some("HAZARD"));
        assert_eq!(haz.ambient_light, 0x00);
        assert!(!haz.courtesy_flash);

        let ack = decode_bcm_status(&[0x10, 0x42, 0x04, 0x00, 0xEE, 0x05, 0x40, 0x80]).unwrap();
        assert!(ack.courtesy_flash);
        assert!(!ack.turn_left_active && !ack.turn_right_active);
        assert_eq!(ack.turn_signal, Some("OFF"));
        assert!(!left.courtesy_flash);

        let front = decode_bcm_status(&[0x44, 0x48, 0x14, 0x11, 0x10, 0x05, 0x00, 0x03]).unwrap();
        assert!(front.front_fog_on);
        assert!(!front.rear_fog_on);
        let both = decode_bcm_status(&[0x46, 0x48, 0x14, 0x11, 0x10, 0x05, 0x00, 0x03]).unwrap();
        assert!(both.front_fog_on);
        assert!(both.rear_fog_on);

        let fl = decode_bcm_status(&[0x40, 0x48, 0x04, 0x11, 0x10, 0x05, 0x00, 0x22]).unwrap();
        assert!(fl.door_ajar_fl && !fl.door_ajar_fr && !fl.door_ajar_rl && !fl.door_ajar_rr);
        let fr = decode_bcm_status(&[0x40, 0x48, 0x04, 0x11, 0x10, 0x05, 0x00, 0x12]).unwrap();
        assert!(fr.door_ajar_fr && !fr.door_ajar_fl);
        let rl = decode_bcm_status(&[0x40, 0x48, 0x04, 0x11, 0x10, 0x05, 0x01, 0x02]).unwrap();
        assert!(rl.door_ajar_rl && !rl.door_ajar_rr);
        let rr = decode_bcm_status(&[0x40, 0x48, 0x04, 0x11, 0x10, 0x05, 0x02, 0x02]).unwrap();
        assert!(rr.door_ajar_rr && !rr.door_ajar_rl);

        let trunk_shut = decode_bcm_status(&[0x40, 0x48, 0x04, 0x11, 0x10, 0x05, 0x00, 0x22]).unwrap();
        assert!(!trunk_shut.trunk_ajar && trunk_shut.door_ajar_fl);
        let trunk_open = decode_bcm_status(&[0x41, 0x48, 0x04, 0x11, 0x10, 0x05, 0x00, 0x22]).unwrap();
        assert!(trunk_open.trunk_ajar && trunk_open.door_ajar_fl);

        let hood_shut = decode_bcm_status(&[0x40, 0x48, 0x04, 0x11, 0x10, 0x05, 0x00, 0x22]).unwrap();
        assert!(!hood_shut.hood_ajar && hood_shut.door_ajar_fl);
        let hood_open = decode_bcm_status(&[0x40, 0x48, 0x04, 0x11, 0x10, 0x05, 0x00, 0x2A]).unwrap();
        assert!(hood_open.hood_ajar && hood_open.door_ajar_fl && !hood_open.trunk_ajar);
    }

    #[test]
    fn test_vin_assembler() {
        let mut assembler = VinAssembler::new();
        // Synthetic 17-character VIN fixture: 3FA6P0XX9YY123456
        // Chunk 0: 3FA6P0
        assert_eq!(assembler.process_frame(&[0xC1, 0x00, 0x33, 0x46, 0x41, 0x36, 0x50, 0x30]), None);
        // Chunk 1: XX9YY1
        assert_eq!(assembler.process_frame(&[0xC1, 0x01, 0x58, 0x58, 0x39, 0x59, 0x59, 0x31]), None);
        // Chunk 2: 23456 + padding
        let vin = assembler.process_frame(&[0xC1, 0x02, 0x32, 0x33, 0x34, 0x35, 0x36, 0xFF]);
        assert_eq!(vin, Some("3FA6P0XX9YY123456"));
        assert_eq!(assembler.vin(), Some("3FA6P0XX9YY123456"));
    }

    #[test]
    fn test_brake_pressed() {
        // 0x101 byte 4 = 0x83 -> bit 7 = 1 (pressed)
        assert_eq!(decode_brake_pressed(&[0x00, 0x00, 0x00, 0x00, 0x83]), Some(true));
        // 0x101 byte 4 = 0x43 -> bit 7 = 0 (released)
        assert_eq!(decode_brake_pressed(&[0x00, 0x00, 0x00, 0x00, 0x43]), Some(false));
        // Short frame
        assert_eq!(decode_brake_pressed(&[0x00, 0x00]), None);
    }

    #[test]
    fn test_pedal_position() {
        // 0x103 bytes 0-1 = 0x8000 -> 0 demand (idle / coasting)
        assert_eq!(decode_pedal_position(&[0x80, 0x00]), Some(0.0));
        // 0x103 bytes 0-1 = 0x7FA5 -> braking / negative torque -> 0% gas pedal
        assert_eq!(decode_pedal_position(&[0x7F, 0xA5]), Some(0.0));
        // 0x103 bytes 0-1 = 0x810A -> +266 counts -> ~50.2%
        let p50 = decode_pedal_position(&[0x81, 0x0A]).unwrap();
        assert!((p50 - 50.2).abs() < 0.5);
        // 0x103 bytes 0-1 = 0x8212 -> +530 counts -> 100.0%
        assert_eq!(decode_pedal_position(&[0x82, 0x12]), Some(100.0));
    }

    #[test]
    fn test_brake_pedal_pct() {
        // Released: 0x80 0x00 -> 0.0%
        assert_eq!(decode_brake_pedal_pct(&[0x80, 0x00]), Some(0.0));
        // Normal stop: 0x80 0xBD -> raw 189 -> 18.5%
        assert_eq!(decode_brake_pedal_pct(&[0x80, 0xBD]), Some(18.5));
        // Harder deceleration: 0x81 0x51 -> raw 337 -> 32.9%
        assert_eq!(decode_brake_pedal_pct(&[0x81, 0x51]), Some(32.9));
        // Stationary hold at stoplight: 0x82 0x03 -> raw 515 -> 50.3%
        assert_eq!(decode_brake_pedal_pct(&[0x82, 0x03]), Some(50.3));
        // Stationary hold pre-start: 0x83 0xB3 -> raw 947 -> 92.6%
        assert_eq!(decode_brake_pedal_pct(&[0x83, 0xB3]), Some(92.6));
        // Uninitialized: 0x9F 0xFF -> None
        assert_eq!(decode_brake_pedal_pct(&[0x9F, 0xFF]), None);
        // 0x106 long accel: B4=FA B5=00 -> raw 512 -> 0.02 m/s²
        assert_eq!(decode_longitudinal_accel(&[0, 0, 0, 0, 0xFA, 0x00]), Some(0.02));
        assert_eq!(decode_longitudinal_accel(&[0, 0, 0, 0, 0x03, 0x00]), None);
        assert_eq!(decode_longitudinal_accel(&[0, 0, 0, 0, 0xFB, 0xFE]), None); // raw=1022 resting sentinel
        assert_eq!(decode_106_raw12(&[0, 0, 0, 0, 0xFA, 0x00, 0xD7, 0x00]), Some(0x700));
        assert_eq!(decode_106_raw12(&[0, 0, 0, 0, 0x03, 0x00, 0x0F, 0x00]), None);
        assert_eq!(decode_106_raw12(&[0, 0, 0, 0, 0xFA, 0x00, 0xDF, 0xFE]), None);
    }

    #[test]
    fn test_odometer_and_distances() {
        // 0x109 Total Odometer: [0x03, 0x47, 0xA1] -> 214,945 km
        assert_eq!(decode_odometer(&[0x03, 0x47, 0xA1]), Some(214945.0));

        // 0x113 Trip Distance: [0, 0, 0, 0, 0x00, 0xA3] -> 163 * 0.1 = 16.3 km
        assert_eq!(decode_trip_distance(&[0, 0, 0, 0, 0x00, 0xA3]), Some(16.3));
        // 0x113 trip fuel: B1=21 -> 2.1 L
        assert_eq!(decode_trip_fuel_l(&[0, 21, 0, 0, 0x01, 0xB5]), Some(2.1));
        assert_eq!(decode_trip_fuel_l(&[0, 0]), Some(0.0));

        // 0x118 ICE Distance: [0x0C, 0x5A, 0x60] -> 809,568 * 0.1 = 80956.8 km
        assert_eq!(decode_hybrid_ice_km(&[0x0C, 0x5A, 0x60]), Some(80956.8));

        // 0x118 DTE: bytes 5-6 [0x23, 0xD8] -> 9176 * 0.1 = 917.6 km
        assert_eq!(decode_dte(&[0, 0, 0, 0, 0, 0x23, 0xD8]), Some(917.6));
    }

    #[test]
    fn test_hybrid_soc_10f() {
        // Verified against OBD Traction Battery SOC: raw16 * 0.0025.
        // 0x418E = 16782 -> 41.955 -> 42.0 %; 0x61D6 = 25046 -> 62.615 -> 62.6 %.
        assert_eq!(decode_hybrid_soc_10f(&[0x41, 0x8E]), Some(42.0));
        assert_eq!(decode_hybrid_soc_10f(&[0x61, 0xD6]), Some(62.6));
        // ~50 % (what the OBD scanner showed): 0x4E20 = 20000 -> 50.0 %.
        assert_eq!(decode_hybrid_soc_10f(&[0x4E, 0x20]), Some(50.0));
        assert_eq!(decode_hybrid_soc_10f(&[0x00, 0x00]), None); // 0 % rejected
        assert_eq!(decode_hybrid_soc_10f(&[0xFF]), None);       // short frame
    }

    #[test]
    fn test_ice_running_142() {
        assert_eq!(decode_ice_running_142(&[0x4E, 0x20, 0x64]), Some(false));
        assert_eq!(decode_ice_running_142(&[0x4E, 0x20, 0x68]), Some(true));
        assert_eq!(decode_ice_running_142(&[0x4E, 0x20, 0x78]), None);
    }

    #[test]
    fn test_ambient_temp() {
        assert_eq!(decode_ambient_temp(&[0x4E]), Some(14.0));
        assert_eq!(decode_ambient_temp(&[0x40]), Some(0.0));
    }

    #[test]
    fn test_gateway_gps() {
        let lat_raw: u16 = 30227;
        let lon_raw: u16 = 3294;
        let frame = [
            0x16,
            (lat_raw >> 8) as u8,
            (lat_raw & 0xFF) as u8,
            0x00,
            0x05,
            (lon_raw >> 8) as u8,
            (lon_raw & 0xFF) as u8,
            0x00,
        ];
        match decode_gateway_mux(&frame) {
            Some(GatewayMux::Gps { lat, lon }) => {
                assert!((lat - 51.3981).abs() < 0.001);
                assert!((lon - 5.0527).abs() < 0.001);
            }
            other => panic!("expected GPS, got {:?}", other),
        }
    }

    #[test]
    fn test_gateway_clock() {
        assert_eq!(
            decode_gateway_mux(&[0x01, 0x0A, 0x13, 0x22, 0x0B, 0x09, 0x0C, 0x00]),
            Some(GatewayMux::Clock {
                hour: 10,
                minute: 19,
                second: 34,
                day: 11,
                month: 9
            })
        );
        assert_eq!(
            decode_vehicle_clock(&[0x00, 0x00, 0x01, 0x0F, 0x27, 0x1B, 0x06, 0x00]),
            Some((0x06, 0x27, 0x1B))
        );
    }

    #[test]
    fn test_hybrid_mode_and_tick() {
        assert_eq!(decode_hybrid_mode(&[0xE8]), Some("CITY"));
        assert_eq!(decode_hybrid_mode(&[0xF8]), Some("HIGHWAY"));
        assert_eq!(decode_motor_temp(&[0, 0, 0, 0, 0, 0, 68]), None);
        assert_eq!(decode_transaxle_park(&[0, 0, 0x39]), Some(true));
        assert_eq!(decode_transaxle_park(&[0, 0, 0x08]), Some(false));
        assert_eq!(decode_drive_active(&[0x17, 0, 0, 0, 0, 0, 0, 0x00]), Some(false));
        assert_eq!(decode_drive_active(&[0x17, 0, 0, 0, 0, 0, 0, 0x01]), Some(true));
        assert_eq!(decode_drive_alt(&[0x17, 0, 0, 0, 0x20, 0x04, 0, 0xA0]), Some(true));
        assert_eq!(decode_drive_alt(&[0x17, 0, 0, 0, 0x00, 0x00, 0, 0x20]), Some(false));
        let ang = decode_motor_angle(&[0x00, 0x12, 0x34, 0x80]).unwrap();
        assert!((ang - 25.6).abs() < 0.05);
        assert_eq!(decode_motor_angle(&[0x00, 0x00, 0x00, 0x00]), None);
        assert_eq!(decode_motor_angle(&[0xFF, 0xFF, 0xFF, 0x80]), None);
        assert_eq!(decode_motor_angle(&[0x00, 0x00, 0x00, 0x80]), Some(0.0));
        assert_eq!(decode_low_fuel_lamp(&[0, 3, 0x5B, 0x03]), None);
        assert_eq!(decode_low_fuel_lamp(&[0, 3, 0x5B, 0x83]), None);
        assert_eq!(decode_0x112_initialised(&[0, 0, 0, 0, 0x03]), Some(true));
        let frame = [0, 0, 0, 0, 0x01, 0x48];
        assert_eq!(decode_trip_mpg(&frame), Some(32.8));
        let l100 = decode_trip_l100km(&frame).unwrap();
        assert!((l100 - 7.2).abs() < 0.05);
        let amps = decode_hv_current(&[0xBA, 0xCB]).unwrap();
        assert!((amps - 2.55).abs() < 0.02);
        assert_eq!(decode_hv_voltage(&[0xBA, 0xCB, 0x02, 0x38]), Some(284.0));
        assert_eq!(decode_hv_current_gauge(&[0x27, 0x10]), Some(0.0));
    }
}
