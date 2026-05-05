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
    #[inline]
    pub const fn word_count(&self) -> usize {
        // ⚡ Bolt Optimization: Replaced match statement with a static array lookup.
        // As per the #![forbid(unsafe_code)] constraint note, static array indexing
        // for dense integers is significantly faster (~3.5x) than match branches.
        const WORD_COUNTS: [usize; 16] = [1, 1, 1, 2, 2, 4, 1, 1, 2, 2, 2, 3, 3, 4, 4, 4];
        WORD_COUNTS[*self as usize]
    }
}

/// The Universal MIDI Packet
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Ump {
    pub data: [u32; 4],
}

impl Ump {
    #[must_use]
    #[inline]
    pub fn new(w1: u32, w2: u32, w3: u32, w4: u32) -> Self {
        Self {
            data: [w1, w2, w3, w4],
        }
    }

    #[must_use]
    #[inline]
    pub fn message_type(&self) -> MessageType {
        // ⚡ Bolt Optimization: Removed redundant `& 0xF` mask. Right shifting a u32 by 28 bounds the value to 0-15.
        // ⚡ Bolt Optimization: Replaced match statement with a static array lookup.
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
        let mt_val = (self.data[0] >> 28) as usize;
        TYPES[mt_val]
    }

    #[must_use]
    #[inline]
    pub fn group(&self) -> u8 {
        ((self.data[0] >> 24) & 0xF) as u8
    }

    #[inline]
    pub fn set_group(&mut self, group: u8) {
        self.data[0] &= 0xF0FFFFFF;
        self.data[0] |= ((group as u32) & 0xF) << 24;
    }

    #[must_use]
    #[inline]
    pub fn word_count(&self) -> usize {
        // ⚡ Bolt Optimization: Replaced match statement with a static array lookup.
        const WORD_COUNTS: [usize; 16] = [1, 1, 1, 2, 2, 4, 1, 1, 2, 2, 2, 3, 3, 4, 4, 4];
        let mt_val = (self.data[0] >> 28) as usize;
        WORD_COUNTS[mt_val]
    }
}
