use crate::math::round_f32;

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

