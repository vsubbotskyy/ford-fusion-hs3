#![no_std]

pub(crate) mod math;
pub mod battery;
pub mod bcm;
pub mod chassis;
pub mod gateway;
pub mod powertrain;
pub mod trip;

pub use battery::*;
pub use bcm::*;
pub use chassis::*;
pub use gateway::*;
pub use powertrain::*;
pub use trip::*;

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
    fn test_fuel_sender_pct() {
        assert_eq!(decode_fuel_sender_pct(&[0x00, 0x03, 0xE8]), None);
        assert_eq!(decode_fuel_sender_pct(&[0x00, 0x03, 0x5B]), Some(85.9));
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
    fn test_temps_and_windows() {
        // 0x100 B6 = 0x9C -> 28 °C.
        assert_eq!(decode_hv_battery_temp(&[0x7C, 0x60, 0x20, 0x08, 0x5F, 0x90, 0x9C, 0xC0]), Some(28.0));
        assert_eq!(decode_hv_battery_temp(&[0; 6]), None);
        // 0x108 B0 = 0xA3 (163) -> 24.5 °C.
        assert_eq!(decode_cabin_temp(&[0xA3, 0x33, 0x33]), Some(24.5));
        // All closed; one window fully open (B1 = B3); two open (BB); mid-travel frame 42.
        assert_eq!(decode_windows(&[0xA3, 0x33, 0x33]), Some([Some(0), Some(0), Some(0), Some(0)]));
        assert_eq!(decode_windows(&[0xA3, 0xB3, 0x33]), Some([Some(100), Some(0), Some(0), Some(0)]));
        assert_eq!(decode_windows(&[0xA3, 0xBB, 0x33]), Some([Some(100), Some(100), Some(0), Some(0)]));
        assert_eq!(decode_windows(&[0xA3, 0x72, 0x33]), Some([Some(50), Some(0), Some(0), Some(0)]));
        assert_eq!(decode_windows(&[0xA3, 0x00, 0x33]), Some([None, None, Some(0), Some(0)]));
        assert_eq!(decode_windows(&[0xA3, 0x33]), None);
        let w = decode_window_positions(&[0xA3, 0x3B, 0x53]).unwrap();
        assert_eq!((w.front_left, w.front_right, w.rear_left, w.rear_right),
                   (Some(0), Some(100), Some(25), Some(0)));
    }

    #[test]
    fn test_gear_selector() {
        // 0x101 B3 values from the corpus; a P->D sweep reads 01 21 23 13 11 21 31.
        let f = |b3: u8| [0x00, 0xA8, 0x00, b3, 0x43, 0xF9, 0xC0, 0xE9];
        assert_eq!(decode_gear_selector(&f(0x01)), Some(GearSelector::Park));
        assert_eq!(decode_gear_selector(&f(0x11)), Some(GearSelector::Reverse));
        assert_eq!(decode_gear_selector(&f(0x13)), Some(GearSelector::Reverse));
        assert_eq!(decode_gear_selector(&f(0x21)), Some(GearSelector::Neutral));
        assert_eq!(decode_gear_selector(&f(0x31)), Some(GearSelector::Drive));
        assert_eq!(GearSelector::Neutral.as_str(), "N");
        assert_eq!(decode_gear_selector(&[0, 0, 0]), None);
    }

    #[test]
    fn test_restraints() {
        // Solo drive, driver buckled: B1=B0 B2=80.
        let r = decode_restraints(&[0xE8, 0xB0, 0x80, 0x09]).unwrap();
        assert_eq!(r.driver_belt_buckled, Some(true));
        assert_eq!(r.passenger_belt_buckled, Some(false));
        assert_eq!(r.passenger_seat_occupied, Some(false));
        // Passenger aboard and buckled: B1=A8 B2=40.
        let r = decode_restraints(&[0xE8, 0xA8, 0x40]).unwrap();
        assert_eq!((r.driver_belt_buckled, r.passenger_belt_buckled, r.passenger_seat_occupied),
                   (Some(true), Some(true), Some(true)));
        // Parked, nobody buckled: B1=D0 B2=80.
        let r = decode_restraints(&[0xE0, 0xD0, 0x80]).unwrap();
        assert_eq!(r.driver_belt_buckled, Some(false));
        // Ignition-on init: F8 C0 -> all unknown; 50 (bit7 clear) -> driver unknown.
        assert_eq!(decode_restraints(&[0, 0xF8, 0xC0]).unwrap(), Restraints::default());
        assert_eq!(decode_restraints(&[0, 0x50, 0xC0]).unwrap().driver_belt_buckled, None);
        assert_eq!(decode_restraints(&[0, 0]), None);
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

    #[test]
    fn test_steering_angle() {
        // 0x175 B3:B4 = 0x2000 = 8192 -> centre (0.0°)
        assert_eq!(decode_steering_angle(&[0, 0, 0, 0x20, 0x00]), Some(0.0));
        // 0x175 B3:B4 = 0x2064 = 8292 -> (8292 - 8192) * 0.1 = 10.0° right
        assert_eq!(decode_steering_angle(&[0, 0, 0, 0x20, 0x64]), Some(10.0));
        // 0x175 B3:B4 = 0x1F9C = 8092 -> (8092 - 8192) * 0.1 = -10.0° left
        assert_eq!(decode_steering_angle(&[0, 0, 0, 0x1F, 0x9C]), Some(-10.0));
        // Out of range (>900°) -> None
        assert_eq!(decode_steering_angle(&[0, 0, 0, 0xFF, 0xFF]), None);
        // Short frame
        assert_eq!(decode_steering_angle(&[0, 0, 0, 0x20]), None);
    }

    #[test]
    fn test_tire_pressures() {
        // 0x1B5: B1=220 (2.20 bar), B3=225 (2.25), B5=218 (2.18), B7=230 (2.30)
        let data = [0x00, 220, 0x00, 225, 0x00, 218, 0x00, 230];
        let p = decode_tire_pressures(&data).unwrap();
        assert!((p[0] - 2.20).abs() < 0.01);
        assert!((p[1] - 2.25).abs() < 0.01);
        assert!((p[2] - 2.18).abs() < 0.01);
        assert!((p[3] - 2.30).abs() < 0.01);
        // Out of range (below 0.5 bar) -> None
        assert_eq!(decode_tire_pressures(&[0, 10, 0, 10, 0, 10, 0, 10]), None);
        // Short frame
        assert_eq!(decode_tire_pressures(&[0, 220, 0, 225, 0, 218, 0]), None);
    }

    #[test]
    fn test_fuel_accumulator() {
        // 0x109 B3:B4 = 0x01 0x00 -> 256 ticks
        assert_eq!(decode_fuel_accumulator(&[0, 0, 0, 0x01, 0x00]), Some(256));
        // Zero ticks
        assert_eq!(decode_fuel_accumulator(&[0, 0, 0, 0x00, 0x00]), Some(0));
        // Short frame
        assert_eq!(decode_fuel_accumulator(&[0, 0, 0, 0x01]), None);
    }

    #[test]
    fn test_bcm_token() {
        // 0x100 B0:B1 = 0x12 0x34 -> 0x1234
        assert_eq!(decode_bcm_token(&[0x12, 0x34]), Some(0x1234));
        // Zero token -> None (invalid)
        assert_eq!(decode_bcm_token(&[0x00, 0x00]), None);
        // Short frame
        assert_eq!(decode_bcm_token(&[0x12]), None);
    }

    #[test]
    fn test_motor_coil_temp() {
        // 0x141 B2 = 100 -> 100 - 40 = 60.0 °C
        assert_eq!(decode_motor_coil_temp(&[0, 0, 100]), Some(60.0));
        // B2 = 40 -> 0.0 °C (minimum valid)
        assert_eq!(decode_motor_coil_temp(&[0, 0, 40]), Some(0.0));
        // B2 = 39 -> below offset, None
        assert_eq!(decode_motor_coil_temp(&[0, 0, 39]), None);
        // Short frame
        assert_eq!(decode_motor_coil_temp(&[0, 0]), None);
    }

    #[test]
    #[allow(deprecated)]
    fn test_hybrid_soc_deprecated() {
        // 0x108 B0 = 0xA0 (160) -> 160 * 0.5 = 80.0 %
        assert_eq!(decode_hybrid_soc(&[0xA0]), Some(80.0));
        // Out of range (< 40%)
        assert_eq!(decode_hybrid_soc(&[0x20]), None);
        // Empty frame
        assert_eq!(decode_hybrid_soc(&[]), None);
    }

    #[test]
    fn test_10f_raw() {
        assert_eq!(decode_10f_raw(&[0x41, 0x8E]), Some(0x418E));
        assert_eq!(decode_10f_raw(&[0x00, 0x00]), Some(0));
        // Short frame
        assert_eq!(decode_10f_raw(&[0x41]), None);
    }

    #[test]
    fn test_odometer_trip_alias() {
        // 0x113 B4:B5 = 0x00 0xA3 -> 163 * 0.1 = 16.3 km -> 16.3 as f64
        let result = decode_odometer_trip(&[0, 0, 0, 0, 0x00, 0xA3]).unwrap();
        assert!((result - 16.3).abs() < 0.01);
        // Short frame
        assert_eq!(decode_odometer_trip(&[0, 0, 0, 0, 0x00]), None);
    }
}
