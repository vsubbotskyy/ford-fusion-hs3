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

/// Seat-belt and passenger-seat status from 0x105. Each field is `None` while the
/// restraints module initialises (raw 3, right after ignition on) or reports 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Restraints {
    pub driver_belt_buckled: Option<bool>,
    pub passenger_belt_buckled: Option<bool>,
    pub passenger_seat_occupied: Option<bool>,
}

fn yes_no_2bit(v: u8) -> Option<bool> {
    match v & 0x03 {
        1 => Some(true),
        2 => Some(false),
        _ => None,
    }
}

/// 0x105 B1 bits 6:5 driver belt, B1 bits 4:3 passenger belt, B2 bits 7:6 passenger
/// seat occupied. Each 2-bit field: 1 = yes, 2 = no, 3 = initialising.
/// B1 bit 7 = 0 briefly after ignition on; the driver belt is not reported then.
pub fn decode_restraints(data: &[u8]) -> Option<Restraints> {
    if data.len() < 3 { return None; }
    let b1 = data[1];
    Some(Restraints {
        driver_belt_buckled: if b1 & 0x80 != 0 { yes_no_2bit(b1 >> 5) } else { None },
        passenger_belt_buckled: yes_no_2bit(b1 >> 3),
        passenger_seat_occupied: yes_no_2bit(data[2] >> 6),
    })
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

/// 0x108 Byte 0: cabin temperature, °C ≈ raw × 0.5 − 57. **Candidate** (r = 0.97 / 0.98 vs
/// OBD `Temp Inside Car` in two logs). Same byte as the superseded [`decode_hybrid_soc`].
pub fn decode_cabin_temp(data: &[u8]) -> Option<f32> {
    if data.is_empty() { return None; }
    Some(data[0] as f32 * 0.5 - 57.0)
}

/// 0x108 B1:B2: four window positions, % open, in the order
/// `[front_left (driver), front_right (passenger), rear_left, rear_right]`
/// (B1 high, B1 low, B2 high, B2 low; verified with a labelled test 2026-09-24).
/// Nibble bits 3:1 = 1 (closed) … 5 (fully open), returned as 0/25/50/75/100 % open.
/// Nibble bit 0 is a flag that is not part of the position (drops briefly while a switch
/// is pressed). See [`decode_window_positions`] for named fields.
pub fn decode_windows(data: &[u8]) -> Option<[Option<u8>; 4]> {
    if data.len() < 3 { return None; }
    let pos = |nib: u8| match (nib >> 1) & 0x07 {
        v @ 1..=5 => Some((v - 1) * 25),
        _ => None,
    };
    Some([pos(data[1] >> 4), pos(data[1] & 0x0F), pos(data[2] >> 4), pos(data[2] & 0x0F)])
}

/// Window positions, % open (0/25/50/75/100). `None` when the nibble is out of range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WindowPositions {
    pub front_left: Option<u8>,
    pub front_right: Option<u8>,
    pub rear_left: Option<u8>,
    pub rear_right: Option<u8>,
}

/// 0x108 B1:B2 as named windows. Same data as [`decode_windows`].
pub fn decode_window_positions(data: &[u8]) -> Option<WindowPositions> {
    let [front_left, front_right, rear_left, rear_right] = decode_windows(data)?;
    Some(WindowPositions { front_left, front_right, rear_left, rear_right })
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

