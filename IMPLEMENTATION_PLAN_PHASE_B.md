# Phase B Implementation Plan: Protocol Translator (`el_bridge`)

## 1. Objective
To implement `el_bridge`, a cross-platform protocol translator that bidirectionally converts between fragmented MIDI 1.0 byte streams and modern MIDI 2.0 Universal MIDI Packets (UMP). This component focuses heavily on real-time thread safety, maintaining the strict zero-allocation (MISRA-2023 equivalent) and `no_std` constraints established in Phase A for its core logic, while utilizing a cross-platform MIDI backend (`midir`) for I/O.

## 2. Architecture: `el_bridge` Integration

The `el_bridge` CLI application bridges the gap between the OS-level MIDI API and our `el_core` embedded engine.

### 2.1 Directory Structure
```text
/
├── el_bridge/
│   ├── Cargo.toml (CLI binary, uses std, midir, crossbeam, clap)
│   └── src/
│       ├── main.rs (CLI setup and thread spawning)
│       ├── backend.rs (midir I/O wrappers)
│       ├── translator/
│       │   ├── mod.rs
│       │   ├── to_ump.rs (MIDI 1.0 -> MIDI 2.0 UMP)
│       │   └── to_midi1.rs (MIDI 2.0 UMP -> MIDI 1.0)
│       ├── state.rs (NRPN / 14-bit CC / SysEx aggregation state)
│       └── queues.rs (Lock-free ringbuffer definitions)
```

## 3. Component Design: Translation Logic (`el_bridge::translator`)

The translation logic will reside largely in `el_bridge::translator` but will lean heavily on new structs added to `el_core`.

### 3.1 `to_ump.rs`: MIDI 1.0 to MIDI 2.0
- **Responsibility:** Consume `u8` streams and emit `Ump` packets.
- **Aggregation State Machine:** Must maintain state per channel to aggregate fragmented MIDI 1.0 messages:
  - **14-bit CCs:** Wait for CC# 0-31 and the corresponding CC# 32-63 before emitting a single 32-bit MT=0x4 UMP.
  - **NRPN/RPN Sequences:** Track CC# 99/98/6/38 sequences. Emit a high-resolution 32-bit MT=0x4 Parameter UMP only when the data entry is complete.
  - **SysEx:** Buffer SysEx 7-bit bytes. Chunk and emit as 64-bit (MT=0x3) Data Messages. (See Section 3.3 for bounded buffering).
- **Scaling:** Uses `el_core::utils::scale_up` to convert 7-bit Velocity/CC values to 16-bit/32-bit equivalents.

### 3.2 `to_midi1.rs`: MIDI 2.0 to MIDI 1.0
- **Responsibility:** Consume `Ump` packets and emit `u8` streams.
- **Fragmentation:**
  - Converts single 32-bit MT=0x4 CC/NRPN messages back into multiple sequential MIDI 1.0 `u8` messages.
  - Reassembles MT=0x3/0x5 SysEx chunks into a continuous `u8` stream.
- **Scaling:** Uses `el_core::utils::scale_down` to convert high-res values back to 7-bit limits safely.

### 3.3 Strict Memory Constraints & Bounded Buffers
- To adhere to our "zero dynamic allocation in hot paths" rule:
  - The aggregation state machine (`state.rs`) will use statically sized arrays per channel (e.g., `[Option<u8>; 16]` for CC states).
  - SysEx accumulation will use a statically bounded buffer (e.g., `heapless::Vec<u8, 256>`). If a MIDI 1.0 SysEx message exceeds 256 bytes, it will be chunked and flushed immediately as an incomplete MT=0x3 sequence to avoid allocation.

## 4. Real-Time Thread Safety & I/O (`el_bridge::main`)

MIDI processing relies heavily on callbacks that execute in high-priority audio threads.

### 4.1 Lock-Free Architecture
- **No Mutexes:** The hot paths (the translation logic inside the `midir` callback) will strictly forbid `std::sync::Mutex` or `RwLock` to prevent priority inversion and jitter.
- **Ringbuffers:** We will use a Single-Producer/Single-Consumer (SPSC) lock-free ringbuffer (e.g., `ringbuf` or `crossbeam-queue`) to pass data between the OS MIDI callback thread and the application/translation thread.

### 4.2 Data Flow
1. **Input Callback (`midir`):** Receives `&[u8]`. Immediately pushes to an SPSC byte-queue. Returns instantly.
2. **Translation Thread:** Pops from the byte-queue, feeds `to_ump.rs` state machine. Yields `Ump` packets.
3. **Output Queue:** Pushes `Ump` packets to a second SPSC queue.
4. **Output Callback (`midir`):** Pops from the UMP queue, transmits to the destination OS port.

## 5. Testing & Verification

### 5.1 Atomic Test Definitions (`el_bridge/tests/`)
1. **NRPN Aggregation:** Feed the translator individual `u8` bytes for CC99, CC98, CC6, CC38. Assert that exactly *one* valid 32-bit MIDI 2.0 NRPN UMP is generated at the end.
2. **14-bit CC Splitting:** Feed the translator a single MIDI 2.0 14-bit CC UMP. Assert it generates exactly *two* `u8` MIDI 1.0 messages (MSB then LSB).
3. **SysEx Bounding:** Feed a 300-byte MIDI 1.0 SysEx message. Assert it chunks correctly into MT=0x3 packets without panicking or allocating heap memory.
4. **Zero-Jitter Lock-Free:** A benchmarking test ensuring the `ringbuf` push/pop operations do not yield to the OS scheduler.

### 5.2 Integration Tests
- Set up loopback virtual MIDI ports.
- Route `el_bridge` to listen on Port A (MIDI 1) and output to Port B (MIDI 2).
- Use `el_gen` (from Phase A) to send high-res UMP packets; assert Port A receives the correctly down-scaled legacy bytes.

## 6. Execution Steps
1. Add `el_bridge` to the workspace.
2. Add `midir`, `ringbuf`, and `heapless` dependencies to `el_bridge`.
3. Implement `state.rs` bounded state tracking for CCs and NRPNs.
4. Implement the `to_ump.rs` and `to_midi1.rs` translation modules.
5. Scaffold the lock-free queues and `midir` threading logic in `main.rs`.
6. Write atomic tests verifying exact bit-for-bit translation accuracy.