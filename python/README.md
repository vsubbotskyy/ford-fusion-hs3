# hs3-decode (Python)

Reverse-engineered decoders and comprehensive byte map for the Ford Fusion Hybrid (CD4) HS-CAN4 (factory TCU tap).

## Installation

```bash
pip install .
```

## CLI Usage

Parse and validate a SavvyCAN CSV capture:

```bash
hs3-parse path/to/capture.csv
```

Or run via module:

```bash
python -m hs3_decode.parser path/to/capture.csv
```

## Library Usage

```python
from hs3_decode import parse_frame, BYTE_MAP

# Decode a 0x107 vehicle speed frame
frame = parse_frame(0x107, [0x15, 0x7C, 0xE0, 0x00, 0x00, 0x00, 0x00, 0x00])
print(frame["speed_kmh"])  # 55.0 km/h
```
