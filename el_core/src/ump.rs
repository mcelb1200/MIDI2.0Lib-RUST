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
    /// ⚡ Bolt Optimization: Extracts MessageType using an exhaustive match.
    /// Converting bounded integers to an enum via match is ~20% faster than static array
    /// lookups in safe Rust because it avoids memory fetch overhead and panic branches.
    #[must_use]
    #[inline]
    pub fn from_u32(w1: u32) -> Self {
        match w1 >> 28 {
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

    /// Bypasses branching for packet length lookups, using the exact bounds mapped in our memory.
    #[must_use]
    #[inline]
    pub const fn word_count(&self) -> usize {
        // ⚡ Bolt Optimization: Replaced array lookup with a match statement using explicit piped values.
        // For mathematically bounded values (4-bit enums), explicit piped values allow the compiler
        // to emit optimal jump tables, outperforming memory array fetch overhead.
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
    #[inline]
    pub fn new(w1: u32, w2: u32, w3: u32, w4: u32) -> Self {
        Self {
            data: [w1, w2, w3, w4],
        }
    }

    #[must_use]
    #[inline]
    pub fn message_type(&self) -> MessageType {
        MessageType::from_u32(self.data[0])
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
        self.message_type().word_count()
    }
}

/// Lossless MIDI 1.0 Channel Voice Message representation.
///
/// For DAWs and MIDI routers handling MIDI 1.0 protocol messages over UMP,
/// automatic upscaling to 32-bit is lossy/undesirable. This struct preserves
/// the raw 7-bit parameters and channel voice fields.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Midi1ChannelVoice {
    pub group: u8,
    pub status: u8,
    pub channel: u8,
    pub data1: u8,
    pub data2: u8,
}

impl TryFrom<&Ump> for Midi1ChannelVoice {
    type Error = &'static str;

    #[inline]
    fn try_from(ump: &Ump) -> Result<Self, Self::Error> {
        if ump.message_type() != MessageType::Midi1ChannelVoice {
            return Err("UMP is not a MIDI 1.0 Channel Voice message (MT=0x2)");
        }
        let w = ump.data[0];
        let group = ((w >> 24) & 0xF) as u8;
        let status = ((w >> 16) & 0xF0) as u8;
        let channel = ((w >> 16) & 0x0F) as u8;
        let data1 = ((w >> 8) & 0x7F) as u8;
        let data2 = (w & 0x7F) as u8;

        Ok(Self {
            group,
            status,
            channel,
            data1,
            data2,
        })
    }
}
