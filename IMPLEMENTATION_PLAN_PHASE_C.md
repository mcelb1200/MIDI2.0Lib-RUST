# Phase C Implementation Plan: MIDI-CI Simulator (`el_ci_sim`)

## 1. Objective
To implement `el_ci_sim`, a cross-platform simulator for testing and validating the MIDI Capability Inquiry (MIDI-CI) protocol handshake. This tool will act as an Initiator or Responder, simulating Discovery, Protocol Negotiation, and Profile Configuration. The implementation will strictly adhere to the `no_std`, zero-allocation (MISRA-2023 equivalent) boundaries established in Phase A, focusing heavily on security against memory exhaustion attacks during SysEx chunking.

## 2. Architecture: `el_ci_sim` Integration

The simulator will leverage the `el_core` framework to build and parse complex, multi-packet SysEx Universal MIDI Packets (UMP).

### 2.1 Directory Structure
```text
/
├── el_ci_sim/
│   ├── Cargo.toml (CLI binary, uses std, clap)
│   └── src/
│       ├── main.rs (CLI setup and execution flow)
│       ├── initiator.rs (State machine for Initiator roles)
│       ├── responder.rs (State machine for Responder roles)
│       └── sysex_helpers.rs (Chunking and Reassembly utilities)
```

## 3. Component Design: MIDI-CI Protocol Logic

The core logic relies on precise bit manipulation and payload formatting for Universal SysEx messages within the MIDI-CI framework.

### 3.1 SysEx Payload Chunking & Reassembly
- **SysEx Manager (`sysex_helpers.rs`):**
  - Converts a large conceptual payload (e.g., Profile Details) into an `Iterator<Item = Ump>` that emits sequential 64-bit (MT=0x3) or 128-bit (MT=0x5) UMPs (Start, Continue, End).
  - Handles the reassembly of incoming fragmented SysEx UMPs into a continuous payload array.
- **Security & Memory Constraints:**
  - **Bounded Buffers:** To prevent memory exhaustion attacks, all reassembly buffers will be statically bounded (e.g., using `heapless::Vec<u8, 1024>`). If an incoming payload exceeds this boundary, the transaction is immediately rejected, and an error state is logged without panicking or allocating heap memory.

### 3.2 State Machines (`initiator.rs` & `responder.rs`)
- Both roles use explicit enum states to track the handshake.
  - `Uninitialized -> DiscoverySent -> DiscoveryReceived -> ProtocolNegotiating -> ProfileConfiguring`
- The state machines will evaluate incoming `Ump` packets from `el_core::parser::UmpStreamParser` and generate the appropriate response `Ump` sequences based on the current state.
- **Strict Offset Synchronization:** When parsing Universal SysEx headers, exact array indexing offsets must be maintained (e.g., extracting MUIDs from specific byte positions). Bounds-checking (`index < payload.len()`) must be explicitly performed before access to prevent underflow or out-of-bounds access.

## 4. Feature Implementation Details

### 4.1 Discovery
- **Initiator:** Generates a Discovery Inquiry containing its MUID, Device ID, and capabilities (Protocol, Profile, Property Exchange).
- **Responder:** Upon receiving an Inquiry, generates a Discovery Reply with its own MUID and capabilities.

### 4.2 Protocol Negotiation
- Simulates the switch from MIDI 1.0 to MIDI 2.0 (or vice versa).
- Generates Protocol Inquiry and Protocol Reply messages.

### 4.3 Profile Configuration
- Simulates checking and enabling/disabling specific instrument profiles.
- Generates Profile Inquiry, Profile Reply, Set Profile On, and Set Profile Off messages.

## 5. Testing & Verification

### 5.1 Atomic Test Definitions (`el_ci_sim/tests/`)
1. **SysEx Bounded Reassembly:** Feed a sequence of MT=0x3 UMPs representing a 1500-byte payload into a reassembler with a 1024-byte static limit. Assert the reassembler correctly halts and returns an error without panicking or allocating.
2. **Discovery Handshake:** Create an Initiator and a Responder state machine. Feed the Initiator's output UMPs directly to the Responder. Assert both machines correctly transition to the `DiscoveryReceived` / `Ready` states.
3. **Offset Validation:** Create a malformed Discovery Inquiry UMP missing the MUID fields. Assert the parser correctly rejects the message via bounds checking instead of accessing out-of-bounds data.

### 5.2 Integration Tests
- Run `el_ci_sim --role initiator` and capture stdout output.
- Pass the output to `el_dump` (from Phase A) to visually verify the SysEx bytes match the MIDI-CI Universal SysEx specifications.

## 6. Execution Steps
1. Add `el_ci_sim` to the workspace.
2. Add `heapless` and `clap` dependencies to `el_ci_sim`.
3. Implement `sysex_helpers.rs` for strictly bounded chunking and reassembly.
4. Implement the `initiator.rs` and `responder.rs` state machines.
5. Implement the main CLI execution loop in `main.rs`.
6. Write atomic tests focusing heavily on security bounds-checking during payload processing.