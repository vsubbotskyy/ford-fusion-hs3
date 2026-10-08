use crate::math::{round_f32, round_f64};

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

/// 0x174 Bytes 1-2, Big-Endian × 0.1 %. **Not the tank level.**
///
/// The field sits on `0x03E8` (100.0 %) most of the time and only carries other
/// values in short bursts that jump around within one drive (70–99 %), while the
/// cluster's distance to empty fell from 1027 to 198 km over the same captures.
/// Likely an instantaneous / unfiltered sender value tied to the gauge-commit strobe
/// (B3 bit 7). Returns the burst value; `0` and `1000` are rejected. For fuel, use
/// [`decode_dte`].
pub fn decode_fuel_sender_pct(data: &[u8]) -> Option<f32> {
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

/// Old name of [`decode_fuel_sender_pct`]. It was believed to be the tank level; it is not.
#[deprecated(since = "0.4.5", note = "0x174 B1:B2 is not the tank level: use decode_dte for fuel, decode_fuel_sender_pct for the raw value")]
pub fn decode_fuel_level(data: &[u8]) -> Option<f32> {
    decode_fuel_sender_pct(data)
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

