//! Rust ↔ Python decoder parity, through one shared vector file.
//!
//! This test decodes deterministic pseudo-random payloads (plus edge cases) for every
//! decoder listed in `CASES` and compares the result with `testdata/parity/vectors.txt`.
//! `python/tests/test_parity.py` reads the same file and checks `parse_frame` gives the
//! same values. A change to either decoder fails one side; an intended change means:
//! regenerate here (`UPDATE_GOLDEN=1 cargo test -p ford-fusion-hs3-decode --test parity`),
//! review the diff, and make the other language agree.
//!
//! Line format: `<name> <id hex> <data hex> <value>`; value is `-` for None, `true`/`false`,
//! a string, or a number printed with `{}`.

use hs3_decode as d;

type Dec = fn(&[u8]) -> Option<String>;

fn num<T: std::fmt::Display>(v: Option<T>) -> Option<String> {
    v.map(|x| format!("{x}"))
}

macro_rules! n {
    ($f:path) => {
        |b: &[u8]| num($f(b))
    };
}

/// (name, CAN id, decoder). The name is the Rust function; Python maps it to its key.
#[allow(deprecated)]
fn cases() -> Vec<(&'static str, u32, Dec)> {
    vec![
        ("bcm_token", 0x100, n!(d::decode_bcm_token)),
        ("hv_battery_temp", 0x100, n!(d::decode_hv_battery_temp)),
        ("brake_pressed", 0x101, n!(d::decode_brake_pressed)),
        ("engine_status.rpm", 0x103, |b| d::decode_engine_status(b).map(|t| format!("{}", t.0))),
        ("engine_status.running", 0x103, |b| d::decode_engine_status(b).map(|t| format!("{}", t.1))),
        ("engine_status.ev_mode", 0x103, |b| d::decode_engine_status(b).map(|t| format!("{}", t.2))),
        ("coolant_temp", 0x104, n!(d::decode_coolant_temp)),
        ("oil_life", 0x104, n!(d::decode_oil_life)),
        ("outside_temp", 0x104, n!(d::decode_outside_temp)),
        ("brake_pedal_pct", 0x106, n!(d::decode_brake_pedal_pct)),
        ("longitudinal_accel", 0x106, n!(d::decode_longitudinal_accel)),
        ("106_raw12", 0x106, n!(d::decode_106_raw12)),
        ("vehicle_speed", 0x107, n!(d::decode_vehicle_speed)),
        ("cabin_temp", 0x108, n!(d::decode_cabin_temp)),
        ("odometer", 0x109, n!(d::decode_odometer)),
        ("fuel_accumulator", 0x109, n!(d::decode_fuel_accumulator)),
        ("ignition_state", 0x10E, |b| d::decode_ignition_state(b).map(str::to_string)),
        ("drive_active", 0x10E, n!(d::decode_drive_active)),
        ("drive_alt", 0x10E, n!(d::decode_drive_alt)),
        ("10f_raw", 0x10F, n!(d::decode_10f_raw)),
        ("motor_angle", 0x110, n!(d::decode_motor_angle)),
        ("0x112_initialised", 0x112, n!(d::decode_0x112_initialised)),
        ("trip_distance", 0x113, n!(d::decode_trip_distance)),
        ("trip_fuel_l", 0x113, n!(d::decode_trip_fuel_l)),
        ("hybrid_ice_km", 0x118, n!(d::decode_hybrid_ice_km)),
        ("trip_ice_km", 0x118, n!(d::decode_trip_ice_km)),
        ("dte", 0x118, n!(d::decode_dte)),
        ("motor_coil_temp", 0x141, n!(d::decode_motor_coil_temp)),
        ("ice_running_142", 0x142, n!(d::decode_ice_running_142)),
        ("remote_start_timer", 0x147, n!(d::decode_remote_start_timer)),
        ("trip_mpg", 0x153, n!(d::decode_trip_mpg)),
        ("trip_l100km", 0x153, n!(d::decode_trip_l100km)),
        ("fuel_sender_pct", 0x174, n!(d::decode_fuel_sender_pct)),
        ("steering_angle", 0x175, n!(d::decode_steering_angle)),
        ("bcm_status.doors_locked", 0x1B3, |b| d::decode_bcm_status(b).map(|s| format!("{}", s.doors_locked))),
        ("bcm_status.trunk_ajar", 0x1B3, |b| d::decode_bcm_status(b).map(|s| format!("{}", s.trunk_ajar))),
        ("bcm_status.hood_ajar", 0x1B3, |b| d::decode_bcm_status(b).map(|s| format!("{}", s.hood_ajar))),
        ("bcm_status.front_fog_on", 0x1B3, |b| d::decode_bcm_status(b).map(|s| format!("{}", s.front_fog_on))),
        ("bcm_status.rear_fog_on", 0x1B3, |b| d::decode_bcm_status(b).map(|s| format!("{}", s.rear_fog_on))),
        ("bcm_status.headlights_on", 0x1B3, |b| d::decode_bcm_status(b).map(|s| format!("{}", s.headlights_on))),
        ("bcm_status.turn_signal", 0x1B3, |b| d::decode_bcm_status(b).and_then(|s| s.turn_signal).map(str::to_string)),
        ("bcm_status.day_night", 0x1B3, |b| d::decode_bcm_status(b).and_then(|s| s.day_night).map(str::to_string)),
        ("bcm_status.ambient_light", 0x1B3, |b| d::decode_bcm_status(b).map(|s| format!("{}", s.ambient_light))),
        ("bcm_status.courtesy_flash", 0x1B3, |b| d::decode_bcm_status(b).map(|s| format!("{}", s.courtesy_flash))),
    ]
}

/// Payloads for one id: all-zero, all-FF, then 120 from a fixed LCG (seeded by the id)
/// so the file is reproducible without a rand dependency.
fn payloads(id: u32) -> Vec<[u8; 8]> {
    let mut v = vec![[0u8; 8], [0xFF; 8]];
    let mut s: u64 = 0x9E37_79B9_7F4A_7C15 ^ id as u64;
    for _ in 0..120 {
        let mut p = [0u8; 8];
        for b in p.iter_mut() {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *b = (s >> 56) as u8;
        }
        v.push(p);
    }
    v
}

fn render() -> String {
    let mut out = String::from("# Generated by rust/hs3-decode/tests/parity.rs; read by python/tests/test_parity.py.\n");
    for (name, id, f) in cases() {
        for p in payloads(id) {
            let hex: String = p.iter().map(|b| format!("{b:02X}")).collect();
            let val = f(&p).unwrap_or_else(|| "-".into());
            out.push_str(&format!("{name} {id:03X} {hex} {val}\n"));
        }
    }
    out
}

#[test]
fn rust_decoders_match_the_shared_vectors() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/parity/vectors.txt");
    let got = render();
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(path, &got).unwrap();
        return;
    }
    let want = std::fs::read_to_string(path).expect("vectors.txt missing: run with UPDATE_GOLDEN=1");
    let diffs: Vec<String> = want
        .lines()
        .zip(got.lines())
        .filter(|(a, b)| a != b)
        .take(10)
        .map(|(a, b)| format!("  want {a}\n   got {b}"))
        .collect();
    assert!(
        diffs.is_empty() && want.lines().count() == got.lines().count(),
        "Rust decoders changed against testdata/parity/vectors.txt:\n{}\nIf intended: UPDATE_GOLDEN=1 cargo test -p ford-fusion-hs3-decode --test parity, then make python/tests/test_parity.py pass.",
        diffs.join("\n")
    );
}
