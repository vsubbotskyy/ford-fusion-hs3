"""hs3_decode: Ford Fusion Hybrid (CD4) HS-CAN4 protocol decoders and byte map."""

from .byte_map import BYTE_MAP, MUX_MAP
from .parser import PARSE_CLASS, parse_frame, main

__all__ = ["BYTE_MAP", "MUX_MAP", "PARSE_CLASS", "parse_frame", "main"]
__version__ = "0.1.0"
