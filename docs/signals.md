# Confirmed CAN findings — 2019 Ford Fusion Hybrid, TCU bus
**Status: verified against labelled captures and live car tests. Updated 2026-09-14.**
**CAN transmit and receive proven working on the vehicle. Door LOCK and Door UNLOCK verified on-car. Remote Start/Stop verified from FordPass captures. See §0a.**

2017+ Fusion Hybrid (C1MCA / CD4) has a Gateway Module. The factory TCU connector is
HS-CAN4 @ 500 kbit/s (this project also calls it HS3): a two-node link, TCU ↔ GWM.
The modem talks to the GWM, not to the BCM. Aftermarket "CGEA 1.2" (no separate GWM)
does not describe this car.

Everything here is derived from captures taken with CAN logging tools at the factory TCU
harness, with the factory TCU installed and functioning, plus live vehicle testing.
Confidence is stated per finding.

> [!NOTE]
> **Master Bus Census**: All **63** HS3 CAN IDs recorded on this vehicle have a parser class. Per-byte roles are in `python/hs3_decode/byte_map.py` — see [bus-census.md](bus-census.md) and `python/hs3_decode/parser.py`.
>
> **Next RE** is [§ Next steps for RE](#next-steps-for-re--2026-09-14) below. Byte roles assigned ≠ every unused bit named, and ≠ every name is the physics.

---

## Next steps for RE — 2026-09-14

The TCU tap is **HS3 only**: a gatewayed telematics slice, not MS-CAN / BCM body and not HS1 powertrain. All 63 recorded IDs have a parser. Remaining work is unused bits, misnamed analogs, and signals the GWM never copied onto this bus.

Public DBC (comma [`ford_lincoln_base_pt.dbc`](https://github.com/commaai/opendbc/blob/master/opendbc/dbc/ford_lincoln_base_pt.dbc)) is a **name list for bits that already toggle here**. Do not paste `BodyInfo_3_FD1` (`0x3B3`, HS1) onto `0x1B3`. Door/hood/trunk/fog bits lined up; ignition, key-in, turn L/R, dimming, and park-brake names occupy bits that are already something else on HS3 (that is how fake turn-side happened).

### Closed on `0x1B3` — do not re-hunt

| Panel / lamp | Byte.bit | JSON | Evidence |
|---|---|---|---|
| FL | B7 bit 5 | `door_ajar_fl` | `doors-window.csv` |
| FR | B7 bit 4 | `door_ajar_fr` | same |
| RL | B6 bit 0 | `door_ajar_rl` | user-confirmed 2026-09-14; matches Ford `DrStatRl` bit 48 |
| RR | B6 bit 1 | `door_ajar_rr` | user-confirmed 2026-09-14; matches Ford `DrStatRr` bit 49 |
| Trunk | B0 bit 0 | `trunk_ajar` | `trunc.csv` |
| Hood | B7 bit 3 | `hood_ajar` | `hood.csv` |
| Front fog | B7 bit 0 | `front_fog_on` | lights-fogs captures |
| Rear fog | B0 bit 1 | `rear_fog_on` | only latches with front fog |
| DRL / lighting master | B1 bit 3 | `headlights_on` | AUTO/dipped/high are **not** separate on HS3 |
| Unlock | B0 bit 2 | `doors_locked` inverted | §3 |
| Turn active / bulb | B1 bit 0 / bit 1 | `turn_signal_active` / `flasher_on` | side is **not** here |

Windows and sunroof: nothing on HS3 (`doors-window.csv`, `sunroof.csv`). Do not add UI for them from this tap.

### Priority on this tap

1. **Motor coil** — `0x141` B2−40 vs OBD Motor Coil (r=1.000, MAE=0.05 °C on `2026-09-10 14-05-10`). Replay morning `2026-09-09 06-28-10` at +66.5 s lag. If it repeats, publish and keep `decode_motor_temp` off `0x10C`. See [Open RE — `0x10C`](#open-re--0x10c-motor-temp-2026-09-13).
2. **Re-home MAP** — `0x141` B1 rolls 0–255; old B1:B2 MAP is likely the counter. Find the real MAP byte before swapping names.
3. **Identify `0x10C` B6** — live band 64–77, not coil temp. Leave unpublished. Same for `0x10C` B0 soak analog.
4. **Cluster low-fuel lamp** — empty-tank capture (SavvyCAN + photo of the tick). `0x174` B3 bit 7 is anti-slosh, not the lamp. Hunt `0x720`/`0x728`, `0x118` DTE, `0x174` B4–B7, `0x172`/`0x173`. See [Open RE — `0x174`](#open-re--0x174-fuel-frame-2026-09-13).
5. **`0x174` B4–B7** — checksum / life counter / second sender. Needs two fills and a long park, not more highway.
6. **Unused `0x1B3` bits** — labeled captures only. Cheap candidates vs Ford names, *after* checking the bit is not already claimed: park brake, park lamps, day/night. Turn L/R on this frame is closed (dead).
7. **Turn side** — labeled left-only vs right-only vs the rest of HS3. Not `0x1B3` B6 bit 6 (flash phase). See [Open RE — turn signal side](#open-re--turn-signal-side-2026-09-13).
8. **`0x10F` / `0x110`** — high-rate 16-bit. Do not chart raw `0x110` angle (Grafana hidden 2026-09-15). RPM/direction plan: [Open RE — `0x110` motor RPM](#open-re--0x110-motor-rpm).

### Not on HS3 — stop unless a new tap

| Want | Where it actually lives | Notes |
|---|---|---|
| Window position / motion | MS-CAN `Driver_Dr_Stat` / `Pass_Dr_Stat` (DBC 818/819) | GWM → HS1 for ADAS, not TCU |
| HVAC compressor / setpoint | MS-CAN (e.g. DBC 806 `Compressor_Req`) | dedicated HS3 captures were empty |
| Sunroof | body / MS-CAN | `sunroof.csv` unchanged on HS3 |
| Reverse / Neutral | not this bus | HS3 has Park vs not-Park only (`0x107` / `0x10A`) |
| Oil life | FordPass / TCU snapshot | do not invent from HS3 |
| Full `BodyInfo_3` (key-in, ignition nibble, dimming) | HS1 `0x3B3` | compact remix on `0x1B3` only |

New body/comfort data means tapping **MS-CAN (125k)** or **HS1**, not more TCU logs.

### Rules

- Do not publish a MQTT/Grafana field until a labelled capture proves the bit/byte.
- Do not copy Ford bit names onto `0x1B3` because the DBC says so — verify, then name.
- Do not poll `0x7E0` from HS3. Do not reintroduce fake oil-life / UDS UI.
- Do not flash unless asked. RL/RR decoder swap (2026-09-14) is firmware-only; containers unchanged.

---

## 0a. The door-command sequence — VERIFIED on-car (Lock & Unlock Working!)

**Lock and Unlock BOTH confirmed working on the vehicle.**
Live testing successfully unlocked the vehicle on on-car test (`seq=0xFB, token=0xA107`), with `0x1B3` transitioning bit 2 to 1 (`doors_locked: false`).

### The factory sequence, measured

Identical in `lock-A.csv`, `unlock-A.csv`, `unlock-2.csv`, `unlock-3.csv`, and `cold-unlock-fordpass.csv`
(t = the single command frame):

| time | ID | payload | note |
|---|---|---|---|
| t−3.0 s | `0x22A` | `00 00 00 00 01 01 00 01` | TcuCmd_Active raised **three seconds early** |
| t+0.0 s | `0x146` | `<cmd> 00 00 <seq> 00 00 <tok_hi> <tok_lo>` | **exactly ONE frame** |
| t+0.2 s | `0x22A` | `00 00 00 00 01 01 00 01` | still active |
| t+0.8 s | `0x146` | `00 00 00 <seq> 00 00 <tok_hi> <tok_lo>` | byte0 cleared, **seq held**, ~1 Hz |
| t+1.0 s | `0x22A` | `00 00 00 00 01 01 00 00` | active flag drops |
| …+10 s | `0x146` | `00 00 00 <seq> 00 00 <tok_hi> <tok_lo>` | held frame repeats |
| t+10 s | `0x146` | `00 00 00 00 00 00 00 00` | back to idle |

Command bytes (byte0), confirmed by capture **and** by the factory app performing the action:

| byte0 | action | Sequence byte | Token bytes (6-7) |
|---|---|---|---|
| `0x0C` | LOCK — **verified on-car, doors locked** | Byte 3 | `0x100` BCM counter |
| `0x04` | UNLOCK — **verified on-car, doors unlocked** | Byte 3 | `0x100` BCM counter |
| `0x01` | REMOTE START — **verified in FordPass capture** | Byte 5 | `0x100` BCM counter |
| `0x02` | REMOTE STOP — **verified in FordPass capture** | Byte 5 | `0x100` BCM counter |

### The 0x146 Authentication Token (Bytes 6-7) — Mystery Solved!
Early captures showed `DA B3` in bytes 6-7 because that happened to be the live counter during that recording.
Bytes 6-7 are the **BCM Rolling Counter** broadcast by the vehicle on `0x100` bytes 0-1 (big-endian).
- When awake, `0x100` increments ~every 800ms.
- When the car enters cold sleep, `0x100` outputs `00 00`!
- The BCM remembers the **last known non-zero counter** from before sleep.
- Commands sent from cold sleep (`cold-unlock-fordpass.csv`, `remote-start-fordpass.csv`, `remote-stop-fordpass.csv`) **must use this last non-zero token**.
- If the token is `00 00` or stale (e.g. from an earlier drive cycle before reboot), the BCM rejects the command!

### Remote UNLOCK — SOLVED (was "gated"; the gate was our command shape) — updated 2026-09-12

On-car result after every other variable was eliminated (correct 0x146 encoding,
fresh sequence number, full 22-frame TCU impersonation running):

- **LOCK** (`0x146` byte0 `0C`): the BCM produces the ack transient (`0x1B3` byte1
  bit1 + byte4 bit3 pulse). It engages with our frame.
- **UNLOCK** (`0x146` byte0 `04`): ✅ **SOLVED — verified on-car.** The
  factory TCU's unlock is *not* a different encoding — it is a different *shape
  and wait*, plus the **BCM rolling token** in bytes 6-7 (see above). With the
  factory sequence hard-wired and a live token, the BCM acks and the doors open.
  Gemini's "v2 / 01 01 is the fix / SOLVED" write-up was **wrong about the mechanism**:
  `door v2` is still the short 5×40 ms burst (only the 0x22A marker bytes change),
  the firmware default was never switched, and `unlock-2.csv` unlocks with
  `0x1B3` byte6 = `00`, which kills the "byte6=02 gate" theory.

> **The remaining gate is REACHABILITY, not CAN.** Unlock works whenever the board is
> actually listening. *Whether it is listening* is the open problem, and it is a
> connectivity problem, not a reverse-engineering one — see HANDOVER §4.

### Factory unlock-from-sleep — measured, not guessed

`unlock-A.csv` and `unlock-3.csv` are the asleep-car captures. Recording was
already running (first timestamps 14.7 s / 21.8 s); the bus is silent; then the
TCU cluster appears as the first frames. That *is* the sleep→wake unlock. They
are clockwork-identical:

| t (from first TCU frame) | ID | payload | note |
|---|---|---|---|
| 0.00 s | TCU cluster + `0x590` | `90 00 FF FF FF 00 FF FF` | AUTOSAR NM wake |
| 0.00 s | `0x22A` | `00 00 00 00 01 01 00 00` | idle, factory marker |
| 0.00 s | `0x27C` | `00 0C 00 00 00 00 00 00` | TCU idle |
| +1.04 s | `0x59E` + first `0x1B3` | `10 .. 02 00` LOCKED | GWM/BCM is up |
| +1.94 s | `0x22A` | `00 00 00 00 01 01 00 01` | TcuCmd_Active, **3.000 s before the pulse** |
| +4.95 s | `0x146` | `04 00 00 <seq> 00 00 DA B3` | **exactly one frame** |
| +4.95 s | `0x27C` | `16 60 00 12 00 00 00 00` | flips at the same instant |
| +5.15 s | `0x1B3` | byte0 `10`→`14` | UNLOCKED |
| +5.25 s | `0x1B3` | byte1 bit1 + byte4 bit3 | ack transient |
| +5.75 s | `0x146` | `00 00 00 <seq> 00 00 DA B3` | hold, ~1 Hz for ~10 s |

Our default `unlock` was the v1 short burst: wake (or skip if traffic already
flows) → 50 ms → **five** command frames. That is why "frames start flowing"
and the BCM still ignores unlock — the bus is awake, the *command shape* is
not the factory one. Lock tolerates the sloppy burst. Unlock does not.

There is **no seed-key / extra ID** unique to unlock in any capture. The factory
TCU itself only sends `0x146` byte0=`04` plus the timing above. Static replay
can work; we were not replaying.

`door v4` was the only firmware switch that even approximated this, and it was
never the default. `unlock` now hard-wires this factory path (wait for a fresh
`0x1B3`, 3.0 s of `0x22A` `01 01` active, one pulse, `0x27C` flip, hold).


### The sequence byte is REPLAY-PROTECTED — verified on-car 2026-09-06

**The BCM rejects any 0x146 command whose byte3 it has already seen.** This is the
single most important finding about these commands, and it masqueraded as an encoding
problem for hours.

| seq | first use | reused later |
|---|---|---|
| `D7` | ✅ accepted, doors moved | ✗ rejected |
| `D8` | ✅ accepted, doors moved | ✗ rejected |
| `D9` | (unlock attempt) | ✗ rejected |
| `D4` `D5` `D6` | — | ✗ rejected |
| `E0` | ✅ accepted, doors moved | — |

`LOCK_SEQ` was initialised to `0xD4` at every boot. The counter had reached `0xDB`
before a reflash, so every command issued afterwards carried a value at or below one
the BCM had already accepted — and `D7`, `D8` and `D9` were replayed verbatim. Lock
appeared to "break" when the firmware changed; in fact it broke when the counter went
backwards. Jumping it with `seq E0` restored lock immediately, with no other change.

**Consequence:** the counter MUST be persistent. It now lives in NVS and is written
after every command. A firmware change that resets it will silently disable every
door command, with the frames still reporting as delivered and ACKed.

Open question: byte3 is 8-bit and wraps every 256 commands. Whether the BCM tolerates
a wrap — i.e. whether it enforces "not seen recently" or strict monotonic increase —
is **untested**.

### What we were doing wrong

The old `tcu_door_command` sent the command byte **five times at 40 ms**, with a 50 ms lead
and no hold phase, finishing in 200 ms. Lock tolerated that. Unlock did not.

### 0x22A bytes 4 and 5 are `01 01`, not `00 00`

Every one of the **897** factory `0x22A` frames across all captures is
`[00 00 00 00 01 01 00 <flag>]`. There is no exception. We had been sending `00` in bytes
4 and 5, so our keepalive never looked quite like the TCU's.

`capture0-tcu-unplugged.csv` contains **zero** `0x146` command frames and **zero** `0x22A`
frames, which independently confirms both IDs are TCU-originated and safe for us to source.

### Key-fob operations do not use 0x146

`lock-keyfob.csv` has 46 `0x146` frames and every one is all-zero. The fob path is RF →
BCM directly; `0x146` is the *remote/telematics* command channel only. So a fob capture can
never be used to reverse-engineer these commands — only app/TCU captures can.

---

---

## 0. ⚠️ THE ROOT CAUSE — the TWAI driver was on an unconnected pin  ✅ RESOLVED 2026-09-06

**Everything below this section was correct all along. The firmware could never act on any
of it, because the CAN controller was transmitting into GPIO19 — a pin wired to nothing.**

The transceiver's D input is on **GPIO22**. The TWAI driver was built on **gpio19**. Every
frame the board ever attempted — `0x22A`, `0x146`, every remote command — went into a dead
pin. No ACK ever came back, TEC climbed to 128, the controller dropped to bus-off, and the
logs reported "Gateway Asleep/Offline". That is the exact behaviour this project opened with.

**Why it hid for months:** `phy_loopback_test()` drives `PIN_CAN_TX`, and that constant said
**22** — the *correct* pin. So the self-test passed while the driver transmitted nowhere.
"PHY PASS but commands don't work" was never a contradiction; it was two different pins. And
because the test passed, it steered every diagnosis toward the transceiver, the termination,
the grounding, the gateway — everything except the driver.

### The evidence that settled it

| `PIN_CAN_TX` | `phy` result | meaning |
|---|---|---|
| 22 | **PASS** — dominant 8/8 | a real 22 → D → R → 21 path exists |
| 19 | **FAIL / NO_DRIVE** — dominant 0/8 | nothing on 19 |

Then `phy scan` swept every candidate on the live board and gave one answer:

```
GPIO19  recessive 8/8, dominant 0/8
GPIO22  recessive 8/8, dominant 8/8   <== TRACKS: this is the D pin
GPIO18  recessive 8/8, dominant 0/8
GPIO23  recessive 8/8, dominant 0/8
GPIO32  recessive 8/8, dominant 0/8
```

### Verified on a two-node bench bus, 2026-09-06

- **Transmit:** `inject 123 1122334455667788` → the frame appeared in SavvyCAN as
  **`Dir = Rx, ID = 0x123`**, with `status` holding `State=1 TEC=0 BusErr=0`.
  **TEC staying at 0 is the proof**: a transmitter that receives no ACK increments TEC by 8
  per attempt, so TEC=0 means another node acknowledged at the bit level.
- **Receive:** `mode listen` with the second node streaming → `frames_rx` climbing.

Both directions working, for the first time in the project's history.

### The fix

`PIN_CAN_TX = 22` **and** the TWAI driver's tx pin set to `gpio22`, bound together by
`const _: () = assert!(PIN_CAN_TX == 22)` beside the pin selection so the self-test and the
driver can never diverge silently again. The `phy` log lines now derive their pin numbers
from the constants instead of hard-coding them.

### A diagnostic worth keeping: `phy scan`

Sweeps GPIO 19/22/18/23/32, driving each and watching the RX pin through the transceiver,
and reports which one the D input actually responds on. Modem pins are excluded. It answers
"which pin is this wire really on?" in one command — no silkscreen, no pin-counting, no
multimeter. It was written precisely because reading colour bands, silkscreen and
recollection had each produced a different, confident, wrong answer.

### Lessons

1. **A passing self-test is only as good as what it tests.** `phy` exercised a pin the
   driver never used, and its green tick actively misdirected diagnosis for weeks.
2. **When a measurement and a recollection disagree, re-measure.** This was diagnosed
   correctly on 2026-09-04, then *reverted* on a memory of which pin had been soldered —
   costing two days. Two electrical readings were overruled by one recollection.
3. **TEC is the ACK oracle.** TEC=0 after a transmit means someone acknowledged it. No
   sniffer needed to know the frame landed.

> ⚠️ **Caveat on attribution:** the transceiver module was replaced in the same session, so
> we cannot claim the old one was healthy. But `phy scan` ran on the *new* chip and still
> showed 19 dead / 22 live — the pin mismatch was real independently of transceiver health.

---

## 1. Door command — `0x146`, byte 0  ✅ VERIFIED

A **momentary one-frame pulse** at the instant the doors actuate. Returns to `0x00`
on the next frame.

| byte 0 | binary | meaning |
|---|---|---|
| `0x04` | `0b00000100` | **UNLOCK** (bit 2) |
| `0x0C` | `0b00001100` | **LOCK** (bit 2 + bit 3) |
| `0x00` | — | idle |

Read either as **bit 2 = "door command executed", bit 3 = lock direction**, or as a
2-bit field at bits[3:2] where `1 = unlock`, `3 = lock`. The data fits both.

> Note vs. the cr08 C-Max research: their `TcuCmd_LockCtrl = 1 = Unlock` **matches**;
> their `2 = Lock` does **not** — this vehicle uses `3`. And on this car the field is
> in **0x146**, not in 0x22A.

---

## 2. It is a TELEMATICS command, not a "doors actuated" broadcast  ✅ VERIFIED

This is the load-bearing test of the whole project, so it gets its own section.

**Key fob test (`lock-keyfob.csv`):** the doors were locked and unlocked with the
physical fob. The door state changed (`0x1B3` flipped, auto-relock followed), proving
the actuation was real — and **`0x146` never pulsed. Not once in the whole capture.**

If `0x146` were the BCM announcing "doors just actuated", the fob would have triggered
it. It did not. `0x146` fires **only** on the FordPass/TCU command path.

A second confirmation from the same capture — the TCU's two distinct modes separate
cleanly:

```
t=61.5   fob unlock       -> 0x1B3 flips UNLOCKED,  no 0x146
t=67.1   0x590 appears    -> TCU wakes
t=69.3   0x22A byte7=0x01 -> TCU active ... still no 0x146
t=73.4   0x22A byte7=0x00 -> TCU done
```

**TCU active + no `0x146` = the TCU is *reporting*. TCU active + `0x146` = the TCU is
*commanding*.** Exactly what you would expect if `0x146` is the command rather than a
side effect.

And in every FordPass capture the `0x146` pulse lands **inside** the `0x22A`-active
window (unlock-3: 26.686 within 23.7–28.5; lock-A: 91.063 within 88.0–92.2;
unlock-C: same instant). Consistent across the board.

### Full evidence table

| capture | trigger | `0x146` pulse | `0x22A` active | `0x1B3` state |
|---|---|---|---|---|
| `unlock-2.csv` | FordPass unlock | `0x04` = UNLOCK ✅ | yes | L→U→L |
| `unlock-3.csv` | FordPass unlock | `0x04` = UNLOCK ✅ | yes | L→U→L |
| `unlock-C.csv` | FordPass unlock | `0x04` = UNLOCK ✅ | yes | L→U→L |
| `lock-A.csv` | FordPass lock | `0x0C` = LOCK ✅ | yes | L |
| **`lock-keyfob.csv`** | **physical key fob** | **none** ✅ | yes (reporting) | L→U→L |
| `unlock-B.csv` | FordPass unlock | none — lost in USB gap | no | U→L |

4/4 correct on labelled FordPass commands; correctly **absent** on the fob control.

---

## 3. Door lock STATE — `0x1B3`, byte 0, bit 2  ✅ VERIFIED

Persistent state, not an event. `1` = unlocked, `0` = locked.

Tracks the `0x146` pulse exactly — two independent IDs agreeing is the strongest
corroboration available:

- `unlock-3`: pulse @26.686s → state UNLOCKED @26.8s
- `unlock-C`: pulse @41.667s → state UNLOCKED @41.7s
- `unlock-B`: state flips UNLOCKED @22.1s but **no pulse captured** — it fell inside
  the USB-disconnect gap. Consistent, and a good check on the method.

**Auto-relock observed** ~25–30s after unlock with no door opened (`unlock-3` @51.1s,
`unlock-C` @65.9s, `lock-keyfob` @86.2s). Standard Ford behaviour, and it confirms
`0x1B3` is genuine state.

Also varies with lock state, not yet pinned down: `0x1B3` byte 6 (`0x00`/`0x40` locked
vs `0x02`/`0x42` unlocked — bit 1 appears to mirror byte 0 bit 2).

---

### byte 0 is TWO fields: a power-state nibble + the lock bit — 2026-09-12

Every labelled door capture was taken with the car **off**, so `0x1B3` byte0 was only
ever seen as `0x10` / `0x14`. On a drive it also takes `0x40` / `0x44`. The high nibble
is a **vehicle-power field**, not part of the lock state:

| byte0 high nibble | frames (drive_home 09-11) | vehicle speed (0x107) |
|---|---|---|
| `0x1_` | 436 | min 0, median 0, **max 0** — stationary / car off |
| `0x4_` | 9367 | 0–88, median 30 — car running / driving |

Confirmed on three drives (`drive_home-alsterbos_bergrijk`, `highway-drive-2`,
`2026-09-10 14-05-10`): `0x1_` frames NEVER coincide with non-zero speed.

So byte0 = `[power nibble] | [bit2 = unlocked]`. Observed values: `0x10` locked/off,
`0x14` unlocked/off, `0x40` locked/running, `0x44` unlocked/running.

**bit2 polarity is re-verified** against the labelled captures (`1` = UNLOCKED):
`unlock-3` `0x10`→`0x14` at the command; `unlock-C` same; `lock-keyfob` `0x10`→`0x14`
on the fob unlock then back to `0x10` on the auto-relock; `lock-A` stays `0x10`.
The decoder (`(byte0 >> 2) & 1`) is correct and the firmware reproduces the raw bus
transition-for-transition.

> ⚠️ **Caveat:** bit2 is only *validated* in the `0x1_` (car-off) context, because that
> is all the labelled captures contain. Its meaning in the `0x4_` (running) context is
> assumed identical, not proven. Two mid-drive flips at 54 and 77 km/h in
> `drive_home` are most likely `canlog.py` USB-dropout artifacts (a transition is
> timestamped when first *seen* after logging resumes), not real door events.

### The lock state is STALE by nature — firmware fix 2026-09-12

`0x1B3` exists only while the BCM broadcasts. Parked with the bus asleep there are no
frames, so the last value persists indefinitely and a dashboard shows an hours-old
lock state as if it were live.

Firmware changes:
- **`0x1B3` is now the ONLY writer of `DOORS_LOCKED`.** Removed the optimistic update in
  `queue_command_event` (it set locked on `lock`/`start` and unlocked on `unlock` the
  moment the command was *sent*) — since remote UNLOCK is ignored by this car, that made
  the dash claim UNLOCKED while the doors stayed locked. Also removed the remote-start
  override that forced `DOORS_LOCKED = true`, and the suppression that blocked `0x1B3`
  from updating during a remote-start countdown. This is rule 4 ("`0x1B3` is the
  evidence") applied to the telemetry path.
- **New `doors_locked_age_s`** in the telemetry JSON: seconds since `0x1B3` last
  refreshed the value, so the UI can show "as of X" or grey it out instead of
  presenting stale state as current.

## 4. TCU command-active flag — `0x22A`, byte 7, bit 0  ✅ VERIFIED

`00 00 00 00 01 01 00 01` while the TCU is executing **or reporting**; `...00`
otherwise. Idle payload is `00 00 00 00 01 01 00 00`.

This is cr08's `TcuCmd_Active` and it **does** transfer to this vehicle. But:

> **The lock/unlock selector is NOT in 0x22A on this car.** Across every capture,
> byte 7 only ever took `0x00` or `0x01` — the `LockCtrl` bits (`0x21`/`0x41`) never
> appeared. `0x22A` is the "TCU is busy" wrapper; `0x146` carries *what*.

Active-window durations vary (one frame in `unlock-C`, ~4–5s elsewhere), so treat the
duration as not yet understood.

## 4b. TCU presence — `0x590`  ✅ VERIFIED
Appears only while the TCU is awake and active. Absent otherwise. Useful as a
"is the TCU participating" probe.

---

## 5. Bus behaviour  ✅ VERIFIED

- **The bus sleeps hard.** With the TCU present and healthy it idles silent at ~1.9 V.
  **KOEO does not wake it.**
- **Opening the FordPass app wakes the bus** — a status-refresh wake with no command.
  This explains the "mystery" wake events in earlier captures and is a real confound:
  not every wake is a command.
- **A key fob action wakes the bus, then wakes the TCU a few seconds later** so it can
  report the new state to the cloud.
- **Unplugging the TCU makes the gateway flood the bus** (lost-comms) — the easiest
  guaranteed traffic source, and it doubles as Capture 0.
- Voltages: active bus ≈ CAN-H 3.0 V / CAN-L 1.6 V; asleep ≈ both 1.9 V.
- Pair reads ~60 Ω H-to-L (two 120 Ω terminators).

---

## 6. Other IDs identified

| ID | content | confidence |
|---|---|---|
| `0x11A` | **VIN**, multiplexed ASCII (`C1 00`/`01`/`02` frames) | high |
| `0x590` | TCU presence — only while the TCU is active | high |
| `0x59E` | `GWM_AutoSar_NetMgmt_FD1` — the **gateway's own** NM heartbeat. Never transmit this. | high |
| `0x1B4` | `40 11 11 FF 23 23 00 00` — static | confirmed real |
| `0x1B8` | `FF FF FF FF 00 00 00 00` — static | confirmed real |
| `0x28A/B/D/E` | schedule / preconditioning frames | confirmed real |
| `0x084` | vehicle clock B6 hour, B4 min, B5 sec | verified 2026-09-09 |
| `0x100/101/104/105/108/10E/141/142` | powertrain/telemetry — see census + verified table below | verified 2026-09-09 |
| `0x107` | **vehicle speed** — b0-1 BE × 0.01 km/h (§7) | verified |

> **Correction to an earlier assessment in this project:** `0x1B4` / `0x1B8` / `0x590` /
> `0x28A` / `0x28B` were previously judged "fabricated" because they appear in no public
> Ford DBC. They are **real** and present on this bus with byte-identical payloads. The
> reasoning error was treating *absent from opendbc* as *invented*. The firmware code
> that transmitted them was based on genuine captures and should be restored.

---

## 7. Vehicle speed — `0x107`, bytes 0-1 (big-endian) × 0.01 km/h  ✅ VERIFIED

Decoded from `highway-drive-2.csv` (490 s: highway → full stop → city stop-and-go →
parked → power-off). No external reference was needed — the drive's own structure is
the proof:

- Reads **exactly `0x0000`** for the entire parked tail and at the mid-drive full stop.
- Tracks the whole speed profile smoothly (highway cruise 78–90, one stop to 0, then
  stop-and-go 15–45).
- Peaks at **9173 raw = 91.73 km/h** — matches the "under 100 km/h" drive.
- Transmitted at ~10 Hz.

Scaling is `(b0<<8 | b1) * 0.01` km/h. 18.6 % of the capture reads exactly 0 (the stops),
which fixes the zero point beyond doubt.

> **Supersedes earlier guesses.** The old `0x118` byte-4 candidate correlates only
> *inversely* (r = -0.94) and never zeroes at a stop; `0x100` "speed" was a rolling
> counter. `0x107` is the clean channel. `0x107` byte 2 also tracks speed but saturates -
> left unidentified for now.

Wired into firmware `decode_can_telemetry` (2026-08-30) -> `VEHICLE_SPEED`, already plumbed
to the telemetry JSON and the UI (km/h). The board also carries an independent **GPS speed**
reference (`parse_gps_info`, km/h), so CAN-vs-GPS speed can be cross-checked live once the
transceiver RX is fixed.

---

## ✅ RESOLVED — who transmits `0x146`: the TCU

**Capture 0 (`capture0-tcu-unplugged.csv`) settled it: the TCU transmits `0x146`.**
With the TCU connector unplugged and the doors worked by fob, `0x146` was **gone from
the bus entirely** (0 frames vs 46 with the TCU present). Nothing else emits it. So it
is the TCU's frame, and it is the command the Franken-Modem must send.

### ⚠️ SUPERSEDED — the Capture 0 ownership map below is WRONG for 9 IDs (2026-09-12)

Capture 0 was taken on a **parked** car with the TCU unplugged. An ID missing from it
therefore means *"that module was asleep"*, not *"the TCU owns it"*. Re-tested against
three **TCU-removed + DRIVEN** captures (`2026-09-09 06-28-10`, `2026-09-10 14-05-10`,
`drive_home-alsterbos_bergrijk` 09-11) the map corrects to:

| verdict | IDs |
|---|---|
| ✅ **TCU-owned — confirmed** (absent from all three TCU-gone drives) | `0x22A` `0x102` `0x127` `0x212` `0x21E` `0x232` `0x27B` `0x27C` `0x27D` `0x27E` `0x27F` `0x2B7` `0x146` `0x590` `0x28A/B/D/E` |
| ❌ **NOT TCU-owned — falsified** (thousands of frames per drive with no TCU) | `0x112` `0x152` `0x267` `0x141` `0x142` `0x172` `0x173` `0x174` `0x175` |

`0x141/0x142/0x172–0x175` were already hedged as unproven — now proven to be vehicle
modules. **`0x112`, `0x152` and `0x267` were in the CONFIRMED list and are definitively
not TCU-owned.** (The `0x146`/`0x590`/`0x28A–E` result is clean, not circular: the
board's keepalive defaults OFF and those IDs are absent from the captures, so we were
not transmitting them.)

**The car does not miss the TCU.** Across the three TCU-removed drives: bus load fell to
79–81 frames/s (vs 92.7 with the TCU), and **zero** new IDs appeared — no lost-comms
flood, no complaint frames. The Capture 0 "gateway floods the bus" observation was a
parked-car transient, not a steady state. Telemetry decoding is pure RX and unaffected.
(Caveat: a *stored* DTC would not necessarily broadcast a new ID, so this is not proof
the car logged nothing — worth a FORScan DTC read if one is ever available.)

**Firmware consequence — a second real bug fixed.** `TCU_PRESENCE_SET` was trimmed from
22 to 13 frames, dropping the nine falsified IDs. Transmitting them would have collided
with live modules (the same bug already fixed for `0x1B4/0x1B5/0x1B8`) **and poisoned our
own telemetry**, because we decode three of them: `0x174` (fuel level),
`0x175` (steering angle), `0x142` (ambient temp, ICE running). `tcu on` would have frozen
fuel, flatlined steering and pinned ambient to our static constants — indistinguishable
from a decoder bug.

**`0x112` is a vehicle module, and `tcu_motion` never meant motion.** It broadcasts at
~10 Hz. Bytes 4-5 are a 16-bit counter resetting to exactly 1000 (`0x03E8`) at each drive
leg then drifting 900-1000; byte 7 resets to 0 and climbs. Reading byte 4 alone only says
"that value is >= 768", i.e. the block is initialised — which latches ~2-3 min into a
drive and never drops at stops. Proof it is not motion: byte4=`0x00` occurs at up to
38 km/h, and 22 % of byte4=`0x03` samples are at speed < 0.5 km/h. Decoder renamed
`decode_0x112_initialised`; use `0x107` for motion. Bytes 5 and 7 unidentified.

> **Method note for the future:** absence in a *parked* capture proves nothing about
> ownership. The correct control for "who owns this ID" is a **driven** capture with the
> TCU removed.

### Capture 0 — full bus ownership map (HISTORICAL — see correction above)
Unplug the TCU, log, and every ID sorts into TCU-owned vs gateway/BCM-owned:

| category | IDs |
|---|---|
| **TCU-transmitted** (vanish when unplugged) | `0x146` `0x22A` `0x590` `0x28A` `0x28B` `0x28D` `0x28E` `0x102` `0x112` `0x127` `0x152` `0x212` `0x21E` `0x232` `0x267` `0x27B–0x27F` `0x2B7` — and `0x141/0x142/0x172–0x175` (but these are more likely modules that simply didn't wake — treat as unproven) |
| **gateway/BCM-transmitted** (still present) | `0x1B3` `0x1B4` `0x1B5` `0x1B8` `0x59E` `0x11A` `0x100–0x110` `0x113` `0x118` `0x122` `0x147` `0x153` `0x15E` `0x07A` `0x084` `0x215` `0x2BF` |

> **Firmware consequence — a real bug fixed.** The old firmware transmitted `0x1B4`,
> `0x1B5`, `0x1B8` as "TCU NM frames." Capture 0 proves they are the **gateway's**.
> Emulating them = two nodes claiming one ID on a two-node bus. **Dropped.** Kept the
> genuinely TCU-owned `0x590` + `0x28A/B/D/E`.

> [!NOTE]
> **DTC U0198:00-0A and DSP Watchdog (FORScan Task):**
> When the factory TCU is unplugged and a custom replacement operates in passive listening (RX-only) mode, the **DSP (Audio Digital Signal Processing Module)** logs DTC `U0198:00-0A` (*Lost Communication With Telematic Control Module 'A'*).
> - **Why only DSP?** The DSP amplifier is wired for eCall / 911 Assist audio ducking and monitors HS3-CAN specifically for TCU telematics audio status. All other modules (GWM, APIM, BdyCM, BECM, PCM) report `None`.
> - **Impact:** Completely benign. The Malfunction Indicator Lamp is **Off**, no cluster warnings are displayed, and audio/drivability are 100% unaffected.
> - **Resolution:** When renewing the FORScan Extended License, open **DSP Module Configuration** (or As-Built `783-xx-xx` / APIM `7D0-02-02` / ACM `727-xx-xx`), disable **Telematics / Emergency Assistance / eCall Present**, cycle ignition, and clear DTCs. This permanently resolves `U0198` cleanly without requiring CAN heartbeat emulation.

---

## Command frame — full specification (as captured)

```
LOCK    0x146 = 0C 00 00 <seq> 00 00 DA B3
UNLOCK  0x146 = 04 00 00 <seq> 00 00 DA B3
```
- **byte 0** = command: `0x0C` lock, `0x04` unlock (bit2 = execute, bit3 = lock direction).
- **byte 3** = `<seq>`, a slowly-incrementing counter: observed `CD, CE, D2` (unlocks)
  then `D3` (lock) across the session. **Suspected anti-replay sequence** — increment it.
- **bytes 6–7** = `DA B3`, constant across every command.
- bytes 1,2,4,5 = `00`.
- Sent **while `0x22A` byte7 = `0x01`** (command-active), as a momentary pulse; `0x146`
  returns to all-zero idle immediately after.

---

## Implemented in firmware (2026-08-29)

`firmware/src/main.rs`:
- `tcu_door_lock` / `tcu_door_unlock` → new `tcu_door_command(can, byte0, label)` sending
  the 0x146 frame above, with `LOCK_SEQ` (seeded `0xD4`) incrementing byte 3.
- `tcu_send_raw(can, id, data)` — general sender (0x146 and presence frames aren't 0x22A).
- `tcu_send_presence()` — broadcasts the verified TCU set (`0x590`, `0x28A/B/D/E`,
  `0x146`-idle); explicitly excludes the gateway frames.
- Old cr08-encoding functions retained as `*_legacy` (dead-code) for reference.

**Status (2026-09-06): compiled, flashed, and the CAN layer is proven on a two-node bench
bus** — the board transmits frames another node ACKs, and receives frames another node sends
(§0). What is still NOT proven is that the BCM *acts* on `0x146` from this board rather than
from the factory TCU. That is Phase 4 below, and it is now genuinely the next step rather
than something blocked behind a broken transmit path.

---

## ⏭ Phase 4 — injection test (the next real milestone)

Goal: confirm the board can actually actuate the doors, and pin down what the command
requires (sequence counter? presence frames? command-active flag?).

**Preconditions:** `phy` PASSES and `phy scan` names GPIO22 (both confirmed 2026-09-06),
`bus` shows TX ARMED, factory TCU still installed (so the bus has its second terminator and an ACK partner — the board is NOT the
sole node yet). Sniffer on a second tap (or SavvyCAN) to watch what happens.

**Tests, in order — change one variable at a time:**
1. Bus awake (open FordPass app or fob), then `unlock`. Do the doors move? Watch for the
   BCM's `0x1B3` state flipping to UNLOCKED — that is the success signal, not the doors alone.
2. If nothing: `hb on` first (presence frames), wait a few seconds, then `unlock`. Tests
   whether the BCM ignores a command from a node it doesn't yet consider a present TCU.
3. If nothing: try from cold sleep — `wake`, then `unlock` — the factory path always
   wakes the bus first.
4. If it works once then fails: the `<seq>` counter is being validated and our seed/step
   is off — capture a fresh factory command, read byte3, reseed `LOCK_SEQ`.
5. Confirm `lock` (`0x0C`) as well as `unlock`.

**Success = `0x1B3` flips state on the board's command with no fob/app involved.**
That is the whole project proven end-to-end at the CAN layer.

> ⚠️ The board is currently in NORMAL (ACK) mode with the factory TCU present. When you
> later REMOVE the factory TCU (two-node bus), the board becomes the gateway's only ACK
> partner — keep it in NORMAL mode then, never NoAck (see docs/can-power-diagnosis).

---

## Method notes / gotchas

- **TEL0150 has no SLCAN listen-only (`L`).** SavvyCAN's "Listen Only" makes it send
  `L`, which the module rejects with `0x07` — connected but deaf. Leave it unchecked.
  `canlog.py` sends `O` and sidesteps this.
- **SavvyCAN v208 drops SLCAN timestamps.** Use `canlog.py`.
- **Timestamps are host-side and coarse.** A burst read shares one timestamp; frame
  *order* is reliable, sub-100 ms *timing* is not.
- **The adapter drops off USB roughly once per run.** Idle-suspend was ruled out (an
  idle keepalive every 5 s did not prevent it). `canlog.py` auto-reconnects and
  salvages handshake frames, so runs survive; suspect the cable/port/adapter if you
  want it gone.
- **Capture in labelled pairs.** One command type alone cannot separate wake-noise from
  command payload — that is exactly why the unlock-only captures were inconclusive and
  the first *lock* capture cracked it immediately.
- **Always run a control.** The key fob capture was worth more than any additional
  unlock capture, because it was the one that could falsify the hypothesis.

## Tools
- `canlog.py` — SLCAN logger; real timestamps, auto-reconnect, SavvyCAN-compatible CSV.
- `candiff.py compare A.csv B.csv --ignore-counters` — diff two captures.
- `candiff.py events X.csv --id 0x146` — diff wake events within one capture.
- `tools/correlate_can.py` — correlation analyzer against Car Scanner OBD-II ground truth.
- `tools/parse_canlog.py` — decode every HS3 ID in a SavvyCAN CSV; exits 1 if any ID is unknown.
- `tools/ford_fusion_hybrid_2018.dbc` — standard DBC signal database.

## Capture library
| file | what it is |
|---|---|
| `drive_45min.csv` | **38.6 min road drive**: 187,836 frames correlated against Car Scanner (`r=0.99995`) |
| `unlock-2/3/C.csv` | FordPass unlock — all three show `0x146 = 0x04` |
| `lock-A.csv` | FordPass lock — `0x146 = 0x0C` |
| `lock-keyfob.csv` | **control**: physical fob, no `0x146` |
| `unlock-B.csv` | unlock with the pulse lost in a USB gap |
| `start-then-stop.csv` | early SavvyCAN capture, no timestamps |

---

## Verified Telemetry Signals (Derived via Car Scanner Correlation)

Correlated with microsecond-precision timestamps against Car Scanner OBD-II ground truth from `drive_45min.csv`:

| Signal | CAN ID | Bytes / Field | Conversion Formula | Correlation ($r$) |
|:---|:---|:---|:---|:---|
| **Vehicle Speed** | `0x107` | `bytes[0:1]` BE | `raw * 0.01` km/h | **0.99995** |
| **Coolant Temperature** | `0x104` | `byte[2]` | `raw - 60.0` °C | **0.9994** |
| **Trip Distance (Odometer)** | `0x113` | `bytes[4:5]` BE | `raw * 0.1` km | **1.0000** |
| **Steering Wheel Angle** | `0x175` | `bytes[3:4]` BE | `(raw - 8192) * 0.1` deg | Verified |
| **Tire Pressure FL** | `0x1B5` | `byte[1]` | `raw * 0.01` bar (or kPa) | Verified |
| **Tire Pressure FR** | `0x1B5` | `byte[3]` | `raw * 0.01` bar (or kPa) | Verified |
| **Tire Pressure RL** | `0x1B5` | `byte[5]` | `raw * 0.01` bar (or kPa) | Verified |
| **Tire Pressure RR** | `0x1B5` | `byte[7]` | `raw * 0.01` bar (or kPa) | Verified |
| **Fuel Tank Level %** | `0x174` | `bytes[1:2]` BE | `raw * 0.1` %; `0` and `1000` invalid | **0.9998** vs OBD |
| **Fuel Used Accumulator** | `0x109` | `bytes[3:4]` BE | Count drops with fuel burn | **0.9996** |
| **VIN Broadcast** | `0x11A` | `bytes[2:7]` ASCII | `C1 00..02` -> `3FA6P0XX9YY123456` | Exact |
| **HV battery SOC** | `0x108` | `byte[0]` | `raw * 0.5` % | Same 0.5 %/LSB as BECM UDS; morning 75–84 % |
| **Ambient air temp** | `0x142` | `byte[0]` | `raw - 64` °C | vs OBD outside ~13 °C |
| **ICE running (HS3)** | `0x142` | `byte[2]` | `0x64`=EV, `0x68`=ICE | vs `0x103` RPM / EV flag |
| **Motor / inverter temp** | `0x10C` B6 | — | **retracted** | warm-drive snapshot only; see Open RE — `0x10C` |
| **Gateway GPS** | `0x15E` mux `16/76/86` | B1:B2 lat, B5:B6 lon | `lat = 43.6591 + raw*0.000256034`; `lon = 5.0 + raw*0.000016` | Car Scanner R²=1.000 (lon is a ~1° regional window) |
| **Hybrid mode** | `0x105` | `byte[0]` | `E0` parked, `E8` city, `F8` highway | vs speed / EV fraction |
| **Transaxle P/D** | `0x10A` | `byte[2]` | `0x39`=P, `0x08`/`0x09`=D | vs `0x107` speed |
| **Vehicle clock** | `0x084` | B6 hour, B4 min, B5 sec | HH:MM:SS (B2:B3 calendar day) | cluster clock |
| **HV pack current / voltage** | `0x07A` | 15-bit current + 10-bit voltage | `A = raw*0.05-750`; `V = raw*0.5` | pedal +45 A / brake −27 A; pack 266–304 V |
| **This-drive ICE distance** | `0x118` | B3:B4 BE | `raw * 0.1` km | Δ matches lifetime ICE (tttt 0→13.0 km, hwy-2 +3.2 km) |
| **This-trip fuel used** | `0x113` | B1 | `raw * 0.1` L | vs OBD Fuel used MAE 0.056 L, r=0.999 |
| **Longitudinal accel** | `0x106` | B4[1:0]+B5 10-bit | `raw*0.035 − 17.9` m/s² | vs OBD GPS accel r=0.85, MAE 0.017 g |
| **HV current gauge** | `0x141` | B0 | `A ≈ (raw − 39) * 24` (12 levels 34–45) | MAE ~12 A vs `0x07A` after controlling for steer |
| **0x141 B5 packing** | `0x141` | B5 | bits7–6 2-bit field; bits5–0 analog always `0x1F–0x28` | all 24 captures |
| **0x10E alt-drive bits** | `0x10E` | B4 bit5, B5 bit2 | `1` iff B7=`A0` | accuracy 1.0 on every log |
| **0x10E B3 zero-gates B6** | `0x10E` | B3, B6 | B3=`00` ⇒ B6=`00` | 136/136 frames; B3 is not km/h |

Paired morning capture `can logs/2026-09-09 06-28-10.csv` with Car Scanner OBD: add **+66.5 s** to OBD timestamps to land on the CAN clock (`0x107` vs OBD speed ρ=0.9993). Do not use the afternoon pair for HV-current correlation (PIDs missing; clocks misaligned).

The former 12 unmapped IDs now have parsers. `0x10F`/`0x110` are high-rate 16-bit values aliased at ~1 Hz in SavvyCAN. Per-byte roles are listed by `python3 tools/parse_canlog.py`.

---

## Open RE — `0x174` fuel frame (2026-09-13)

Do **not** publish B3 bit 7 as the cluster low-fuel lamp. Decoder returns `None`. Drive / Grafana flags must stay off until a real empty-tank lamp bit is found.

### Frame (all logs)

| Byte | Observed | Status |
|---|---|---|
| B0 | always `00` | pad |
| B1:B2 | BE16 × 0.1 % | **solved** — OBD fuel level r=0.9998. `raw==0` and `raw==1000` (`03 E8`) are init/invalid, not a real 100.0 % |
| B3 bits 0–1 | always `11` → byte `03` / `83` | likely sticky **sender OK / data valid**. Never seen `00`/`01`/`02` |
| B3 bits 2–6 | always 0 | unused in every capture |
| B3 bit 7 | `03` vs `83`, 1–5 s bursts | **not** the lamp — see below |
| B4 | always `8E` | constant trailer; unknown |
| B5 | always `02` | constant trailer; unknown |
| B6 | `7F`–`81` per session | almost constant per drive; unknown |
| B7 | session-stable (`04`, `1C`, `28`, `A9`, `C8`, `C9`, `EB`, `F6`, …) | not an enum of 2–3 states; unknown |

### B3 bit 7 — working ID: IPC anti-slosh / low-speed fuel-gauge commit

Ford damps the cluster fuel needle: hold while the tank sloshes, commit a new filtered % after speed stays low. Bit 7 is the short **update-in-progress** strobe on that commit.

| Check | Result |
|---|---|
| Duration | 1, 2, 3 or ~5 s (discrete), ~0.5–4 % of `0x174` frames |
| Repeat | every ~30 s–15 min on city drives; **zero** on `highway-drive-1` (123 s, no stops) |
| Speed (`0x107`) | bit7 mean ~1.5 km/h, max **4.6–6.7** across four drives; off-bit mean 33–69 km/h |
| Fuel % step | P(B1:B2 changes \| in burst) ≈ **0.18** vs **0.005** outside (`drive_home-alsterbos_bergrijk`, 36×) |
| Tank / DTE | fires at 70–100 % / 700+ km DTE — same rate as mid-tank |
| ICE / RPM | independent (EV crawl and ICE both) |
| `0x175` | ignore — steering always moves; false companion |

Logs: `drive_home-alsterbos_bergrijk.csv` (11 bursts / 4419 frames), `2026-09-10 14-05-10.csv` (10), `2026-09-09 15-43-38.csv` (6), `2026-09-09 06-28-10.csv` (3), `highway-drive-2.csv` (1). Unlock / lock / remote-start clips are too short and sit on the `0000` sentinel.

### Ruled out

- **Cluster low-fuel lamp** — guessed because `0x83` looks like a flag. Sticky lamp cannot flicker 1–5 s every few minutes on a full tank.
- **Refuel event** — repeats throughout a drive as % drops, not only after a fill.
- **Sender open/short** — would stick or latch a DTC, not pulse at every stop.

Factory name is unknown (no DBC). Older Ford wording is *Fuel level input status (instant)* / anti-slosh update. Do not invent a MQTT field until the lamp hunt is done.

### Still needed

1. **Empty-tank capture** — SavvyCAN on HS3 + photo/timestamp of the cluster low-fuel tick. Hunt every other ID (IPC `0x720/0x728`, `0x118` DTE, `0x174` B4–B7, `0x172/0x173`) for a bit that **stays** on with the lamp. That is the real lamp.
2. **Key-on prove-out** — some clusters sweep the lamp at ignition; a 1–5 s pulse at key-on is prove-out, not this stop-and-go bit.
3. **B4–B7** — checksum / life counter / second sender? Need two fills and a long park, not more highway.
4. **B3 bits 0–1** — need a sender-fault or ignition-off residual frame where they are not `11`.

Do not re-label bit 7 as a Drive flag, Grafana lamp, or `low_fuel_lamp` in Influx. `decode_low_fuel_lamp` is intentionally `None`.

---

## Open RE — `0x10C` “motor temp” (2026-09-13)

`0x10C` B6 as raw °C is **not** motor / inverter temperature. Grafana showing ~70 °C on a stationary cold car is the byte, not the physics.

### Why it looked solved

The census matched B6 (66–76) to a **warm-drive OBD Motor Coil snapshot** (66–68 °C). That overlap is the live band of B6, not a correlation.

### What the byte actually does

| State | `0x10C` | B6 |
|---|---|---|
| thermal off (unlock, key-off residual) | B3=`00` B4=`00` | `00` (decoder used to drop this as `<20`, so MQTT last-known stuck at 70) |
| thermal live | B3=`02` B4=`B0` | **64–77 only**, every log |

Cold-start `drive_home-alsterbos_bergrijk.csv`: first live frame `[6F 00 00 02 B0 00 46 30]`, B6=70, ICE coolant `0x104` = **17 °C**. B6 vs coolant r=0.05–0.21.

Paired `2026-09-10 14-05-10.csv` + OBD (speed lag **+28 s**):

| OBD PID | Range that drive |
|---|---|
| Motor Coil Temp | **23 → 66 °C** |
| Generator Coil Temp | 41 → 110 °C |
| eCVT Temp | 23 → 55 °C |
| Engine coolant | 25 → 96 °C |
| `0x10C` B6 live | 65–77, vs Motor Coil **r=0.03 MAE=22 °C** |

No `0x10C` byte hits r≥0.5 vs those PIDs. B0 (54–161) soaks while B6 is 0; it is not coolant (`B0−40` vs `0x104` r<0.3).

### Leading candidate for real motor coil

`0x141` **B2 − 40** vs OBD Motor Coil on that pair: **r=1.000, MAE=0.05 °C** (23–66). B1 on the same frame rolls 0–255 (counter), so the old `0x141` B1:B2 MAP (`raw*0.01` gated 80–120 kPa) is likely the counter landing in 0x1F–0x2E. Do not swap MAP or publish B2 as motor coil until MAP is re-homed and a second log repeats the B2 identity.

Also on `0x141`: B3/B7 already noted as temp-like (not `0x104` coolant). Check those vs generator / eCVT.

### Still needed

1. Replay morning `2026-09-09 06-28-10` OBD Motor Coil vs `0x141` B2−40 (same +66.5 s speed lag).
2. Find real MAP — not B1:B2 if B1 is a counter.
3. Identify `0x10C` B6 (setpoint? inverter-loop enable default?). Leave unpublished.
4. `0x10C` B0 soak analog — needs a formula, not a name.

`decode_motor_temp` is intentionally `None`. Do not put `motor_temp` back on Grafana until a cold-start OBD match exists.

---

## Open RE — turn signal side (2026-09-13)

`0x1B3` B1 bit 0 is **stalk/hazard active**. B1 bit 1 is the bulb. B6 bit 6 was labeled left vs right; it is **not**.

On every multi-second burst in `drive_home-alsterbos_bergrijk` (and 09-09, 09-10, hwy-2), the only bits that toggle while active are **B1 bit 1 and B6 bit 6**, in lockstep (~1.5 Hz, ~50/50). Nothing else on `0x1B3` is constant-per-burst in a way that splits two stalk sides. Decoder `LEFT`/`RIGHT` was the flash phase. Grafana “only RIGHT” is whichever phase MQTT/`last()` sampled.

Publish `OFF`/`ON` + `turn_signal_active` / `flasher_on`. Side needs another ID (or a labeled left-only vs right-only capture vs the rest of the bus).

---

## Open RE — `0x110` motor RPM

Raw electrical angle is a 0–360° wrap from `0x110` B1:B2 (resolver valid = B3 bit 7). Native rate is ~kHz; MQTT/Wi-Fi only publishes the last snapshot, so a 2 Hz Fran poll is still aliased junk. Grafana tiles for the angle were removed 2026-09-15. Keep the MQTT field for Lab/RE.

RPM has to be computed **on the CAN RX thread**, not in Grafana.

1. **Hit rate first.** The `canlog.py` trips (TEL0150 SLCAN @ 115200) are what we already measured: `0x110` at **1.00 Hz**, `0x107` at 10 Hz, whole-log ~80–90 fps. That is the UART bottleneck (~460 SLCAN frames/s theoretical, far below HS3), not a kHz tap. Highway-2 angle sitting in a 4° band is 1 Hz alias. These files are great for slow RE; they cannot prove RPM. Need on-device `0x110` frames/s + TWAI `rx_missed`. If we keep ≳100 Hz of valid frames, unwrap works without a hardware filter. If not, a temporary TWAI accept filter (`0x110` + `0x107`) is a probe only — do not ship that as the default listen set.
2. **Unwrap.** Keep last raw `u16` + timestamp. `d = wrapping i16 delta`, `elec_rps = (d / 65536) / dt`. Drop samples with invalid bit, `0xFF` sentinel, `dt` too large (FIFO gap), or implausible RPM. EMA → publish `motor_rpm` (electrical) at the normal telemetry cycle.
3. **Direction.** `sign(d)` is electrical sense, not cabin Reverse (HS3 has Park vs not-Park only). Correlate vs `0x107` speed, HV current sign, EV vs ICE. That also tells us MG1 vs MG2.
4. **Mechanical RPM** needs pole pairs. Do not guess; publish electrical until a Ford/OBD match exists.
5. Then a Grafana/Fran RPM tile. Do not put the 0–360° angle back on a chart.

`0x10F` is the same class (high-rate 16-bit, aliased at 1 Hz capture). Do not overlay it either until it has an on-board derived quantity.

---

## Questionable Signals & Pending RE Log Targets (2026-09-18)

This register tracks signals where decoding or physical meaning is ambiguous, contradicted by real-world driving observations, or pending dedicated ground-truth CAN captures. **Do not guess logic or commit speculative decoders**—verify with labeled logs first.

| Signal / Field | CAN ID & Location | Current Decoding / Behavior | Real-World Anomaly & Questions | Required Capture / Test to Resolve |
|---|---|---|---|---|
| **`flasher_on`** (formerly labeled "HAZARDS") | `0x1B3` Byte 1 Bit 1 | `(data[1] >> 1) & 0x01 != 0`<br>Exported as `flasher_bulb_on` | **Fires in lockstep with normal turn signals** even when hazard button is never touched. Grafana mapped `1` to "HAZARDS", creating false hazard events. Is this bit just the bulb/relay blink pulse, or does a dedicated hazard flag exist on HS3? | Labeled capture: (1) Hazards pressed while parked, (2) Left turn only, (3) Right turn only, (4) Hazards pressed while turn stalk active. |
| **Turn Direction** (Left vs Right) | `0x1B3` B1 bit 0 & B6 bit 6 | B1 bit 0 = active<br>B6 bit 6 = flash phase | Turn direction (Left vs Right) is **not present on `0x1B3`**. B6 bit 6 toggles on phase, not side. Grafana left/right mapping was an artifact of sampling phase. | Labeled left-turn vs right-turn log comparing all 63 HS3 IDs to determine if stalk direction is gatewayed onto HS3 at all. |
| **`intake_map_kpa`** | `0x141` Bytes 0–2 | Formerly B1:B2 MAP scaling | Bytes 0–1 are the Empower / power gauge (offset 10000). Byte 1 is an 8-bit rolling counter. HS3 does not appear to carry true intake MAP. | OBD-II MAP PID paired with high-load ICE acceleration log to check if MAP is hidden on another frame. |
| **`motor_temp`** | `0x10C` vs `0x141` Byte 2 | Suppressed (`None`) in firmware | `0x10C` does not track OBD motor coil temp (hovers in 64–77 range). `0x141` Byte 2 ($raw - 40$) is strong candidate ($r=1.000$, MAE $0.05^\circ\text{C}$). | Cold-start capture from cold soak to fully warmed motor with simultaneous OBD motor temp PID. |
| **`low_fuel_lamp`** | `0x174` Byte 3 Bit 7 | Flag `(b3 & 0x80) != 0` | Becomes active during normal fuel levels; acts as fuel sender anti-slosh/slosh dampening flag rather than cluster warning lamp. | Empty-tank capture (DTE $< 50\text{ km}$, low fuel cluster chime/lamp on) to identify the real cluster lamp bit. |
| **`vehicle_in_motion`** vs **`tcu_motion`** | `0x1B3` B0 bit 6 vs `0x112` Byte 4 | `(b0 & 0x40) != 0` vs `b4 == 0x03` | Both are currently logged and charted. `0x1B3` is BCM motion lockout flag; `0x112` is TCU motion status. | Low-speed crawl ($< 3\text{ km/h}$) and reverse log to observe threshold speeds, transition delays, and edge cases. |



