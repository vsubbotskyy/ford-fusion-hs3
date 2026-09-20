#!/usr/bin/env python3
"""Parse every HS3-CAN frame in a SavvyCAN CSV using the verified decoder set."""
from __future__ import annotations

import argparse
import csv
import sys
from collections import Counter, defaultdict
from pathlib import Path

try:
    from .byte_map import BYTE_MAP, MUX_MAP
except (ImportError, ValueError):
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from byte_map import BYTE_MAP, MUX_MAP
PARSE_CLASS = {
    0x07A: "engineering",
    0x084: "engineering",
    0x100: "engineering",
    0x101: "engineering",
    0x102: "static",
    0x103: "engineering",
    0x104: "engineering",
    0x105: "enum",
    0x106: "engineering",
    0x107: "engineering",
    0x108: "engineering",
    0x109: "engineering",
    0x10A: "enum",
    0x10C: "engineering",
    0x10E: "engineering",
    0x10F: "raw",
    0x110: "raw",
    0x112: "enum",
    0x113: "engineering",
    0x118: "engineering",
    0x11A: "engineering",
    0x122: "static",
    0x127: "static",
    0x141: "engineering",
    0x142: "engineering",
    0x146: "tcu-cmd",
    0x147: "engineering",
    0x152: "static",
    0x153: "engineering",
    0x15E: "mux",
    0x172: "static",
    0x173: "static",
    0x174: "engineering",
    0x175: "engineering",
    0x1B3: "engineering",
    0x1B4: "static",
    0x1B5: "engineering",
    0x1B8: "static",
    0x212: "static",
    0x215: "static",
    0x21E: "static",
    0x22A: "tcu-cmd",
    0x232: "static",
    0x246: "uds",
    0x24E: "uds",
    0x267: "static",
    0x27B: "static",
    0x27C: "tcu-cmd",
    0x27D: "static",
    0x27E: "static",
    0x27F: "static",
    0x28A: "static",
    0x28B: "static",
    0x28D: "static",
    0x28E: "static",
    0x2B7: "uds",
    0x2BF: "uds",
    0x590: "nm",
    0x59E: "nm",
    0x720: "uds",
    0x728: "uds",
    0x760: "uds",
    0x768: "uds",
    0x7DF: "uds",
    0x7E2: "uds",
    0x7E8: "uds",
    0x7EA: "uds",
}


def be16(d, o=0):
    return (d[o] << 8) | d[o + 1]


def be24(d, o=0):
    return (d[o] << 16) | (d[o + 1] << 8) | d[o + 2]


def parse_frame(cid: int, d: list[int]) -> dict:
    out = {"id": cid, "class": PARSE_CLASS.get(cid, "unknown")}
    if cid == 0x07A and len(d) >= 4:
        raw_i = ((d[0] & 0x7F) << 8) | d[1]
        out["hv_current_a"] = round(raw_i * 0.05 - 750.0, 2)
        out["hv_voltage_v"] = round(((d[2] & 0x03) << 8 | d[3]) * 0.5, 1)
        if len(d) >= 6:
            out["hv_vmax_v"] = d[4] * 2
            out["hv_vmin_v"] = d[5] * 2
    elif cid == 0x084 and len(d) >= 7:
        out["clock"] = f"{d[6]:02d}:{d[4]:02d}:{d[5]:02d}"
        out["calendar_day"] = be16(d, 2)
    elif cid == 0x100 and len(d) >= 2:
        tok = be16(d, 0)
        if tok:
            out["bcm_token"] = tok
    elif cid == 0x101 and len(d) >= 8:
        out["brake_pressed"] = bool(d[4] & 0x80)
        out["pwrtrain"] = {0xE9: "mixed", 0xEA: "EV", 0xC7: "parked"}.get(d[7], f"0x{d[7]:02X}")
    elif cid == 0x103 and len(d) >= 4:
        raw = be16(d, 2)
        if raw > 0xE000:
            out["rpm"] = (raw - 0xE000) * 2.0
            out["engine_running"] = True
            out["ev_mode"] = False
        elif raw == 0xE000:
            out["rpm"] = 0.0
            out["engine_running"] = False
            out["ev_mode"] = True
        else:
            out["rpm"] = 0.0
            out["engine_running"] = False
            out["ev_mode"] = False
        delta = be16(d, 0) - 0x8000
        out["pedal_pct"] = max(0.0, min(100.0, delta / 5.3)) if delta > 0 else 0.0
    elif cid == 0x104 and len(d) >= 3:
        out["coolant_c"] = d[2] - 60.0
        out["alive2"] = d[0] & 3
        if len(d) >= 2:
            out["highrate8"] = d[1]
    elif cid == 0x105:
        out["hybrid_mode"] = {0xE0: "OFF", 0xE8: "CITY", 0xF8: "HIGHWAY"}.get(d[0], f"0x{d[0]:02X}")
        if len(d) >= 7:
            out["hybrid_strategy"] = {0x64: "CITY", 0x34: "HIGHWAY", 0x60: "PARK"}.get(d[6], f"0x{d[6]:02X}")
    elif cid == 0x106 and len(d) >= 2:
        raw = ((d[0] & 0x03) << 8) | d[1]
        if not (d[0] == 0x9F and d[1] == 0xFF):
            out["brake_pct"] = round(min(100.0, raw / 10.23), 1)
        if len(d) >= 4:
            cluster = (d[2] << 8) | d[3]
            out["brake_cluster"] = cluster
            if cluster in (0x0BFE, 0x13FE, 0x1BFE):
                out["brake_cluster_sentinel"] = True
        if len(d) >= 6 and (d[4] >> 2) == 0x3E:
            raw_a = ((d[4] & 3) << 8) | d[5]
            out["long_accel_ms2"] = round(raw_a * 0.035 - 17.9, 2)
        if len(d) >= 8 and d[6] != 0x0F:
            raw12 = ((d[6] & 0x0F) << 8) | d[7]
            if raw12 != 0xFFE:
                out["raw12"] = raw12
    elif cid == 0x107 and len(d) >= 3:
        out["speed_kmh"] = be16(d, 0) * 0.01
        out["gear"] = {0x60: "P", 0xE0: "D/R/N"}.get(d[2])
    elif cid == 0x108:
        pct = d[0] * 0.5
        if 40.0 <= pct <= 100.0:
            out["hybrid_soc_pct"] = pct
    elif cid == 0x109 and len(d) >= 5:
        out["odometer_km"] = be24(d, 0)
        out["fuel_accum"] = be16(d, 3)
        if len(d) >= 7:
            out["odo_companion"] = d[6]
    elif cid == 0x10A and len(d) >= 3:
        out["transaxle"] = {0x39: "P", 0x08: "not-P", 0x09: "not-P-low", 0x38: "stop-38"}.get(d[2], f"0x{d[2]:02X}")
    elif cid == 0x10C and len(d) >= 7:
        out["thermal_live"] = d[3] == 2 and d[4] == 0xB0
        out["b6_live_band"] = d[6]
        out["component_temp_raw"] = d[0]
    elif cid == 0x10E:
        if d[0] == 0x17:
            out["ignition"] = "ON"
        elif d[0] == 0x03 and len(d) >= 8 and d[7] == 0:
            out["ignition"] = "OFF"
        elif d[0] == 0x03:
            out["ignition"] = "ON"
        else:
            out["ignition"] = "STANDBY"
        if len(d) >= 8:
            out["drive_active"] = d[7] != 0
            out["drive_alt"] = bool(d[4] & 0x20)
            out["drive_alt_b5"] = bool(d[5] & 0x04)
            out["slow_analog_b1"] = d[1]
            out["analog_b3"] = d[3]
            out["analog_b6"] = d[6]
            if d[3] == 0:
                out["b3_zeroes_b6"] = d[6] == 0
    elif cid == 0x10F and len(d) >= 2:
        out["raw16"] = be16(d, 0)
    elif cid == 0x110 and len(d) >= 3:
        out["mg_high"] = d[0]
        valid = len(d) < 4 or (d[3] & 0x80) != 0
        if valid and not (d[0] == 0xFF and d[1] == 0xFF and d[2] == 0xFF):
            out["motor_angle_deg"] = round(((d[1] << 8) | d[2]) * 360 / 65536, 1)
    elif cid == 0x112 and len(d) >= 5:
        out["tcu_motion"] = {0x00: False, 0x03: True}.get(d[4])
        if len(d) >= 6:
            out["tcu_motion_analog"] = d[5]
        if len(d) >= 8:
            out["tcu_tick"] = d[7]
    elif cid == 0x113 and len(d) >= 6:
        out["trip_km"] = be16(d, 4) * 0.1
        if len(d) >= 2:
            out["trip_fuel_l"] = round(d[1] * 0.1, 1)
    elif cid == 0x118 and len(d) >= 7:
        out["ice_km"] = be24(d, 0) * 0.1
        out["trip_ice_km"] = be16(d, 3) * 0.1
        out["dte_km"] = be16(d, 5) * 0.1
    elif cid == 0x11A and len(d) >= 8:
        if d[0] == 0xC1:
            out["vin_chunk"] = d[1]
            out["vin_ascii"] = bytes(d[2:8]).decode("ascii", errors="replace")
        elif d[0] == 0xC0:
            out["vin_alt_mux"] = True
    elif cid == 0x141 and len(d) >= 3:
        if d[2] >= 40:
            out["motor_coil_c"] = d[2] - 40
        raw = be16(d, 1)
        if 8000 <= raw <= 12000:
            out["map_kpa"] = raw * 0.01
        else:
            out["map_raw"] = raw
        if 34 <= d[0] <= 45:
            out["hv_i_gauge_a"] = (d[0] - 39) * 24
        if len(d) >= 5 and d[4] <= 100:
            out["pct_0_100"] = d[4]
        if len(d) >= 6:
            out["b5_mode"] = d[5] >> 6
            out["b5_analog6"] = d[5] & 0x3F
    elif cid == 0x142:
        out["ambient_c"] = d[0] - 64.0
        if len(d) >= 2:
            out["raw7"] = d[1] >> 1
        if len(d) >= 3:
            ice = {0x64: False, 0x68: True}.get(d[2])
            if ice is not None:
                out["ice_running"] = ice
    elif cid == 0x146:
        cmd = {0x0C: "LOCK", 0x04: "UNLOCK", 0x01: "REMOTE_START", 0x02: "REMOTE_STOP", 0x00: "IDLE"}.get(d[0], f"0x{d[0]:02X}")
        out["tcu_cmd"] = cmd
        if len(d) >= 4:
            out["seq"] = d[3]
    elif cid == 0x147 and len(d) >= 3:
        out["remote_start_s"] = be16(d, 1)
    elif cid == 0x153 and len(d) >= 6:
        raw = be16(d, 4)
        if 0 < raw < 1000:
            mpg = raw * 0.1
            out["trip_mpg"] = round(mpg, 1)
            l100 = 235.214583 / mpg
            if 1.5 <= l100 <= 30:
                out["trip_l100km"] = round(l100, 1)
    elif cid == 0x15E:
        mux = d[0]
        out["mux"] = mux
        if mux == 0x01 and len(d) >= 6 and d[1] <= 23 and d[2] <= 59 and d[3] <= 59:
            out["clock_utc"] = f"{d[1]:02d}:{d[2]:02d}:{d[3]:02d}"
            out["clock"] = out["clock_utc"]
            if 1 <= d[5] <= 12 and 1 <= d[4] <= 31:
                out["date_utc"] = f"{d[4]:02d}-{d[5]:02d}"
        elif mux in (0x16, 0x76, 0x86) and len(d) >= 7:
            lat_fine = (d[3] - 128) * 1e-6 if len(d) >= 4 else 0.0
            lat = 43.6591 + be16(d, 1) * 0.000256034 + lat_fine
            lon = 5.0 + be16(d, 5) * 0.000016
            out["lat"] = round(lat, 6)
            out["lon"] = round(lon, 6)
    elif cid == 0x174 and len(d) >= 3:
        pct = be16(d, 1) * 0.1
        if 0 <= pct <= 100:
            out["fuel_pct"] = pct
        if len(d) >= 4:
            out["fuel_gauge_update"] = bool(d[3] & 0x80)
    elif cid == 0x175 and len(d) >= 5:
        out["steer_deg"] = (be16(d, 3) - 8192) * 0.1
    elif cid == 0x1B3:
        out["doors_locked"] = not bool((d[0] >> 2) & 1)
        out["trunk_ajar"] = bool(d[0] & 1)
        out["rear_fog"] = bool((d[0] >> 1) & 1)
        if len(d) >= 2:
            out["headlights"] = bool(d[1] & 0x08)
            day_night = (d[1] >> 6) & 0x03
            if day_night in (1, 2):
                out["day_night"] = "DAY" if day_night == 1 else "NIGHT"
        if len(d) >= 6:
            out["ambient_light"] = d[5]
        if len(d) >= 8:
            left = bool(d[1] & 0x01)
            right = bool((d[7] >> 6) & 0x01)
            out["turn_left"] = left
            out["turn_right"] = right
            out["hazards"] = left and right
            out["turn_signal"] = (
                "HAZARD" if left and right
                else "LEFT" if left
                else "RIGHT" if right
                else "OFF"
            )
            out["turn_signal_active"] = left or right
            left_bulb = bool((d[1] >> 1) & 1)
            right_bulb = bool((d[7] >> 7) & 1)
            out["flasher_bulb_on"] = left_bulb or right_bulb
            out["courtesy_flash"] = left_bulb and right_bulb and not (left and right)
            out["front_fog"] = bool(d[7] & 1)
            out["hood_ajar"] = bool((d[7] >> 3) & 1)
            out["door_ajar_fl"] = bool((d[7] >> 5) & 1)
            out["door_ajar_fr"] = bool((d[7] >> 4) & 1)
            out["door_ajar_rl"] = bool(d[6] & 1)
            out["door_ajar_rr"] = bool((d[6] >> 1) & 1)
    elif cid == 0x1B5 and len(d) >= 8:
        out["tpms_bar"] = [round(d[i] * 0.01, 2) for i in (1, 3, 5, 7)]
    elif cid == 0x22A and len(d) >= 8:
        out["tcu_active"] = d[7] == 1
    elif cid == 0x27C:
        out["tcu_27c"] = "CMD" if d[:2] == [0x16, 0x60] else "IDLE"
    elif cid == 0x590:
        out["nm"] = "TCU_WAKE"
    elif cid == 0x59E:
        out["nm"] = "GWM_NM"
    return out


def load_csv(path: Path):
    frames = []
    with open(path, newline="", encoding="utf-8-sig") as f:
        r = csv.reader(f)
        next(r)
        for row in r:
            if len(row) < 8:
                continue
            try:
                ts = float(row[0])
                cid = int(row[1], 16)
            except ValueError:
                continue
            dlc = int(row[5]) if row[5].strip() else 8
            data = []
            for j in range(6, 6 + min(dlc, 8)):
                data.append(int(row[j], 16) if j < len(row) and row[j].strip() else 0)
            while len(data) < 8:
                data.append(0)
            frames.append((ts, cid, data))
    return frames


def main():
    ap = argparse.ArgumentParser(description="Parse every recorded HS3-CAN frame")
    ap.add_argument("canlog", nargs="?", default=str(Path(__file__).resolve().parents[1] / "can logs" / "2026-09-09 06-28-10.csv"))
    ap.add_argument("--dump", action="store_true", help="print decoded non-static frames")
    ap.add_argument("--limit", type=int, default=0)
    args = ap.parse_args()
    path = Path(args.canlog)
    frames = load_csv(path)
    if args.limit:
        frames = frames[: args.limit]
    print(f"file {path.name}  frames={len(frames)}")

    ids = Counter(cid for _, cid, _ in frames)
    unknown = []
    parsed = 0
    by_class = Counter()
    samples = defaultdict(list)
    for ts, cid, data in frames:
        cls = PARSE_CLASS.get(cid)
        if cls is None:
            unknown.append(cid)
            by_class["unknown"] += 1
            continue
        by_class[cls] += 1
        parsed += 1
        dec = parse_frame(cid, data)
        if len(samples[cid]) < 3:
            keys = {k: v for k, v in dec.items() if k not in ("id", "class")}
            if keys:
                samples[cid].append(keys)

    print(f"unique IDs {len(ids)}  parsed_frames {parsed}/{len(frames)}")
    print("class counts:", dict(by_class))
    if unknown:
        u = sorted(set(unknown))
        print("UNPARSED IDs:", [f"0x{c:03X}" for c in u])
        sys.exit(1)

    print("\nID coverage:")
    for cid in sorted(ids):
        cls = PARSE_CLASS[cid]
        extra = samples.get(cid, [])
        shown = f"  e.g. {extra[0]}" if extra else ""
        print(f"  0x{cid:03X}  n={ids[cid]:6d}  {cls:12s}{shown}")

    missing_bytes = []
    unverified = []
    for cid in sorted(ids):
        rows = BYTE_MAP.get(cid)
        if not rows or len(rows) != 8:
            missing_bytes.append(cid)
            continue
        for b, (role, meaning) in enumerate(rows):
            if role == "unverified":
                unverified.append((cid, b, meaning))
    if missing_bytes:
        print("MISSING BYTE MAP:", [f"0x{c:03X}" for c in missing_bytes])
        sys.exit(1)
    print(f"\nbyte map: 8/8 for {len(ids)} IDs; unverified bytes {len(unverified)}")
    for cid, b, meaning in unverified:
        print(f"  UNVERIFIED  0x{cid:03X} B{b}  {meaning}")
    mux_u = []
    if 0x15E in ids:
        seen_mux = set()
        for _, cid, data in frames:
            if cid == 0x15E and data:
                seen_mux.add(data[0])
        maps = MUX_MAP.get(0x15E, {})
        missing_mux = sorted(m for m in seen_mux if m not in maps)
        for mux in sorted(seen_mux):
            rows = maps.get(mux)
            if not rows:
                continue
            for b, (role, meaning) in enumerate(rows):
                if role == "unverified":
                    mux_u.append((mux, b, meaning))
        print(f"0x15E mux maps for {len(seen_mux & set(maps))} seen muxes; unverified mux-bytes {len(mux_u)}")
        for mux, b, meaning in mux_u:
            print(f"  UNVERIFIED  0x15E mux {mux:02X} B{b}  {meaning}")
        if missing_mux:
            print("  UNMAPPED 0x15E muxes:", [f"{m:02X}" for m in missing_mux])


    if args.dump:
        print("\n--- dump ---")
        for ts, cid, data in frames:
            if PARSE_CLASS[cid] in ("static", "nm"):
                continue
            dec = parse_frame(cid, data)
            keys = {k: v for k, v in dec.items() if k not in ("id", "class")}
            if keys:
                print(f"{ts:10.3f}  0x{cid:03X}  {keys}")

    print("\nOK: every ID in this capture has a parser.")


if __name__ == "__main__":
    main()
