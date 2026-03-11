# Comprehensive MIDI 2.0 Specification & Property Exchange Mapping

## 1. Objective
To explicitly map out the internal data structures, constants, and enumerations required to comprehensively support the Expanded Messages List (Status Bytes), Universal System Exclusive Messages, and the specific suite of MIDI-CI Property Exchange (PE) Specifications (M2-103 through M2-117) within the zero-allocation `no_std` `el_core` framework.

## 2. Expanded Messages List (Status Bytes)

The MIDI 1.0/2.0 protocol categorizes messages via Status Bytes. To avoid magic numbers and bounds-checking panics, we define exhaustive `#[repr(u8)]` enums.

### 2.1 Channel Voice Status Bytes (0x80 - 0xEF)
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

### 2.2 System Common & Real-Time Status Bytes (0xF0 - 0xFF)
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

### 2.3 Channel Mode Messages (CC 120-127)
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

## 3. Universal System Exclusive Messages

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

## 4. MIDI-CI Property Exchange (PE) Specifications Mapping

To handle the extensive suite of PE documents without dynamic allocation, we define statically sized, strict struct schemas that our SAX-style JSON tokenizer maps into directly.

### 4.1 M2-103: Common Rules for Property Exchange
Establishes the foundation of PE Headers (JSON).
- **Implementation:** `el_core::ci::pe::header::PeHeaderSchema` defines static slices for `resource`, `resId`, `action`, `mutability`, and `status`.

### 4.2 M2-105: Foundational Resources
Covers `DeviceInfo`, `ResourceList`, `SubscriptionList`.
- **Implementation:** The `ResourceList` payload will be generated procedurally (streaming strings) rather than allocating a monolithic JSON array, ensuring a fixed memory footprint.

### 4.3 M2-106: Mode Resources
Covers Global vs Channel modes (e.g., MPE vs standard MIDI).
- **Implementation:** `struct ModeResourceState` handles string-matching for `"Mode"` resources, parsing specific enums `["Standard", "MPE", "MultiChannel"]`.

### 4.4 M2-107: ProgramList Resource
Allows clients to retrieve the names and banks of presets.
- **Implementation:** Given Program Lists can contain thousands of entries, `el_core` will implement a `ProgramListIterator` that streams chunks of `{ "program": N, "bank": B, "title": "Name" }` objects directly to the `PeReplyChunker` without buffering the entire JSON array in memory.

### 4.5 M2-108: Channel Resources
Describes properties scoped strictly to a single MIDI channel.
- **Implementation:** Resource strings will be matched using a prefix scanner. E.g., `ChannelList/0` extracts the integer `0` to route to `[ChannelState; 16]`.

### 4.6 M2-109: LocalOn Resource
Correlates to `ChannelModeMessage::LocalControl` but via Property Exchange.
- **Implementation:** Bound to a global boolean `static mut LOCAL_ON: bool` (protected via atomic access in embedded contexts). Modifying this property automatically triggers a PE Notification to subscribers.

### 4.7 M2-111: Get and Set Device State
Covers bulk backup and restore of device memory.
- **Implementation:** Because Device States can be Megabytes in size, the `PeReplyChunker` will directly interface with underlying Flash/EEPROM memory via a `Read` trait, streaming raw binary payloads inside the PE Body chunk-by-chunk using MT=0x5 UMPs.

### 4.8 M2-112: ExternalSync Resource
Handles properties defining Clock Source, Transport controls, and MIDI Time Code synchronization.
- **Implementation:** Maps to `struct SyncResourceState { source: SyncSourceEnum, active: bool }`. Validates JSON `"source"` against `["Internal", "MIDI", "SMPTE", "Audio"]`.

### 4.9 M2-117: Controller Resources
Extends high-resolution CC mapping with string names, default values, and unit definitions (e.g., "Hz", "dB").
- **Implementation:** A statically bounded array `[ControllerDefinition; 128]` defining CC meta-data. Queries to `"ControllerList"` stream this array into JSON chunks.