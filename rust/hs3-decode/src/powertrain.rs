use crate::math::round_f32;

/// 0x107 Bytes 0-1 (Big-Endian, 0.01 km/h / LSB) — Verified r=0.99995
pub fn decode_vehicle_speed(data: &[u8]) -> Option<f32> {
    if data.len() < 2 { return None; }
    let raw = ((data[0] as u16) << 8) | (data[1] as u16);
    let speed = (raw as f32) * 0.01;
    if speed < 260.0 { Some(speed) } else { None }
}

/// 0x107 Byte 2: Park latch, not PRND. For P/R/N/D use [`decode_gear_selector`] (0x101).
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

/// Gear selector (rotary dial) position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GearSelector {
    Park,
    Reverse,
    Neutral,
    Drive,
}

impl GearSelector {
    pub fn as_str(&self) -> &'static str {
        match self {
            GearSelector::Park => "P",
            GearSelector::Reverse => "R",
            GearSelector::Neutral => "N",
            GearSelector::Drive => "D",
        }
    }
}

/// 0x101 B3 bits 5:4 — gear selector: 0 = P, 1 = R, 2 = N, 3 = D.
/// A P→D turn of the dial passes R and N within ~0.4 s, so debounce if you log changes.
pub fn decode_gear_selector(data: &[u8]) -> Option<GearSelector> {
    if data.len() < 4 { return None; }
    Some(match (data[3] >> 4) & 0x03 {
        0 => GearSelector::Park,
        1 => GearSelector::Reverse,
        2 => GearSelector::Neutral,
        _ => GearSelector::Drive,
    })
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

/// 0x105 Byte 0: hybrid operating mode.
pub fn decode_hybrid_mode(data: &[u8]) -> Option<&'static str> {
    match data.first()? {
        0xE0 => Some("OFF"),
        0xE8 => Some("CITY"),
        0xF8 => Some("HIGHWAY"),
        _ => None,
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

