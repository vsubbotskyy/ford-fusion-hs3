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


if __name__ == "__main__":
    unittest.main()
