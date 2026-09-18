# Factory TCU Remote Command Protocol & Timing

> **Bus**: HS-CAN4 (HS3) @ 500 kbit/s  
> **Topology**: Two-node link between factory TCU and Gateway Module (GWM).  
> **Vehicle**: 2018–2019 Ford Fusion Hybrid (CD4).

This document details the reverse-engineered message formats, rolling authentication token, sequence tracking, factory timing schedule, and validation verdict rules for executing remote vehicle commands (Door Lock, Door Unlock, Remote Engine Start, Remote Engine Stop).

---

## 1. Remote Command Architecture

When an owner issues a lock or start command via cellular telematics, the TCU does not broadcast raw actuator pulses or BCM internal bus commands. Instead, it sends a high-level **telematics request frame** (`0x146`) over the HS-CAN4 link to the GWM, armed by state transitions on `0x22A` and `0x27C`. The GWM bridges and translates this request to the BCM and powertrain modules across other vehicle networks.

```text
[TCU] ──(HS-CAN4 500k)──> [GWM] ──(Internal Body/PT)──> [BCM / PCM]
       0x146 command
       0x22A arming
       0x27C state
```

---

## 2. Frame Formats & Encodings (`0x146`)

The primary command frame is CAN ID **`0x146`** (8 data bytes).

### 2.1 Door Commands (Lock / Unlock)

Door actuation uses **Byte 3** for the sequence counter:

```text
Byte 0: Command ID
         0x0C = Door Lock
         0x04 = Door Unlock
Byte 1: 0x00
Byte 2: 0x00
Byte 3: Sequence counter (seq)
Byte 4: 0x00
Byte 5: 0x00
Byte 6: Token high byte (tok_hi)
Byte 7: Token low byte  (tok_lo)
```

Example payload (Unlock, seq `0x42`, token `0x1234`):  
`04 00 00 42 00 00 12 34`

### 2.2 Power Commands (Remote Start / Stop)

Powertrain remote start/stop uses **Byte 5** for the sequence counter:

```text
Byte 0: Command ID
         0x01 = Remote Engine Start
         0x02 = Remote Engine Stop
Byte 1: 0x00
Byte 2: 0x00
Byte 3: 0x00
Byte 4: 0x00
Byte 5: Sequence counter (seq)
Byte 6: Token high byte (tok_hi)
Byte 7: Token low byte  (tok_lo)
```

Example payload (Start, seq `0x17`, token `0x1234`):  
`01 00 00 00 00 17 12 34`

---

## 3. Rolling Authentication Token (`0x100`)

To protect commands against blind injection, the BCM broadcasts a rolling 16-bit challenge on **`0x100`** (Bytes 0–1, Big-Endian) approximately every 800 ms.

### The Sleep-Token Trap (Root Cause Analysis)

> [!CAUTION]
> **Do not overwrite your stored token with zeros during bus sleep.**
>
> When the vehicle enters sleep mode, `0x100` continues to broadcast periodic sleep heartbeats containing `00 00` in Bytes 0–1. If an implementation blindly saves the latest received `0x100` bytes, the valid rolling token is wiped and replaced with zero. Any subsequent wake-and-unlock command will fail silently because the BCM rejects token `00 00`.
>
> **The Rule**: Always filter incoming `0x100` frames and record only the **last non-zero token**.

```rust
// Proper token update pattern:
if let Some(token) = hs3_decode::decode_bcm_token(&data) {
    if token != 0 {
        last_valid_token.store(token, Ordering::Relaxed);
    }
}
```

---

## 4. Sequence Counter & Replay Protection

The BCM strictly enforces replay protection on the sequence byte (`seq`):
- Door commands maintain a dedicated sequence counter.
- Power commands maintain an independent sequence counter.
- The sequence counter must increment monotonically ($+1$) for every command pulse.
- **Replay failure**: If the BCM receives a command frame with a sequence number equal to or lower than one it has previously accepted, the command is ignored silently.
- **Persistence requirement**: Implementations must store the latest sequence counter in non-volatile storage (NVS / EEPROM / flash) so that power cycles or reboots do not reset the counter to zero.

---

## 5. Factory Command Sequence Schedule

Live captures of factory FordPass interactions reveal that actuation is not a single isolated CAN frame. The factory TCU executes a multi-stage timed sequence:

```text
Time (offset)     ID       Payload                          Role
──────────────────────────────────────────────────────────────────────────────────────────
t - 3.2 s to 0 s  0x22A    01 01 00 01 (bytes 4-7)          Command-active flag raised early (~4 cycles @ 800ms)
                  0x27C    00 0C 00 00 00 00 00 00          TCU status idle
                  0x146    00 00 00 00 00 00 00 00          Command bus idle

t = 0.0 s         0x146    [cmd  00 00 seq 00 00 tok_h tok_l]  EXACTLY ONE command pulse
                  0x27C    16 60 00 12 00 00 00 00          Flips from idle to active command state

t = +0.2 s        0x22A    01 01 00 01                      Active follow-up

t = +1.0 s to     0x146    [00   00 00 seq 00 00 tok_h tok_l]  Hold frames (~12 cycles @ 800ms):
    +10.0 s                                                 cmd byte cleared, seq & token held
                  0x27C    16 60 00 12 00 00 00 00          Held in command mode

t = +10.8 s       0x146    00 00 00 00 00 00 00 00          Return to idle
                  0x22A    00 00 00 00 00 00 00 01          Command-active flag lowered
                  0x27C    00 0C 00 00 00 00 00 00          Return to idle
```

In the Rust library, this exact timeline is generated by:
- `hs3_tcu::factory_door_sequence(cmd, auth)`
- `hs3_tcu::factory_power_sequence(cmd, auth)`

### Historical Note: The Fast Burst Experiment
During early reverse-engineering, a 5-frame burst spaced at 40 ms was tested. While occasionally successful on a warm, active bus, it does not match factory TCU signaling, frequently fails from sleep, and has been omitted from the public protocol library in favor of the factory schedule.

---

## 6. Presence Sets vs Gateway Impersonation

The factory TCU transmits periodic node descriptors and presence frames. In a replacement integration:
- Sourced TCU IDs may be broadcast: `0x112`, `0x146` (idle), `0x22A`, `0x27C`, `0x590`.
- **GWM-owned IDs must NEVER be broadcast by the TCU**:
  - `0x1B3` (BCM status)
  - `0x1B4` (BCM config)
  - `0x1B5` (TPMS)
  - `0x1B8` (Airbag status)
  - `0x59E` (Gateway NM)
  Transmitting on GWM-owned IDs causes bus contention, arbitration collision, and DTC faults. The unit test `test_presence_frames_no_gwm_leakage` in `hs3-tcu` strictly validates that none of these IDs exist in the presence set.

---

## 7. Command Validation Verdict Rule

> [!IMPORTANT]
> **Hearing acoustic door latch clicks is not a valid scientific verdict.**
>
> 1. **Ground truth verdict**: The sole authoritative source of door lock state on this tap is **`0x1B3` Byte 0 Bit 2**:
>    - Bit 2 = `0` (`0x10` in Byte 0): **Doors Locked**
>    - Bit 2 = `1` (`0x14` in Byte 0): **Doors Unlocked**
>
> 2. **Auto-Relock Window**: The CD4 BCM features an automatic re-lock security function (~25–30 seconds). If the doors are unlocked remotely and no door is physically opened, the BCM automatically re-locks the vehicle.
>
> 3. **Validation Procedure**: Never test a lock command on an already-locked car. Always:
>    - Verify `0x1B3` Bit 2 before command transmission.
>    - Transmit the command schedule.
>    - Sample incoming `0x1B3` frames across the hold period to observe the bit transition.
