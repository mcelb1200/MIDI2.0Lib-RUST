# MIDI 2.0 Core Architecture Data Structures & Methods

## 1. Objective
This document defines the exact Rust data structures, structs, enums, methods, and pure functions required to implement the entire MIDI 2.0 protocol (UMP, Voice, System, Utility, Data/SysEx, Stream, and MIDI-CI). All structures adhere to strictly `no_std`, zero-dynamic-allocation (MISRA-2023 equivalent) constraints.

## 2. Core Primitives (`el_core::ump`)

### 2.1 The Universal MIDI Packet (UMP)
The foundational primitive representing any message up to 128 bits.

```rust
/// The raw wrapper over 32-bit words, exactly matching network-endian Little Endian.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Ump {
    pub data: [u32; 4],
}

impl Ump {
    #[must_use]
    pub fn new(w1: u32, w2: u32, w3: u32, w4: u32) -> Self;

    #[must_use]
    pub fn message_type(&self) -> MessageType;

    #[must_use]
    pub fn group(&self) -> u8;

    #[must_use]
    pub fn word_count(&self) -> usize;
}
```

### 2.2 Message Types (MT)
Used to safely identify and branch on packet boundaries.

```rust
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MessageType {
    Utility = 0x0,
    System = 0x1,
    Midi1ChannelVoice = 0x2,
    Data64 = 0x3,
    Midi2ChannelVoice = 0x4,
    Data128 = 0x5,
    Reserved6 = 0x6,
    Reserved7 = 0x7,
    // ... 0x8 through 0xE Reserved
    UmpStream = 0xF,
}

impl MessageType {
    /// Bitwise constant lookup instead of branching
    #[must_use]
    pub const fn word_count(&self) -> usize;
}
```

## 3. Bit Manipulation Utilities (`el_core::utils`)
Critical for handling MIDI 2.0 High-Resolution data and translating legacy messages.

```rust
pub mod utils {
    /// Duplicates bits for high-res scaling (e.g., 7-bit to 32-bit) without branching.
    #[must_use]
    pub fn scale_up(value: u32, src_bits: u8, dst_bits: u8) -> u32;

    /// Safely truncates while preserving maximums (e.g., 32-bit to 7-bit).
    #[must_use]
    pub fn scale_down(value: u32, src_bits: u8, dst_bits: u8) -> u32;

    /// Safely joins 14-bit CC MSB and LSB
    #[must_use]
    pub fn join_14bit(msb: u8, lsb: u8) -> u16;
}
```

## 4. Message Builders (`el_core::builder`)

### 4.1 MIDI 1.0 & 2.0 Channel Voice (MT=0x2, MT=0x4)
Used to construct musical data. Uses `scale_up` internally when required.

```rust
pub struct VoiceBuilder;

impl VoiceBuilder {
    // --- MIDI 1.0 (32-bit) ---
    #[must_use]
    pub fn midi1_note_on(group: u8, channel: u8, note: u8, velocity: u8) -> Ump;

    #[must_use]
    pub fn midi1_cc(group: u8, channel: u8, index: u8, value: u8) -> Ump;

    // --- MIDI 2.0 (64-bit) ---
    #[must_use]
    pub fn midi2_note_on(group: u8, channel: u8, note: u8, attr_type: u8, velocity: u16, attr_data: u16) -> Ump;

    #[must_use]
    pub fn midi2_cc(group: u8, channel: u8, index: u8, value: u32) -> Ump;

    #[must_use]
    pub fn midi2_pitch_bend(group: u8, channel: u8, value: u32) -> Ump;

    #[must_use]
    pub fn midi2_nrpn(group: u8, channel: u8, bank: u8, index: u8, value: u32) -> Ump;
}
```

### 4.2 Utility & Clock Messages (MT=0x0)

```rust
pub struct UtilityBuilder;

impl UtilityBuilder {
    #[must_use]
    pub fn noop() -> Ump; // Returns [0, 0, 0, 0]

    #[must_use]
    pub fn jitter_reduction_clock(group: u8, timestamp: u16) -> Ump;

    #[must_use]
    pub fn jitter_reduction_timestamp(group: u8, timestamp: u16) -> Ump;
}
```

### 4.3 UMP Stream Configuration (MT=0xF)
Endpoint discovery and protocol binding.

```rust
pub struct StreamBuilder;

impl StreamBuilder {
    #[must_use]
    pub fn endpoint_discovery(ump_version_major: u8, ump_version_minor: u8, filter: u8) -> Ump;

    #[must_use]
    pub fn protocol_negotiation(protocols: u8, extensions: u8) -> Ump;
}
```

## 5. Bounded SysEx & Data Chunking (MT=0x3, MT=0x5)

Because MIDI 2.0 Data messages are split across 64-bit or 128-bit UMPs, we need alloc-free chunking iterators.

```rust
/// Represents the Status field of a Data Message (Complete, Start, Continue, End)
#[repr(u8)]
pub enum DataStatus {
    Complete = 0x0,
    Start = 0x1,
    Continue = 0x2,
    End = 0x3,
}

/// An iterator that borrows a `&[u8]` slice and yields `Ump` chunks.
/// Emits MT=0x3 (64-bit) or MT=0x5 (128-bit) Data Messages.
pub struct SysExChunker<'a> {
    group: u8,
    stream_id: u8,
    payload: &'a [u8],
    offset: usize,
    use_128bit: bool,
}

impl<'a> Iterator for SysExChunker<'a> {
    type Item = Ump;
    fn next(&mut self) -> Option<Self::Item>; // Emits Start, Continue, End statuses based on `offset`
}
```

## 6. Real-Time Parsers

### 6.1 Stream Parser
Consumes raw `u32` words and safely assembles `Ump` structures.

```rust
pub struct UmpStreamParser<'a> {
    stream: core::slice::Iter<'a, u32>,
}

impl<'a> Iterator for UmpStreamParser<'a> {
    type Item = Ump;
    /// Bypasses enum bounds checks using a const lookup table.
    /// Returns None if a packet truncates early.
    fn next(&mut self) -> Option<Self::Item>;
}
```

## 7. Protocol Translation State Machines (`el_bridge`)

State tracking arrays required to buffer legacy MIDI 1.0 fragmented messages into unified MIDI 2.0 UMPs without allocating memory.

```rust
/// Tracks 14-bit CC fragments (CC# 0-31 waiting for CC# 32-63)
#[derive(Default)]
pub struct Cc14BitState {
    // 32 potential MSB values per channel
    msb_waiting: [Option<u8>; 32],
}

/// Tracks NRPN state machine (CC99 -> CC98 -> CC6 -> CC38)
#[derive(Default)]
pub struct NrpnState {
    msb_index: Option<u8>,
    lsb_index: Option<u8>,
    msb_data: Option<u8>,
}

/// Wraps state for all 16 channels to allow zero-allocation translation
pub struct Midi1ToMidi2Translator {
    group: u8,
    cc_states: [Cc14BitState; 16],
    nrpn_states: [NrpnState; 16],
    sysex_buffer: heapless::Vec<u8, 256>, // Bounded buffer for accumulating legacy 7-bit SysEx
}

impl Midi1ToMidi2Translator {
    /// Takes a legacy MIDI 1.0 byte (Status, Data1, Data2).
    /// If it completes an aggregated message (like NRPN), it returns `Some(Ump)`.
    pub fn process_legacy_message(&mut self, status: u8, d1: u8, d2: u8) -> Option<Ump>;
}
```

## 8. MIDI-CI Protocol Logic (`el_ci_sim`)

### 8.1 Capability Inquiry State Enum
```rust
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CiRoleState {
    Uninitialized,
    DiscoveryInquirySent,
    DiscoveryReplyReceived,
    ProtocolNegotiating,
    ProfileConfiguring,
    Error(CiErrorType),
}
```

### 8.2 Security Bounded SysEx Assembler
A struct to re-assemble MIDI-CI fragmented UMPs safely.

```rust
pub struct CiSysExReassembler<const MAX_BYTES: usize> {
    buffer: heapless::Vec<u8, MAX_BYTES>, // Typically 1024 or 2048
    expected_stream_id: u8,
}

impl<const MAX_BYTES: usize> CiSysExReassembler<MAX_BYTES> {
    /// Pushes a Data chunk (MT=0x3 or MT=0x5).
    /// Returns `Err` immediately if pushing would exceed MAX_BYTES.
    pub fn push_chunk(&mut self, ump: &Ump) -> Result<(), CiErrorType>;

    /// Returns the continuous byte slice once the `End` chunk is received.
    pub fn get_completed_payload(&self) -> Option<&[u8]>;
}
```