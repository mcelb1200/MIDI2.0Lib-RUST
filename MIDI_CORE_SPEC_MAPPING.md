# Comprehensive MIDI 2.0 Specification & Property Exchange Mapping

## 1. Objective
To explicitly map out the internal data structures, constants, and enumerations required to comprehensively support the Expanded Messages List (Status Bytes), Universal System Exclusive Messages, the foundational overarching MIDI 2.0 and MIDI-CI specifications, and the specific suite of MIDI-CI Property Exchange (PE) Specifications within the zero-allocation `no_std` `el_core` framework.

## 2. Foundational MIDI 2.0 & Protocol Specifications

### 2.1 M2-100: MIDI 2.0 Specification Overview
This specification provides the overarching architecture bridging MIDI 1.0 legacy devices with MIDI 2.0.
- **Implementation Mapping:** The `el_bridge` protocol translator (defined in Phase B) physically manifests this specification. The abstraction of "Groups" (16 concurrent streams) is embedded directly into the `Ump` primitive via the `group()` bitmask getter, rather than requiring separate port instances.

### 2.2 M2-104: UMP and MIDI 2.0 Protocol Specification
This defines the 32-bit word packet standard and the exact bit-layout of High-Resolution messages.
- **Implementation Mapping:** The `[u32; 4]` `Ump` struct in `el_core::ump` maps this exactly. The `el_core::utils` module enforces the specific Bit-Duplication scaling algorithms defined in this specification (e.g., repeating lower bits when scaling 7-bit to 32-bit values).

### 2.3 M2-116: MIDI Clip File Specification (`.midiclip`)
Defines how MIDI 2.0 sequences are stored in a file, integrating UMPs with SMPTE or Tick-based timestamps.
- **Implementation Mapping:** To be supported by `el_dump` and `el_gen`. We define a `MidiClipHeader` struct to parse the "SMF2" magic bytes and chunk lengths. The `el_core::parser::UmpStreamParser` will be augmented to accept an iterator of `(Timestamp, Ump)` tuples when operating on a `.midiclip` byte slice, strictly adhering to zero-allocation memory mapping (mmap).

## 3. Foundational MIDI-CI Specifications

### 3.1 M2-101: MIDI-CI Specification
Defines the three pillars of Capability Inquiry: Protocol Negotiation, Profile Configuration, and Property Exchange.
- **Implementation Mapping:** Represented by the `CiRoleState` state machine. Defines the Universal SysEx structure (`0x7E`, Device ID, `0x0D`, SubID2, MUIDs) implemented via `CiSysExReassembler`.

### 3.2 M2-102: Common Rules for MIDI-CI Profiles
Defines how devices assume specific functional roles (e.g., "Drawbar Organ" or "Analog Synth").
- **Implementation Mapping:** Defines the `ProfileId` struct as a 5-byte identifier (1-byte standard/manufacturer flag + 2-byte bank + 2-byte number).
```rust
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ProfileId {
    pub is_standard: bool,
    pub bank: u16,
    pub index: u16,
}
```
`el_core` tracks enabled profiles via a bounded array `[Option<ProfileId>; 8]` per channel, rejecting Profile Activation requests if the pool is full or the profile is unsupported.

## 4. Expanded Messages List (Status Bytes)

The MIDI 1.0/2.0 protocol categorizes messages via Status Bytes. To avoid magic numbers and bounds-checking panics, we define exhaustive `#[repr(u8)]` enums.

### 4.1 Channel Voice Status Bytes (0x80 - 0xEF)
Used in both MT=0x2 (MIDI 1.0) and MT=0x4 (MIDI 2.0).

```rust
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ChannelVoiceStatus {
    NoteOff = 0x80,
    NoteOn = 0x90,
    PolyphonicKeyPressure = 0xA0,
    ControlChange = 0xB0,        // Includes Mode Messages (Data1 120-127)
    ProgramChange = 0xC0,
    ChannelPressure = 0xD0,
    PitchBend = 0xE0,
    // MIDI 2.0 Specific (MT=0x4)
    RegisteredPerNoteController = 0x00,     // Status in byte 2
    AssignablePerNoteController = 0x01,     // Status in byte 2
    RegisteredParameterNumber = 0x02,       // Status in byte 2
    AssignableParameterNumber = 0x03,       // Status in byte 2
    RelativeRegisteredParameterNumber = 0x04,
    RelativeAssignableParameterNumber = 0x05,
}
```

### 4.2 System Common & Real-Time Status Bytes (0xF0 - 0xFF)
Used in MT=0x1 (System Messages) and MT=0x3/0x5 (SysEx).

```rust
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SystemMessageStatus {
    // Common
    SystemExclusiveStart = 0xF0, // Used to trigger SysEx chunking
    MidiTimeCodeQuarterFrame = 0xF1,
    SongPositionPointer = 0xF2,
    SongSelect = 0xF3,
    TuneRequest = 0xF6,
    SystemExclusiveEnd = 0xF7,

    // Real-Time
    TimingClock = 0xF8,
    Start = 0xFA,
    Continue = 0xFB,
    Stop = 0xFC,
    ActiveSensing = 0xFE,
    SystemReset = 0xFF,
}
```

### 4.3 Channel Mode Messages (CC 120-127)
Defined when `ControlChange (0xB0)` has a Data1 byte between 120-127.

```rust
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ChannelModeMessage {
    AllSoundOff = 120,
    ResetAllControllers = 121,
    LocalControl = 122,      // Data2 = 0 (Off), 127 (On)
    AllNotesOff = 123,
    OmniModeOff = 124,
    OmniModeOn = 125,
    MonoModeOn = 126,        // Data2 = M (Number of channels)
    PolyModeOn = 127,
}
```

## 5. Universal System Exclusive Messages

Universal SysEx messages are categorized into Non-Real-Time (0x7E) and Real-Time (0x7F).

```rust
pub struct UniversalSysEx;

impl UniversalSysEx {
    pub const NON_REAL_TIME_ID: u8 = 0x7E;
    pub const REAL_TIME_ID: u8 = 0x7F;

    // Sub-ID #1 (Categories)
    pub const SUBID1_SAMPLE_DUMP: u8 = 0x01;
    pub const SUBID1_DEVICE_INQUIRY: u8 = 0x06;
    pub const SUBID1_MIDI_TUNING: u8 = 0x08;
    pub const SUBID1_GENERAL_MIDI: u8 = 0x09;
    pub const SUBID1_MIDI_CI: u8 = 0x0D; // Core to Phase C Protocol Negotiation
}

// Data structures mapping exact Sub-ID #2 formats for MIDI-CI (0x0D)
#[repr(u8)]
pub enum MidiCiSubId2 {
    DiscoveryInquiry = 0x70,
    DiscoveryReply = 0x71,
    EndpointInfoInquiry = 0x72,
    EndpointInfoReply = 0x73,
    ProtocolNegotiationInquiry = 0x10,
    ProtocolNegotiationReply = 0x11,
    SetNewProtocol = 0x12,
    ProfileInquiry = 0x20,
    ProfileInquiryReply = 0x21,
    SetProfileOn = 0x22,
    SetProfileOff = 0x23,
    ProfileEnabledReport = 0x24,
    ProfileDisabledReport = 0x25,
    PropertyExchangeInquiry = 0x30,
    PropertyExchangeReply = 0x31,
    PropertyExchangeNotify = 0x32,
}
```

## 6. MIDI-CI Property Exchange (PE) Specifications Mapping

To handle the extensive suite of PE documents without dynamic allocation, we define statically sized, strict struct schemas that our SAX-style JSON tokenizer maps into directly.

### 6.1 M2-103: Common Rules for Property Exchange
Establishes the foundation of PE Headers (JSON).
- **Implementation:** `el_core::ci::pe::header::PeHeaderSchema` defines static slices for `resource`, `resId`, `action`, `mutability`, and `status`. Extends the `PeTransactionManager` to track chunked `Request IDs` safely.

### 6.2 M2-105: Foundational Resources
Covers `DeviceInfo`, `ResourceList`, `SubscriptionList`.
- **Implementation:** The `ResourceList` payload will be generated procedurally (streaming strings) rather than allocating a monolithic JSON array, ensuring a fixed memory footprint.

### 6.3 M2-106: Mode Resources
Covers Global vs Channel modes (e.g., MPE vs standard MIDI).
- **Implementation:** `struct ModeResourceState` handles string-matching for `"Mode"` resources, parsing specific enums `["Standard", "MPE", "MultiChannel"]`.

### 6.4 M2-107: ProgramList Resource
Allows clients to retrieve the names and banks of presets.
- **Implementation:** Given Program Lists can contain thousands of entries, `el_core` will implement a `ProgramListIterator` that streams chunks of `{ "program": N, "bank": B, "title": "Name" }` objects directly to the `PeReplyChunker` without buffering the entire JSON array in memory.

### 6.5 M2-108: Channel Resources
Describes properties scoped strictly to a single MIDI channel.
- **Implementation:** Resource strings will be matched using a prefix scanner. E.g., `ChannelList/0` extracts the integer `0` to route to `[ChannelState; 16]`.

### 6.6 M2-109: LocalOn Resource
Correlates to `ChannelModeMessage::LocalControl` but via Property Exchange.
- **Implementation:** Bound to a global boolean `static mut LOCAL_ON: bool` (protected via atomic access in embedded contexts). Modifying this property automatically triggers a PE Notification to subscribers.

### 6.7 M2-111: Get and Set Device State
Covers bulk backup and restore of device memory.
- **Implementation:** Because Device States can be Megabytes in size, the `PeReplyChunker` will directly interface with underlying Flash/EEPROM memory via a `Read` trait, streaming raw binary payloads inside the PE Body chunk-by-chunk using MT=0x5 UMPs.

### 6.8 M2-112: ExternalSync Resource
Handles properties defining Clock Source, Transport controls, and MIDI Time Code synchronization.
- **Implementation:** Maps to `struct SyncResourceState { source: SyncSourceEnum, active: bool }`. Validates JSON `"source"` against `["Internal", "MIDI", "SMPTE", "Audio"]`.

### 6.9 M2-117: Controller Resources
Extends high-resolution CC mapping with string names, default values, and unit definitions (e.g., "Hz", "dB").
- **Implementation:** A statically bounded array `[ControllerDefinition; 128]` defining CC meta-data. Queries to `"ControllerList"` stream this array into JSON chunks.

## 7. MIDI 2.0 Physical Hardware Transports for Embedded Systems

### 7.1 USB-MIDI 2.0 (Universal Serial Bus Device Class Definition for MIDI Devices v2.0)
Defines how Universal MIDI Packets (UMP) are transported across USB bulk/interrupt endpoints.
- **Group Terminal Blocks (GTB):** Describes the logical routing of UMP groups to physical or virtual jacks.
```rust
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GroupTerminalBlockType {
    Bidirectional = 0x00,
    InputOnly = 0x01,
    OutputOnly = 0x02,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GroupTerminalBlock {
    pub gtb_id: u8,
    pub block_type: GroupTerminalBlockType,
    pub group_first: u8,
    pub group_count: u8,
    pub midi_protocol: u8, // 1 = MIDI 1.0, 2 = MIDI 2.0
    pub max_sysex8_streams: u8,
}
```
- **Embedded USB Framing:** USB-MIDI 2.0 sends 32-bit UMP words Little Endian across USB endpoints. Each USB packet contains an integral number of UMP words (up to max packet size, typically 64, 256, or 512 bytes).

### 7.2 Serial DIN & TRS Hardware Transport (UART @ 31,250 baud or High-Speed)
Traditional 5-pin DIN or 3.5mm TRS jacks transmit raw MIDI 1.0 byte streams or high-speed UMP serial streams.
- **Bidirectional Byte-to-UMP State Machine:**
```rust
pub struct ByteStreamToUmpParser {
    running_status: u8,
    expected_bytes: u8,
    buffer: [u8; 3],
    buf_len: u8,
}
```
- **Real-Time Byte Interleaving:** In standard MIDI 1.0 streams, 1-byte System Real-Time messages (`0xF8` Clock, `0xFE` Active Sense, `0xFA` Start) can be interleaved anywhere inside a multi-byte message (even mid-SysEx or mid-NoteOn). The embedded parser immediately yields a 32-bit MT=0x1 UMP without corrupting the multi-byte accumulator.

### 7.3 Embedded HAL Trait Integration (`embedded-hal`)
To make `el_midi2` universally portable across ARM Cortex-M (STM32, RP2040, nRF52), RISC-V (ESP32-C3, GD32), and AVR microcontrollers:
```rust
pub trait EmbeddedTransport {
    type Error: core::fmt::Debug;

    /// Attempts to read incoming UMP packets without blocking.
    fn read_ump(&mut self) -> Result<Option<Ump>, Self::Error>;

    /// Sends a single UMP packet.
    fn write_ump(&mut self, ump: &Ump) -> Result<(), Self::Error>;

    /// Flushes any pending outbound buffers.
    fn flush(&mut self) -> Result<(), Self::Error>;
}
```

## 8. Embedded Device Architecture & Memory Budgets

### 8.1 Static Memory Footprint Allocations
Embedded microcontrollers have strict RAM limitations (often 16KB to 256KB total SRAM). All buffers and state machines in `el_midi2` operate within guaranteed bounds:

| Component | Embedded Allocation | Purpose |
|---|---|---|
| `Ump` | 16 bytes (`[u32; 4]`) | Raw packet container |
| `SysEx7Receiver<CAP>` | `CAP` bytes (default 128-512) | Bounded SysEx7 collector |
| `SysEx8Receiver<CAP>` | `CAP` bytes (default 256-1024) | High-speed SysEx8 / MDS collector |
| `StreamTextReceiver<CAP>` | 128 bytes | Endpoint / FB string reassembly |
| `FunctionBlock` (per instance) | 8 bytes | Internal device topology |
| `CiInitiator` / `CiResponder` | ~128 bytes | Capability Inquiry state machine |
| `PeTransactionManager<8>` | ~96 bytes | Concurrent Property Exchange requests |
| `SubscriptionManager<8>` | 64 bytes | Active subscriber resource hashes |

### 8.2 MISRA 2023 / High-Integrity Safety Guarantees
- `#![no_std]` core with zero dynamic heap allocation in normal operating paths.
- `#![forbid(unsafe_code)]` in all parsers, builders, and state machines.
- Integer shift guards ensuring shift amounts never equal or exceed bit width (`< 32` for `u32`).
- Branchless mask generation and jump-table optimized exhaustive pattern matches.
- All public structs implement `Copy`, `Clone`, `Debug`, `PartialEq`, and `Eq` where applicable.
