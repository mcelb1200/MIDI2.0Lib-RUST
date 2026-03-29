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
        // Direct match grouped logically allows the compiler to generate an
        // optimized branch table that is faster than a secondary array lookup.
        match *self as u8 {
            0x0 | 0x1 | 0x2 | 0x6 | 0x7 => 1,
            0x3 | 0x4 | 0x8 | 0x9 | 0xA => 2,
            0xB | 0xC => 3,
            0x5 | 0xD | 0xE | 0xF => 4,
            _ => unreachable!(),
        }
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
        let mt_val = ((self.data[0] >> 28) & 0xF) as u8;
        // ⚡ Bolt Optimization: Replacing the array lookup with a direct match allows the compiler
        // to generate an optimized branch/jump table that skips the memory load operation entirely.
        // Safety constraint: MT is masked to 4 bits (0x0 - 0xF), exhaustively covered here.
        match mt_val {
            0x0 => MessageType::Utility,
            0x1 => MessageType::System,
            0x2 => MessageType::Midi1ChannelVoice,
            0x3 => MessageType::Data64,
            0x4 => MessageType::Midi2ChannelVoice,
            0x5 => MessageType::Data128,
            0x6 => MessageType::Reserved6,
            0x7 => MessageType::Reserved7,
            0x8 => MessageType::Reserved8,
            0x9 => MessageType::Reserved9,
            0xA => MessageType::ReservedA,
            0xB => MessageType::ReservedB,
            0xC => MessageType::ReservedC,
            0xD => MessageType::ReservedD,
            0xE => MessageType::ReservedE,
            0xF => MessageType::UmpStream,
            _ => unreachable!(),
        }
    }

    #[must_use]
    pub fn group(&self) -> u8 {
        ((self.data[0] >> 24) & 0xF) as u8
    }

    pub fn set_group(&mut self, group: u8) {
        self.data[0] &= 0xF0FFFFFF;
        self.data[0] |= ((group as u32) & 0xF) << 24;
    }

    #[must_use]
    pub fn word_count(&self) -> usize {
        let mt_val = ((self.data[0] >> 28) & 0xF) as u8;
        // Direct match grouped logically allows the compiler to generate an
        // optimized branch table that is faster than a secondary array lookup.
        match mt_val {
            0x0 | 0x1 | 0x2 | 0x6 | 0x7 => 1,
            0x3 | 0x4 | 0x8 | 0x9 | 0xA => 2,
            0xB | 0xC => 3,
            0x5 | 0xD | 0xE | 0xF => 4,
            _ => unreachable!(),
        }
    }
}
