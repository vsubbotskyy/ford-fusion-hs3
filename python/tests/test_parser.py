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
