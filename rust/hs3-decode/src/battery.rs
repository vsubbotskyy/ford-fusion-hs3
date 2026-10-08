use crate::math::round_f32;

/// 0x100 Byte 6: HV (traction) battery temperature, °C = raw × 0.5 − 50.
/// r = 0.9975 vs OBD `HV Battery Temp`; falls while parked, so it is not a counter.
pub fn decode_hv_battery_temp(data: &[u8]) -> Option<f32> {
    if data.len() < 7 { return None; }
    Some(data[6] as f32 * 0.5 - 50.0)
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

