//! Comprehensive MIDI 2.0 & Protocol Specification Types
//!
//! Maps standard Channel Voice status bytes, System Common/Real-Time status bytes,
//! Channel Mode messages, Universal System Exclusive IDs, and MIDI-CI Sub-ID #2 formats
//! adhering to M2-100, M2-101, M2-102, and M2-104.

/// Standard Channel Voice status bytes (0x80 - 0xEF) used across MIDI 1.0 (MT=0x2) and MIDI 2.0 (MT=0x4).
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ChannelVoiceStatus {
    NoteOff = 0x80,
    NoteOn = 0x90,
    PolyphonicKeyPressure = 0xA0,
    ControlChange = 0xB0,
    ProgramChange = 0xC0,
    ChannelPressure = 0xD0,
    PitchBend = 0xE0,
    // MIDI 2.0 Specific (MT=0x4, Status in byte 2)
    RegisteredPerNoteController = 0x00,
    AssignablePerNoteController = 0x01,
    RegisteredParameterNumber = 0x02,
    AssignableParameterNumber = 0x03,
    RelativeRegisteredParameterNumber = 0x04,
    RelativeAssignableParameterNumber = 0x05,
}

impl ChannelVoiceStatus {
    /// Attempts to parse a 4-bit status nibble or 8-bit status byte into a `ChannelVoiceStatus`.
    #[must_use]
    pub const fn from_u8(status: u8) -> Option<Self> {
        match status & 0xF0 {
            0x80 => Some(Self::NoteOff),
            0x90 => Some(Self::NoteOn),
            0xA0 => Some(Self::PolyphonicKeyPressure),
            0xB0 => Some(Self::ControlChange),
            0xC0 => Some(Self::ProgramChange),
            0xD0 => Some(Self::ChannelPressure),
            0xE0 => Some(Self::PitchBend),
            _ => match status {
                0x00 => Some(Self::RegisteredPerNoteController),
                0x01 => Some(Self::AssignablePerNoteController),
                0x02 => Some(Self::RegisteredParameterNumber),
                0x03 => Some(Self::AssignableParameterNumber),
                0x04 => Some(Self::RelativeRegisteredParameterNumber),
                0x05 => Some(Self::RelativeAssignableParameterNumber),
                _ => None,
            },
        }
    }
}

/// Standard System Common & Real-Time status bytes (0xF0 - 0xFF).
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SystemMessageStatus {
    // System Common
    SystemExclusiveStart = 0xF0,
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

impl SystemMessageStatus {
    /// Parses a raw status byte into a `SystemMessageStatus`.
    #[must_use]
    pub const fn from_u8(status: u8) -> Option<Self> {
        match status {
            0xF0 => Some(Self::SystemExclusiveStart),
            0xF1 => Some(Self::MidiTimeCodeQuarterFrame),
            0xF2 => Some(Self::SongPositionPointer),
            0xF3 => Some(Self::SongSelect),
            0xF6 => Some(Self::TuneRequest),
            0xF7 => Some(Self::SystemExclusiveEnd),
            0xF8 => Some(Self::TimingClock),
            0xFA => Some(Self::Start),
            0xFB => Some(Self::Continue),
            0xFC => Some(Self::Stop),
            0xFE => Some(Self::ActiveSensing),
            0xFF => Some(Self::SystemReset),
            _ => None,
        }
    }
}

/// Standard Channel Mode Messages (Control Change 120-127).
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ChannelModeMessage {
    AllSoundOff = 120,
    ResetAllControllers = 121,
    LocalControl = 122,
    AllNotesOff = 123,
    OmniModeOff = 124,
    OmniModeOn = 125,
    MonoModeOn = 126,
    PolyModeOn = 127,
}

impl ChannelModeMessage {
    /// Parses a CC number (120-127) into a `ChannelModeMessage`.
    #[must_use]
    pub const fn from_cc(cc: u8) -> Option<Self> {
        match cc {
            120 => Some(Self::AllSoundOff),
            121 => Some(Self::ResetAllControllers),
            122 => Some(Self::LocalControl),
            123 => Some(Self::AllNotesOff),
            124 => Some(Self::OmniModeOff),
            125 => Some(Self::OmniModeOn),
            126 => Some(Self::MonoModeOn),
            127 => Some(Self::PolyModeOn),
            _ => None,
        }
    }
}

/// Universal System Exclusive Constants.
pub struct UniversalSysEx;

impl UniversalSysEx {
    pub const NON_REAL_TIME_ID: u8 = 0x7E;
    pub const REAL_TIME_ID: u8 = 0x7F;

    // Sub-ID #1 (Categories)
    pub const SUBID1_SAMPLE_DUMP: u8 = 0x01;
    pub const SUBID1_DEVICE_INQUIRY: u8 = 0x06;
    pub const SUBID1_MIDI_TUNING: u8 = 0x08;
    pub const SUBID1_GENERAL_MIDI: u8 = 0x09;
    pub const SUBID1_MIDI_CI: u8 = 0x0D;
}

/// MIDI-CI Sub-ID #2 formats mapping the Capability Inquiry specification (M2-101).
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
    ProcessInquiryCapability = 0x40,
    ProcessInquiryCapabilityReply = 0x41,
    MidiMessageReportInquiry = 0x42,
    MidiMessageReportReply = 0x43,
    MidiMessageReportEnd = 0x44,
}

/// MIDI 2.0 Flex Data Status Banks (MT=0xD).
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FlexDataStatusBank {
    Common = 0x00,
    Performance = 0x01,
    Lyric = 0x02,
    ProjectMetadata = 0x03,
    TrackMetadata = 0x04,
}

impl FlexDataStatusBank {
    #[must_use]
    pub const fn from_u8(val: u8) -> Option<Self> {
        match val {
            0x00 => Some(Self::Common),
            0x01 => Some(Self::Performance),
            0x02 => Some(Self::Lyric),
            0x03 => Some(Self::ProjectMetadata),
            0x04 => Some(Self::TrackMetadata),
            _ => None,
        }
    }
}

/// Status codes within Flex Data Status Bank 0x00 (Common).
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FlexDataCommonStatus {
    SetTempo = 0x00,
    SetTimeSignature = 0x01,
    SetMetronome = 0x02,
    SetKeySignature = 0x05,
    SetChord = 0x06,
    BarMarker = 0x07,
    BeatMarker = 0x08,
}

/// Status codes within Flex Data Status Bank 0x03 (Project Metadata).
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FlexDataProjectMetadataStatus {
    ProjectName = 0x00,
    CompositionName = 0x01,
    ArrangerName = 0x02,
    ComposerName = 0x03,
    CopyrightNotice = 0x04,
}

/// Status codes within Flex Data Status Bank 0x04 (Track Metadata).
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FlexDataTrackMetadataStatus {
    TrackName = 0x00,
    ChannelName = 0x01,
    ClipName = 0x02,
    MarkerNote = 0x03,
}

impl MidiCiSubId2 {
    /// Parses a Sub-ID #2 byte into a `MidiCiSubId2`.
    #[must_use]
    pub const fn from_u8(val: u8) -> Option<Self> {
        match val {
            0x70 => Some(Self::DiscoveryInquiry),
            0x71 => Some(Self::DiscoveryReply),
            0x72 => Some(Self::EndpointInfoInquiry),
            0x73 => Some(Self::EndpointInfoReply),
            0x10 => Some(Self::ProtocolNegotiationInquiry),
            0x11 => Some(Self::ProtocolNegotiationReply),
            0x12 => Some(Self::SetNewProtocol),
            0x20 => Some(Self::ProfileInquiry),
            0x21 => Some(Self::ProfileInquiryReply),
            0x22 => Some(Self::SetProfileOn),
            0x23 => Some(Self::SetProfileOff),
            0x24 => Some(Self::ProfileEnabledReport),
            0x25 => Some(Self::ProfileDisabledReport),
            0x30 => Some(Self::PropertyExchangeInquiry),
            0x31 => Some(Self::PropertyExchangeReply),
            0x32 => Some(Self::PropertyExchangeNotify),
            0x40 => Some(Self::ProcessInquiryCapability),
            0x41 => Some(Self::ProcessInquiryCapabilityReply),
            0x42 => Some(Self::MidiMessageReportInquiry),
            0x43 => Some(Self::MidiMessageReportReply),
            0x44 => Some(Self::MidiMessageReportEnd),
            _ => None,
        }
    }
}
