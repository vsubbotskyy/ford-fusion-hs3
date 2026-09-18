#![no_std]

extern crate alloc;
use alloc::vec::Vec;

// ============================================================================
// Ford Fusion Hybrid (CD4) HS-CAN4 TCU Command Encoders & Factory Sequences
// Derived from factory FordPass TCU telemetry recordings (lock/unlock/start/stop)
// ============================================================================

/// Single CAN 2.0B frame with up to 8 bytes of data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    pub id: u32,
    pub data: [u8; 8],
}

impl Frame {
    pub const fn new(id: u32, data: [u8; 8]) -> Self {
        Self { id, data }
    }
}

/// A CAN frame scheduled to be sent at a specific offset from the start of a sequence
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledFrame {
    /// Milliseconds elapsed from t=0 of the sequence
    pub t_ms: u32,
    pub frame: Frame,
}

impl ScheduledFrame {
    pub const fn new(t_ms: u32, id: u32, data: [u8; 8]) -> Self {
        Self {
            t_ms,
            frame: Frame::new(id, data),
        }
    }
}

/// Rolling authentication credentials required by the vehicle BCM
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Auth {
    /// Incremented sequence counter (replay-protected by BCM)
    pub seq: u8,
    /// Last non-zero rolling token received from 0x100 bytes 0-1
    pub token: u16,
}

impl Auth {
    pub const fn new(seq: u8, token: u16) -> Self {
        Self { seq, token }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorCmd {
    Lock,
    Unlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerCmd {
    Start,
    Stop,
}

// Well-known CAN IDs on the TCU connector (HS-CAN4 / HS3)
pub const ID_CMD: u32 = 0x146;
pub const ID_CMD_ACTIVE: u32 = 0x22A;
pub const ID_27C: u32 = 0x27C;
pub const ID_BCM_STATUS: u32 = 0x1B3;
pub const ID_BCM_TOKEN: u32 = 0x100;

// Command bytes for 0x146 byte 0
pub const CMD_DOOR_LOCK: u8 = 0x0C;
pub const CMD_DOOR_UNLOCK: u8 = 0x04;
pub const CMD_POWER_START: u8 = 0x01;
pub const CMD_POWER_STOP: u8 = 0x02;

// Standard payloads
pub const C27C_IDLE: [u8; 8] = [0x00, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
pub const C27C_CMD: [u8; 8] = [0x16, 0x60, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00];
pub const IDLE_146: [u8; 8] = [0x00; 8];
pub const C22A_ACTIVE: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00, 0x01];
pub const C22A_IDLE: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00];

/// 0x146 door command: [cmd, 0, 0, seq, 0, 0, tok_hi, tok_lo]
pub fn encode_door(cmd: DoorCmd, auth: Auth) -> [u8; 8] {
    let cmd_byte = match cmd {
        DoorCmd::Lock => CMD_DOOR_LOCK,
        DoorCmd::Unlock => CMD_DOOR_UNLOCK,
    };
    [
        cmd_byte,
        0x00,
        0x00,
        auth.seq,
        0x00,
        0x00,
        (auth.token >> 8) as u8,
        (auth.token & 0xFF) as u8,
    ]
}

/// 0x146 power command: [cmd, 0, 0, 0, 0, seq, tok_hi, tok_lo]
pub fn encode_power(cmd: PowerCmd, auth: Auth) -> [u8; 8] {
    let cmd_byte = match cmd {
        PowerCmd::Start => CMD_POWER_START,
        PowerCmd::Stop => CMD_POWER_STOP,
    };
    [
        cmd_byte,
        0x00,
        0x00,
        0x00,
        0x00,
        auth.seq,
        (auth.token >> 8) as u8,
        (auth.token & 0xFF) as u8,
    ]
}

/// Generate the factory door lock/unlock timed sequence.
///
/// Timeline (identical across lock-A, unlock-A, unlock-3 captures):
///   t=0..3.2s   0x22A active (4 frames, 800ms spacing), 0x27C idle, 0x146 idle
///   t=3.2s      single 0x146 command pulse + 0x27C flips to command state
///   t=3.4s      0x22A active (+200ms)
///   t=4.2s..13s 0x146 held frame (12 frames, 800ms spacing, cmd cleared, seq held)
///   t=13.8s     0x146 and 0x22A return to idle
pub fn factory_door_sequence(cmd: DoorCmd, auth: Auth) -> Vec<ScheduledFrame> {
    let mut seq = Vec::with_capacity(64);

    // Phase 1: 4 lead-in cycles of 800 ms (0x22A active for 3.2s before the pulse)
    for i in 0..4 {
        let t = i * 800;
        seq.push(ScheduledFrame::new(t, ID_CMD_ACTIVE, C22A_ACTIVE));
        seq.push(ScheduledFrame::new(t, ID_27C, C27C_IDLE));
        seq.push(ScheduledFrame::new(t, ID_CMD, IDLE_146));
    }

    // Phase 2: Command pulse at t = 3200 ms
    let pulse_t = 3200;
    seq.push(ScheduledFrame::new(pulse_t, ID_27C, C27C_CMD));
    seq.push(ScheduledFrame::new(pulse_t, ID_CMD, encode_door(cmd, auth)));

    // 0x22A remains active at pulse + 200 ms
    seq.push(ScheduledFrame::new(pulse_t + 200, ID_CMD_ACTIVE, C22A_ACTIVE));

    // Phase 3: Hold frames (12 frames, ~1 Hz for ~10 s)
    // Command byte cleared, sequence and token held
    let tok_hi = (auth.token >> 8) as u8;
    let tok_lo = (auth.token & 0xFF) as u8;
    let held_door = [0x00, 0x00, 0x00, auth.seq, 0x00, 0x00, tok_hi, tok_lo];

    let hold_start = pulse_t + 200 + 800; // 4200 ms
    for i in 0..12 {
        let t = hold_start + i * 800;
        seq.push(ScheduledFrame::new(t, ID_CMD, held_door));
        seq.push(ScheduledFrame::new(t, ID_27C, C27C_CMD));
        seq.push(ScheduledFrame::new(
            t,
            ID_CMD_ACTIVE,
            if i == 0 { C22A_ACTIVE } else { C22A_IDLE },
        ));
    }

    // Phase 4: Return to idle at the end
    let finish_t = hold_start + 12 * 800; // 13800 ms
    seq.push(ScheduledFrame::new(finish_t, ID_CMD, IDLE_146));
    seq.push(ScheduledFrame::new(finish_t, ID_CMD_ACTIVE, C22A_IDLE));

    seq
}

/// Generate the factory remote start/stop timed sequence.
///
/// Follows the same timing structure as the door sequence, with power command encoding
/// and the sequence counter in byte 5 instead of byte 3.
pub fn factory_power_sequence(cmd: PowerCmd, auth: Auth) -> Vec<ScheduledFrame> {
    let mut seq = Vec::with_capacity(64);

    for i in 0..4 {
        let t = i * 800;
        seq.push(ScheduledFrame::new(t, ID_CMD_ACTIVE, C22A_ACTIVE));
        seq.push(ScheduledFrame::new(t, ID_27C, C27C_IDLE));
        seq.push(ScheduledFrame::new(t, ID_CMD, IDLE_146));
    }

    let pulse_t = 3200;
    seq.push(ScheduledFrame::new(pulse_t, ID_27C, C27C_CMD));
    seq.push(ScheduledFrame::new(pulse_t, ID_CMD, encode_power(cmd, auth)));
    seq.push(ScheduledFrame::new(pulse_t + 200, ID_CMD_ACTIVE, C22A_ACTIVE));

    let tok_hi = (auth.token >> 8) as u8;
    let tok_lo = (auth.token & 0xFF) as u8;
    let held_power = [0x00, 0x00, 0x00, 0x00, 0x00, auth.seq, tok_hi, tok_lo];

    let hold_start = pulse_t + 200 + 800;
    for i in 0..12 {
        let t = hold_start + i * 800;
        seq.push(ScheduledFrame::new(t, ID_CMD, held_power));
        seq.push(ScheduledFrame::new(t, ID_27C, C27C_CMD));
        seq.push(ScheduledFrame::new(
            t,
            ID_CMD_ACTIVE,
            if i == 0 { C22A_ACTIVE } else { C22A_IDLE },
        ));
    }

    let finish_t = hold_start + 12 * 800;
    seq.push(ScheduledFrame::new(finish_t, ID_CMD, IDLE_146));
    seq.push(ScheduledFrame::new(finish_t, ID_CMD_ACTIVE, C22A_IDLE));

    seq
}

/// TCU presence frames verified to originate from the factory TCU and vanish when unplugged.
/// Explicitly excludes GWM-owned frames (0x1B3, 0x1B4, 0x1B5, 0x1B8, 0x59E).
const TCU_PRESENCE_SET: [Frame; 13] = [
    Frame::new(0x146, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
    Frame::new(0x212, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
    Frame::new(0x27B, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
    Frame::new(0x27C, [0x16, 0x60, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00]),
    Frame::new(0x27D, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
    Frame::new(0x27E, [0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
    Frame::new(0x27F, [0x00, 0xF4, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
    Frame::new(0x28A, [0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x01, 0x00]),
    Frame::new(0x28B, [0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x11, 0x00]),
    Frame::new(0x28D, [0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00]),
    Frame::new(0x28E, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
    Frame::new(0x590, [0x90, 0x00, 0xFF, 0xFF, 0xFF, 0x00, 0xFF, 0xFF]),
    Frame::new(0x22A, [0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00]),
];

pub fn presence_frames() -> &'static [Frame] {
    &TCU_PRESENCE_SET
}

const TCU_MINIMAL_PRESENCE_SET: [Frame; 5] = [
    Frame::new(0x590, [0x90, 0x00, 0xFF, 0xFF, 0xFF, 0x00, 0xFF, 0xFF]),
    Frame::new(0x28A, [0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x01, 0x00]),
    Frame::new(0x28B, [0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x11, 0x00]),
    Frame::new(0x28D, [0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00]),
    Frame::new(0x28E, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
];

pub fn minimal_presence_frames() -> &'static [Frame] {
    &TCU_MINIMAL_PRESENCE_SET
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_door_lock() {
        let auth = Auth::new(0xD7, 0xA107);
        let data = encode_door(DoorCmd::Lock, auth);
        assert_eq!(
            data,
            [0x0C, 0x00, 0x00, 0xD7, 0x00, 0x00, 0xA1, 0x07]
        );
    }

    #[test]
    fn test_encode_door_unlock() {
        let auth = Auth::new(0xFB, 0xDAB3);
        let data = encode_door(DoorCmd::Unlock, auth);
        assert_eq!(
            data,
            [0x04, 0x00, 0x00, 0xFB, 0x00, 0x00, 0xDA, 0xB3]
        );
    }

    #[test]
    fn test_encode_power_start() {
        let auth = Auth::new(0x42, 0x1234);
        let data = encode_power(PowerCmd::Start, auth);
        assert_eq!(
            data,
            [0x01, 0x00, 0x00, 0x00, 0x00, 0x42, 0x12, 0x34]
        );
    }

    #[test]
    fn test_encode_power_stop() {
        let auth = Auth::new(0x43, 0x5678);
        let data = encode_power(PowerCmd::Stop, auth);
        assert_eq!(
            data,
            [0x02, 0x00, 0x00, 0x00, 0x00, 0x43, 0x56, 0x78]
        );
    }

    #[test]
    fn test_factory_door_sequence_structure() {
        let auth = Auth::new(0x10, 0xABCD);
        let seq = factory_door_sequence(DoorCmd::Unlock, auth);

        // Check that lead-in frames exist at t = 0, 800, 1600, 2400
        for i in 0..4 {
            let t = i * 800;
            assert!(seq.iter().any(|f| f.t_ms == t && f.frame.id == ID_CMD_ACTIVE && f.frame.data == C22A_ACTIVE));
            assert!(seq.iter().any(|f| f.t_ms == t && f.frame.id == ID_27C && f.frame.data == C27C_IDLE));
            assert!(seq.iter().any(|f| f.t_ms == t && f.frame.id == ID_CMD && f.frame.data == IDLE_146));
        }

        // Check command pulse at t = 3200
        let pulse_frame = seq.iter().find(|f| f.t_ms == 3200 && f.frame.id == ID_CMD).unwrap();
        assert_eq!(pulse_frame.frame.data[0], CMD_DOOR_UNLOCK);
        assert_eq!(pulse_frame.frame.data[3], 0x10);
        assert_eq!(pulse_frame.frame.data[6], 0xAB);
        assert_eq!(pulse_frame.frame.data[7], 0xCD);

        // Check 0x27C flips at t = 3200
        assert!(seq.iter().any(|f| f.t_ms == 3200 && f.frame.id == ID_27C && f.frame.data == C27C_CMD));

        // Check 0x22A active at t = 3400
        assert!(seq.iter().any(|f| f.t_ms == 3400 && f.frame.id == ID_CMD_ACTIVE && f.frame.data == C22A_ACTIVE));

        // Check hold frames at t = 4200..13000
        let hold_frames: Vec<_> = seq.iter().filter(|f| f.t_ms >= 4200 && f.t_ms <= 13000 && f.frame.id == ID_CMD).collect();
        assert_eq!(hold_frames.len(), 12);
        for h in hold_frames {
            assert_eq!(h.frame.data[0], 0x00); // cmd byte cleared
            assert_eq!(h.frame.data[3], 0x10); // seq held
            assert_eq!(h.frame.data[6], 0xAB); // token held
            assert_eq!(h.frame.data[7], 0xCD);
        }

        // Check finish to idle at t = 13800
        assert!(seq.iter().any(|f| f.t_ms == 13800 && f.frame.id == ID_CMD && f.frame.data == IDLE_146));
        assert!(seq.iter().any(|f| f.t_ms == 13800 && f.frame.id == ID_CMD_ACTIVE && f.frame.data == C22A_IDLE));
    }

    #[test]
    fn test_presence_frames_no_gwm_leakage() {
        let presence = presence_frames();
        assert_eq!(presence.len(), 13);
        let gwm_forbidden = [0x1B3, 0x1B4, 0x1B5, 0x1B8, 0x59E];
        for f in presence {
            assert!(
                !gwm_forbidden.contains(&f.id),
                "Presence set must never impersonate GWM-owned ID 0x{:03X}",
                f.id
            );
        }
    }

    #[test]
    fn test_factory_power_sequence_structure() {
        let auth = Auth::new(0x2A, 0x55AA);
        let seq = factory_power_sequence(PowerCmd::Start, auth);

        let pulse_frame = seq.iter().find(|f| f.t_ms == 3200 && f.frame.id == ID_CMD).unwrap();
        assert_eq!(pulse_frame.frame.data[0], CMD_POWER_START);
        assert_eq!(pulse_frame.frame.data[5], 0x2A); // power sequence byte in byte 5
        assert_eq!(pulse_frame.frame.data[6], 0x55);
        assert_eq!(pulse_frame.frame.data[7], 0xAA);

        let hold_frames: Vec<_> = seq.iter().filter(|f| f.t_ms >= 4200 && f.t_ms <= 13000 && f.frame.id == ID_CMD).collect();
        assert_eq!(hold_frames.len(), 12);
        for h in hold_frames {
            assert_eq!(h.frame.data[0], 0x00);
            assert_eq!(h.frame.data[5], 0x2A);
            assert_eq!(h.frame.data[6], 0x55);
            assert_eq!(h.frame.data[7], 0xAA);
        }
    }
}
