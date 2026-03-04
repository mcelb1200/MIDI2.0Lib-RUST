The following document is designed to be your Master Development Log. It serves two purposes:
1. **Project Management:** It breaks the massive MIDI 2.0 spec into atomic, sequential coding sessions.
2. **Context Injection:** It provides the technical constraints and definitions "Jules" (the AI) needs to maintain consistency across sessions.

**Note:** You should save this as MIDI2_ROADMAP.md in your repository root. At the start of each new AI session, copy the "Global Context" section and the specific "Task Definition" for that session.

# MIDI 2.0 Rust Implementation Pipeline

**Project Goal:** Build a fully compliant, no_std compatible, idiomatic Rust library for MIDI 2.0 (UMP, MIDI-CI, Transport Agnostic).

**Status:** 🔄 In Progress

## 🛠 Global Context (Paste at start of every session)

**Context for AI Session:**
We are building `am_midi2` (formerly midi2-rs), a comprehensive MIDI 2.0 library in Rust.

**Core Constraints:**
* **no_std Support:** The core library must support no_std (embedded targets). Use alloc feature flags only where necessary (e.g., large SysEx buffers).
* **Zero-Copy:** Prefer passing slices `&[u32]` and modifying Ump structs in place rather than heap allocation.
* **Strong Typing:** Use the "New Type" pattern or Enums to prevent invalid states (e.g., `struct Velocity(u16)`).
* **Endianness:** MIDI 2.0 UMP is always Little Endian on the wire (32-bit words). Ensure bit-shifting logic respects this.

**Architecture:**
* `ump.rs`: The raw container (`[u32; 4]`).
* `messages/`: Strongly typed builders (Voice, System, Utility).
* `ci/`: Capability Inquiry state machines and SysEx helpers.
* `transport/`: Traits for sending/receiving data.

## 📅 Phase 1: The Foundation (UMP & Primitive Types)
**Goal:** Establish the bit-level packet handling. No application logic yet.

### ✅ Session 1.1: The UMP Container
**Prompt:** "Implement the Ump struct wrapping `[u32; 4]`. Implement helper methods to get/set the Message Type (MT) and Group. Create the `MessageType` enum covering all 16 MIDI 2.0 types (Utility, System, MIDI 1.0, MIDI 2.0, Data, Stream, etc.). Ensure Debug and Display traits are useful for hex dumping."
**Definition of Done:** `Ump::new()`, `ump.message_type()`, and `ump.group()` work in unit tests.

### ✅ Session 1.2: Bit Manipulation Utilities
**Prompt:** "Create a utility module for bit packing. We need functions to extract/inject 7-bit, 14-bit, 16-bit, and 32-bit values into the UMP words. MIDI 2.0 relies heavily on splitting data across 32-bit boundaries. Add tests ensuring endianness is handled correctly."
**Definition of Done:** Helper functions `scale_up`, `scale_down`, etc., are tested and panic-free.

## 🎹 Phase 2: Protocol Logic (Voice Messages)
**Goal:** Ability to generate and parse musical data.

### ✅ Session 2.1: MIDI 2.0 Channel Voice (Note On/Off)
**Prompt:** "Implement the voice module. create builders/parsers for NoteOn and NoteOff. These are Message Type 0x4. Critical: Support 16-bit velocity and the Attribute fields (Attribute Type + Attribute Data). Use the builder pattern (e.g., `NoteOn::new(channel, note).velocity(v).build()`)."
**Definition of Done:** Unit tests verifying bit-perfect hex output against the spec examples.

### ✅ Session 2.2: Controllers & Pitch Bend
**Prompt:** "Extend the voice module. Implement ControlChange (Status 0xB), ProgramChange (Status 0xC), ChannelPressure (Status 0xD), and PitchBend (Status 0xE). Note: In MIDI 2.0, CC and Pitch Bend are 32-bit values. Ensure the API accepts u32 for these values."
**Definition of Done:** Ability to build a full suite of performance messages.

### ✅ Session 2.3: Jitter Reduction & Utility Messages
**Prompt:** "Implement Message Type 0x0 (Utility). Specifically, the JitterReductionClock and JitterReductionTimestamp messages. Implement the logic to prepend these timestamps to a Ump packet."
**Definition of Done:** Utility enum and logic to wrap a message in a JR timestamp.

## 🤝 Phase 3: MIDI-CI (Capability Inquiry)
**Goal:** The handshake. This is the hardest part involving multi-packet logic.

### ⬜ Session 3.1: System Exclusive (SysEx) Manager
**Prompt:** "Implement the sysex module. MIDI 2.0 sends SysEx in 64-bit (MT 0x3) or 128-bit (MT 0x5) Data packets. We need a SysExBuilder that takes a `&[u8]` payload and returns an `Iterator<Item = Ump>` to chunk the data into packets (Start, Continue, End). Also need a SysExCollector state machine to reassemble incoming packets."
**Definition of Done:** A test that takes a long string, chunks it into 10 UMPs, and reassembles it back to the string.

### ⬜ Session 3.2: Discovery & Protocol Negotiation
**Prompt:** "Implement the ci module basics. Define the Universal SysEx header for MIDI-CI (0x7E, 0x0D, ...). Create a struct CiDiscovery that generates the 'Discovery Inquiry' message. Create a state machine CiInitiator that transitions: Uninitialized -> DiscoverySent -> DiscoveryReceived -> Negotiating."
**Definition of Done:** A state machine that advances when fed simulated valid CI responses.

### ⬜ Session 3.3: Profile Configuration
**Prompt:** "Implement MIDI-CI Profile Configuration messages. We need struct definitions for ProfileInquiry, ProfileOn, ProfileOff, and ProfileEnabledReport. These use the SysEx mechanism built in Session 3.1."
**Definition of Done:** Ability to generate the SysEx payload for turning on a specific Profile ID.

## 🔌 Phase 4: Stream & Transport
**Goal:** Discovery of endpoints and sending data.

### ⬜ Session 4.1: UMP Stream Messages
**Prompt:** "Implement Message Type 0xF (Stream). This includes EndpointDiscovery, EndpointInfo, DeviceIdentity, and FunctionBlockInfo. These are crucial for the DAW to know what the device is (e.g., 'I am a synth on Group 1')."
**Definition of Done:** Structs that serialize to valid MT 0xF UMPs.

### ⬜ Session 4.2: Function Blocks
**Prompt:** "Implement a FunctionBlock struct. This helps a device describe its internal topology. It needs to store: Block Direction (Input/Output), MI (MIDI 1/2), Group First, and Group Count."
**Definition of Done:** A helper that generates the Function Block Info Notification message.

### ⬜ Session 4.3: Transport Trait
**Prompt:** "Define a Transport trait. It should have `send(&[Ump])` and `recv() -> Option<Ump>`. Implement a MockTransport for testing. Then, sketch out how this would integrate with a Midi2Device struct that holds the CI state and the Transport."
**Definition of Done:** A high-level integration test where a Device sends a Discovery message via MockTransport.

## ✅ Progress Log

- Initialized Rust port with Core primitives and utility methods, effectively completing Phase 1 and 2.
- Transitioned CI/CD workflows and started developing a `TOOLS_PLAN.md` for broader standard compliance and tooling support.
