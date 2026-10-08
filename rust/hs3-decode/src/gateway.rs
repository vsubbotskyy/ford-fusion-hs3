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

