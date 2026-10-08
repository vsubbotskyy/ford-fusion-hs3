import unittest
from pathlib import Path
from hs3_decode import parse_frame, PARSE_CLASS
from hs3_decode.parser import load_csv


class TestHs3Parser(unittest.TestCase):
    def test_parse_individual_frames(self):
        # 0x107: Speed 55.00 km/h
        dec = parse_frame(0x107, [0x15, 0x7C, 0xE0, 0x00, 0x00, 0x00, 0x00, 0x00])
        self.assertEqual(dec.get("speed_kmh"), 55.0)
        self.assertEqual(dec.get("gear"), "D/R/N")

        # 0x104: Coolant 84.0 C
        dec = parse_frame(0x104, [0x00, 0x00, 0x90, 0x00, 0x00, 0x00, 0x00, 0x00])
        self.assertEqual(dec.get("coolant_c"), 84.0)

        # 0x1B3: Locked (bit2=0)
        dec = parse_frame(0x1B3, [0x10, 0x08, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00])
        self.assertTrue(dec.get("doors_locked"))
        self.assertTrue(dec.get("headlights"))

    def test_temps_and_windows(self):
        self.assertEqual(parse_frame(0x100, [0x7C, 0x60, 0x20, 0x08, 0x5F, 0x90, 0x9C, 0xC0])["hv_battery_temp_c"], 28.0)
        dec = parse_frame(0x108, [0xA3, 0xB3, 0x33, 0xF0, 0xE5, 0, 0, 0])
        self.assertEqual(dec["cabin_temp_c"], 24.5)
        self.assertEqual([dec[f"window_{n}_open_pct"] for n in range(1, 5)], [100, 0, 0, 0])
        self.assertEqual([dec[f"window_{w}_open_pct"] for w in ("fl", "fr", "rl", "rr")], [100, 0, 0, 0])

    def test_gear_selector_and_restraints(self):
        self.assertEqual(parse_frame(0x101, [0, 0xA8, 0, 0x31, 0x43, 0xF9, 0xC0, 0xE9])["gear_selector"], "D")
        self.assertEqual(parse_frame(0x101, [0, 0xA8, 0, 0x13, 0x43, 0xF9, 0xC0, 0xE9])["gear_selector"], "R")
        self.assertEqual(parse_frame(0x101, [0, 0xA8, 0, 0x01, 0x43, 0xF9, 0xC0, 0xC7])["gear_selector"], "P")
        dec = parse_frame(0x105, [0xE8, 0xA8, 0x40, 0x09, 0xFF, 0xB2, 0x64, 0x8C])
        self.assertTrue(dec["driver_belt_buckled"])
        self.assertTrue(dec["passenger_belt_buckled"])
        self.assertTrue(dec["passenger_seat_occupied"])
        dec = parse_frame(0x105, [0xE0, 0xF8, 0xC0, 0x08, 0, 0x10, 0x60, 0x08])
        self.assertIsNone(dec["driver_belt_buckled"])
        self.assertIsNone(dec["passenger_seat_occupied"])

    def test_load_and_parse_synthetic_csv(self):
        csv_path = Path(__file__).resolve().parents[2] / "testdata" / "synthetic_sample.csv"
        self.assertTrue(csv_path.exists(), f"Missing fixture at {csv_path}")
        frames = load_csv(csv_path)
        self.assertEqual(len(frames), 11)
        for _, cid, data in frames:
            cls = PARSE_CLASS.get(cid)
            self.assertIsNotNone(cls, f"Unknown CAN ID 0x{cid:03X}")
            dec = parse_frame(cid, data)
            self.assertIsInstance(dec, dict)


    def test_powertrain_and_chassis_frames(self):
        # 0x07A: HV battery current & voltage
        dec = parse_frame(0x07A, [0x3A, 0x98, 0x02, 0x58, 0x64, 0x60])
        self.assertEqual(dec.get("hv_voltage_v"), 300.0)
        self.assertEqual(dec.get("hv_vmax_v"), 200)
        self.assertEqual(dec.get("hv_vmin_v"), 192)

        # 0x084: Clock and calendar
        dec = parse_frame(0x084, [0, 0, 0x00, 0x2A, 14, 30, 9])
        self.assertEqual(dec.get("clock"), "09:14:30")
        self.assertEqual(dec.get("calendar_day"), 42)

        # 0x103: Engine RPM and pedal
        dec = parse_frame(0x103, [0x81, 0x00, 0xE3, 0xE8, 0, 0])
        self.assertEqual(dec.get("rpm"), 2000.0)
        self.assertTrue(dec.get("engine_running"))
        self.assertFalse(dec.get("ev_mode"))

        # 0x106: Brake pct & acceleration
        dec = parse_frame(0x106, [0x02, 0x00, 0x0B, 0xFE, 0xF8, 0x00, 0x00, 0x00])
        self.assertAlmostEqual(dec.get("brake_pct"), 50.0, places=0)
        self.assertEqual(dec.get("brake_cluster"), 0x0BFE)
        self.assertTrue(dec.get("brake_cluster_sentinel"))

        # 0x109: Odometer and fuel accum
        dec = parse_frame(0x109, [0x01, 0x86, 0xA0, 0x03, 0xE8, 0, 0x42])
        self.assertEqual(dec.get("odometer_km"), 100000)
        self.assertEqual(dec.get("fuel_accum"), 1000)
        self.assertEqual(dec.get("odo_companion"), 0x42)

        # 0x10A: Transaxle state
        self.assertEqual(parse_frame(0x10A, [0, 0, 0x39]).get("transaxle"), "P")
        self.assertEqual(parse_frame(0x10A, [0, 0, 0x08]).get("transaxle"), "not-P")

        # 0x10C: Thermal live
        dec = parse_frame(0x10C, [45, 0, 0, 2, 0xB0, 0, 0x1A])
        self.assertTrue(dec.get("thermal_live"))
        self.assertEqual(dec.get("b6_live_band"), 0x1A)
        self.assertEqual(dec.get("component_temp_raw"), 45)

        # 0x10E: Ignition & drive
        dec = parse_frame(0x10E, [0x17, 0, 0, 0, 0x20, 0x04, 0, 1])
        self.assertEqual(dec.get("ignition"), "ON")
        self.assertTrue(dec.get("drive_active"))
        self.assertTrue(dec.get("drive_alt"))
        self.assertTrue(dec.get("drive_alt_b5"))

        # 0x10F: Raw16
        dec = parse_frame(0x10F, [0x12, 0x34])
        self.assertEqual(dec.get("raw16"), 0x1234)

        # 0x110: Motor angle
        dec = parse_frame(0x110, [0x01, 0x40, 0x00, 0x80])
        self.assertEqual(dec.get("mg_high"), 1)
        self.assertEqual(dec.get("motor_angle_deg"), 90.0)

        # 0x112: TCU motion / tick
        dec = parse_frame(0x112, [0, 0, 0, 0, 0x03, 0x20, 0, 0x55])
        self.assertTrue(dec.get("tcu_motion"))
        self.assertEqual(dec.get("tcu_motion_analog"), 0x20)
        self.assertEqual(dec.get("tcu_tick"), 0x55)

        # 0x113: Trip distance and fuel
        dec = parse_frame(0x113, [0, 50, 0, 0, 0x03, 0xE8])
        self.assertEqual(dec.get("trip_km"), 100.0)
        self.assertEqual(dec.get("trip_fuel_l"), 5.0)

        # 0x118: ICE km and DTE
        dec = parse_frame(0x118, [0x00, 0x13, 0x88, 0x00, 0x64, 0x01, 0xF4])
        self.assertEqual(dec.get("ice_km"), 500.0)
        self.assertEqual(dec.get("trip_ice_km"), 10.0)
        self.assertEqual(dec.get("dte_km"), 50.0)

        # 0x11A: VIN decode
        dec = parse_frame(0x11A, [0xC1, 0x01, ord('3'), ord('F'), ord('A'), ord('6'), ord('P'), ord('0')])
        self.assertEqual(dec.get("vin_chunk"), 1)
        self.assertEqual(dec.get("vin_ascii"), "3FA6P0")

    def test_electrical_and_body_frames(self):
        # 0x141: MAP and motor coil temp
        dec = parse_frame(0x141, [39, 0x27, 0x10, 0, 50, 0x80])
        self.assertEqual(dec.get("motor_coil_c"), None)  # d[2] is 0x10 < 40
        self.assertEqual(dec.get("hv_i_gauge_a"), 0)
        self.assertEqual(dec.get("map_kpa"), 100.0)
        self.assertEqual(dec.get("pct_0_100"), 50)
        self.assertEqual(dec.get("b5_mode"), 2)

        # 0x142: Ambient temp & ICE status
        dec = parse_frame(0x142, [84, 0x10, 0x68])
        self.assertEqual(dec.get("ambient_c"), 20.0)
        self.assertEqual(dec.get("raw7"), 8)
        self.assertTrue(dec.get("ice_running"))

        # 0x146: TCU command
        dec = parse_frame(0x146, [0x0C, 0, 0, 42])
        self.assertEqual(dec.get("tcu_cmd"), "LOCK")
        self.assertEqual(dec.get("seq"), 42)

        # 0x147: Remote start and lamp mode
        dec = parse_frame(0x147, [0, 0x02, 0x58, 0, 0, 0x20])
        self.assertEqual(dec.get("remote_start_s"), 600)
        self.assertEqual(dec.get("lamp_mode"), "LOW_BEAM")

        # 0x153: Trip MPG and L/100km
        dec = parse_frame(0x153, [0, 0, 0, 0, 0x01, 0xF4])  # 500 * 0.1 = 50.0 mpg
        self.assertEqual(dec.get("trip_mpg"), 50.0)
        self.assertEqual(dec.get("trip_l100km"), 4.7)

        # 0x15E: Clock UTC / Date UTC
        dec = parse_frame(0x15E, [0x01, 10, 30, 45, 15, 6])
        self.assertEqual(dec.get("clock_utc"), "10:30:45")
        self.assertEqual(dec.get("date_utc"), "15-06")

        # 0x174: Fuel sender pct
        dec = parse_frame(0x174, [0, 0x02, 0x58, 0x80])
        self.assertEqual(dec.get("fuel_sender_pct"), 60.0)
        self.assertEqual(dec.get("fuel_pct"), 60.0)
        self.assertTrue(dec.get("fuel_gauge_update"))

        # 0x175: Steering wheel angle
        dec = parse_frame(0x175, [0, 0, 0, 0x20, 0x00])  # 8192 -> 0.0 deg
        self.assertEqual(dec.get("steer_deg"), 0.0)

        # 0x1B5: TPMS
        dec = parse_frame(0x1B5, [0, 230, 0, 235, 0, 240, 0, 245])
        self.assertEqual(dec.get("tpms_bar"), [2.3, 2.35, 2.4, 2.45])

        # TCU and NM frames
        self.assertTrue(parse_frame(0x22A, [0, 0, 0, 0, 0, 0, 0, 1]).get("tcu_active"))
        self.assertEqual(parse_frame(0x27C, [0x16, 0x60]).get("tcu_27c"), "CMD")
        self.assertEqual(parse_frame(0x27C, [0x00, 0x00]).get("tcu_27c"), "IDLE")
        self.assertEqual(parse_frame(0x590, [0]).get("nm"), "TCU_WAKE")
        self.assertEqual(parse_frame(0x59E, [0]).get("nm"), "GWM_NM")


if __name__ == "__main__":
    unittest.main()
