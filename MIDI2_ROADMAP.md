The following document is designed to be your Master Development Log. It serves two purposes:
1. **Project Management:** It breaks the massive MIDI 2.0 spec into atomic, sequential coding sessions.
2. **Context Injection:** It provides the technical constraints and definitions "Jules" (the AI) needs to maintain consistency across sessions.

**Note:** You should save this as MIDI2_ROADMAP.md in your repository root. At the start of each new AI session, copy the "Global Context" section and the specific "Task Definition" for that session.

# MIDI 2.0 Rust Implementation Pipeline

**Project Goal:** Build a fully compliant, no_std compatible, idiomatic Rust library for MIDI 2.0 (UMP, MIDI-CI, Transport Agnostic).

**Status:** 🔄 In Progress

## 🛠 Global Context (Paste at start of every session)

**Context for AI Session:**
We are building `el_midi2` (formerly midi2-rs), a comprehensive MIDI 2.0 library in Rust.

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

### ✅ Session 3.1: System Exclusive (SysEx) Manager
**Prompt:** "Implement the sysex module. MIDI 2.0 sends SysEx in 64-bit (MT 0x3) or 128-bit (MT 0x5) Data packets. We need a SysExBuilder that takes a `&[u8]` payload and returns an `Iterator<Item = Ump>` to chunk the data into packets (Start, Continue, End). Also need a SysExCollector state machine to reassemble incoming packets."
**Definition of Done:** A test that takes a long string, chunks it into 10 UMPs, and reassembles it back to the string.

### ✅ Session 3.2: Discovery & Protocol Negotiation
**Prompt:** "Implement the ci module basics. Define the Universal SysEx header for MIDI-CI (0x7E, 0x0D, ...). Create a struct CiDiscovery that generates the 'Discovery Inquiry' message. Create a state machine CiInitiator that transitions: Uninitialized -> DiscoverySent -> DiscoveryReceived -> Negotiating."
**Definition of Done:** A state machine that advances when fed simulated valid CI responses.

### ✅ Session 3.3: Profile Configuration
**Prompt:** "Implement MIDI-CI Profile Configuration messages. We need struct definitions for ProfileInquiry, ProfileOn, ProfileOff, and ProfileEnabledReport. These use the SysEx mechanism built in Session 3.1."
**Definition of Done:** Ability to generate the SysEx payload for turning on a specific Profile ID.

## 🔌 Phase 4: Stream & Transport
**Goal:** Discovery of endpoints and sending data.

### ✅ Session 4.1: UMP Stream Messages
**Prompt:** "Implement Message Type 0xF (Stream). This includes EndpointDiscovery, EndpointInfo, DeviceIdentity, and FunctionBlockInfo. These are crucial for the DAW to know what the device is (e.g., 'I am a synth on Group 1')."
**Definition of Done:** Structs that serialize to valid MT 0xF UMPs.

### ✅ Session 4.2: Function Blocks
**Prompt:** "Implement a FunctionBlock struct. This helps a device describe its internal topology. It needs to store: Block Direction (Input/Output), MI (MIDI 1/2), Group First, and Group Count."
**Definition of Done:** A helper that generates the Function Block Info Notification message.

### ✅ Session 4.3: Core Transport Traits & Mock Transport
**Prompt:** "Define a `Transport` trait and `AsyncTransport` trait suitable for both synchronous polling and interrupt/DMA driven embedded runtimes. Methods: `send(&[Ump]) -> Result<(), TransportError>` and `recv() -> Option<Ump>`. Implement a zero-allocation `MockTransport` with bounded internal ringbuffers for testing. Add integration test verifying endpoint discovery over `MockTransport`."
**Definition of Done:** `Transport` trait and `MockTransport` tested and working with zero allocations.

## 📦 Phase 5: MIDI-CI Property Exchange (PE) & Process Inquiry
**Goal:** Full bidirectional parameter exchange, discovery, and state synchronisation under strict embedded `no_std` constraints.

### ✅ Session 5.1: Zero-Allocation SAX/Streaming JSON Tokenizer
**Prompt:** "Implement a lightweight, no_std SAX-style JSON tokenizer in `ci::pe::json`. Capable of parsing Property Exchange headers and simple value bodies directly from byte slices without memory allocation (`alloc`). Support extracting string slices for `resource`, `resId`, `action`, `status`, and integer values."
**Definition of Done:** Tokenizer extracts keys and values from realistic PE JSON header payloads with zero heap allocation.

### ✅ Session 5.2: PE Transaction Manager & Request Pools
**Prompt:** "Implement `PeTransactionManager<const MAX_CONCURRENT: usize>` to manage asynchronous Request IDs without hash maps. Support matching Inquiries (Get, Set, Subscribe) to Replies and handling timeouts. Implement `PeHeader` and `PeMessage` view types."
**Definition of Done:** Unit test exercising concurrent Request ID allocation, matching, release, and pool exhaustion guards.

### ✅ Session 5.3: Property Chunking, Streaming & Subscriptions
**Prompt:** "Implement `PeReplyChunker` that streams arbitrary property bodies across multiple MT=0x3 (SysEx7) or MT=0x5 (SysEx8) UMP packets. Implement `SubscriptionManager<const MAX_SUBS: usize>` using resource string hashes (`u32`) for low-memory embedded subscriber tracking."
**Definition of Done:** A test that streams a simulated preset file across multiple UMP SysEx chunks and delivers notifications to registered subscribers.

### ✅ Session 5.4: MIDI-CI Process Inquiry (PI)
**Prompt:** "Implement MIDI-CI Process Inquiry messages: MIDI Message Report Inquiry/Reply (Sub-ID 0x40/0x41). This allows querying which MIDI 1.0 or MIDI 2.0 messages (Channel Voice, System Common, SysEx, etc.) are supported across groups and channels. Generate the 7-bit bitmap representation for supported messages."
**Definition of Done:** Serialization and parsing of MIDI Message Report Inquiry and Reply packets.

## 🔌 Phase 6: Embedded Transport Adapters & Hardware Interfaces
**Goal:** Direct hardware protocol bindings for physical microcontroller peripherals.

### ⬜ Session 6.1: USB-MIDI 2.0 Class Specification & Group Terminal Blocks
**Prompt:** "Implement USB-MIDI 2.0 descriptor structures and packet framing in `transport::usb`. Define Group Terminal Block (GTB) descriptors (Standard and Class-Specific) describing group-to-terminal mappings, protocol capabilities (MIDI 1.0 / MIDI 2.0), and bidirectional association. Implement zero-copy USB packet unpackers/packers."
**Definition of Done:** Bit-accurate serialization of Group Terminal Block descriptors and packet framing tests against USB-MIDI 2.0 spec examples.

### ⬜ Session 6.2: Serial UART / DIN / TRS Byte-Stream Transport & Bi-Directional Adapter
**Prompt:** "Implement the UART byte stream bridge (`transport::serial`). Provide `ByteStreamToUmp` and `UmpToByteStream` state machines adhering to `embedded-hal::serial` or raw byte slice buffers. Properly reconstruct running status, handle realtime byte interleaving, and aggregate 14-bit CCs, NRPNs, and SysEx."
**Definition of Done:** Bidirectional tests streaming legacy MIDI 1.0 serial bytes into UMPs and down-converting UMPs into compliant MIDI 1.0 byte streams without latency or heap allocation.

### ⬜ Session 6.3: DMA-Friendly Ringbuffers & Interrupt-Safe Queues
**Prompt:** "Implement bounded, interrupt-safe lock-free SPSC queues (`transport::queue::PacketQueue<const CAP: usize>`) tailored for DMA transfer buffers. Provide zero-copy slice leasing for transmitting and receiving bursts of UMPs directly to/from hardware peripheral FIFOs."
**Definition of Done:** Stress tests verifying lock-free push/pop behavior across simulated concurrent producer and consumer contexts.

## 🎛 Phase 7: Embedded MIDI 2.0 Device Engine & High-Level Runtime
**Goal:** Turnkey embedded device runtime integrating Endpoint, Function Blocks, Profiles, and Transports into a unified system.

### ⬜ Session 7.1: `Midi2Device` Coordinator
**Prompt:** "Implement a unified `Midi2Device<T: Transport, const NUM_FB: usize>` struct. Manages Endpoint info, up to `NUM_FB` Function Blocks, local MUID, CI state machine (Initiator + Responder), and active protocol. Provide a clean polling method `device.poll()` to process incoming UMPs and emit responses."
**Definition of Done:** High-level test simulating complete device startup: Endpoint Discovery -> Stream Config -> Function Block Discovery -> CI Discovery -> Protocol switch to MIDI 2.0.

### ⬜ Session 7.2: Standard Embedded Profile Handlers
**Prompt:** "Implement standard profile managers for common embedded scenarios (e.g., Default Profile, Drawbar Organ, Analog Synth, General MIDI). Provide hooks for parameter updates and profile-specific data callbacks without heap allocations."
**Definition of Done:** Test enabling a standard profile on a Function Block and responding to Profile Inquiries and Profile On/Off messages.

### ⬜ Session 7.3: Standard Resources & Device Identity Provider
**Prompt:** "Implement default Property Exchange resource providers: `DeviceInfo`, `ResourceList`, and `ChannelList`. Implement procedural string generators that write JSON chunks directly into SysEx UMP streams without assembling full JSON strings in RAM."
**Definition of Done:** End-to-end Property Exchange query for `DeviceInfo` yielding a valid, bit-perfect response stream parsed and verified against spec.

## ✅ Progress Log

- Initialized Rust port with Core primitives and utility methods, effectively completing Phase 1 and 2.
- Transitioned CI/CD workflows and developed `TOOLS_PLAN.md` for broader standard compliance and tooling support.
- Completed Phase 3 (SysEx7, SysEx8 / MDS, CI Discovery, Protocol Negotiation, and Profile Configuration).
- Completed Phase 4 (UMP Stream Messages, Endpoint Discovery/Notifications, Function Blocks, Function Block Discovery/Notifications, and Core Transport Traits & Mock Transport).
- Completed Phase 5 (MIDI-CI Property Exchange: Zero-allocation SAX JSON tokenizer, Transaction Manager, Subscription Manager, SysEx Chunker, and Process Inquiry).
- Enriched Master Development Log with complete Phase 6 (USB-MIDI 2.0, Serial UART, DMA queues), and Phase 7 (Unified `Midi2Device` embedded engine).
