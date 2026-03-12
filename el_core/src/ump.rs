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
    Reserved8 = 0x8,
    Reserved9 = 0x9,
    ReservedA = 0xA,
    ReservedB = 0xB,
    ReservedC = 0xC,
    ReservedD = 0xD,
    ReservedE = 0xE,
    UmpStream = 0xF,
}

impl MessageType {
    /// Bypasses branching for packet length lookups, using the exact bounds mapped in our memory.
    #[must_use]
    pub const fn word_count(&self) -> usize {
        const WORD_COUNTS: [u8; 16] = [
            1, // 0x0 Utility
            1, // 0x1 System
            1, // 0x2 MIDI 1.0 Voice
            2, // 0x3 Data 64-bit
            2, // 0x4 MIDI 2.0 Voice
            4, // 0x5 Data 128-bit
            1, // 0x6 Reserved
            1, // 0x7 Reserved
            2, // 0x8 Reserved
            2, // 0x9 Reserved
            2, // 0xA Reserved
            3, // 0xB Reserved
            3, // 0xC Reserved
            4, // 0xD Reserved
            4, // 0xE Reserved
            4, // 0xF Stream
        ];
        WORD_COUNTS[*self as usize] as usize
    }
}

/// The Universal MIDI Packet
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Ump {
    pub data: [u32; 4],
}

impl Ump {
    #[must_use]
    pub fn new(w1: u32, w2: u32, w3: u32, w4: u32) -> Self {
        Self {
            data: [w1, w2, w3, w4],
        }
    }

    #[must_use]
    pub fn message_type(&self) -> MessageType {
        let mt_val = ((self.data[0] >> 28) & 0xF) as usize;
        // Safety constraint: MT is bounded to 4 bits (0x0 - 0xF),
        // so we use a direct array lookup without branch overhead.
        const TYPES: [MessageType; 16] = [
            MessageType::Utility,
            MessageType::System,
            MessageType::Midi1ChannelVoice,
            MessageType::Data64,
            MessageType::Midi2ChannelVoice,
            MessageType::Data128,
            MessageType::Reserved6,
            MessageType::Reserved7,
            MessageType::Reserved8,
            MessageType::Reserved9,
            MessageType::ReservedA,
            MessageType::ReservedB,
            MessageType::ReservedC,
            MessageType::ReservedD,
            MessageType::ReservedE,
            MessageType::UmpStream,
        ];
        TYPES[mt_val]
    }

    #[must_use]
    pub fn group(&self) -> u8 {
        ((self.data[0] >> 24) & 0xF) as u8
    }

    #[must_use]
    pub fn word_count(&self) -> usize {
        self.message_type().word_count()
    }
}
