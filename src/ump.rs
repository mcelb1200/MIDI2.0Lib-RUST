use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    Utility = 0x0,
    System = 0x1,
    Midi1ChannelVoice = 0x2,
    SysEx7 = 0x3,
    Midi2ChannelVoice = 0x4,
    Data = 0x5,
    Reserved6 = 0x6,
    Reserved7 = 0x7,
    Reserved8 = 0x8,
    Reserved9 = 0x9,
    ReservedA = 0xA,
    ReservedB = 0xB,
    ReservedC = 0xC,
    FlexData = 0xD,
    ReservedE = 0xE,
    Stream = 0xF,
}

impl MessageType {
    pub fn from_u8(val: u8) -> Self {
         match val {
            0x0 => MessageType::Utility,
            0x1 => MessageType::System,
            0x2 => MessageType::Midi1ChannelVoice,
            0x3 => MessageType::SysEx7,
            0x4 => MessageType::Midi2ChannelVoice,
            0x5 => MessageType::Data,
            0x6 => MessageType::Reserved6,
            0x7 => MessageType::Reserved7,
            0x8 => MessageType::Reserved8,
            0x9 => MessageType::Reserved9,
            0xA => MessageType::ReservedA,
            0xB => MessageType::ReservedB,
            0xC => MessageType::ReservedC,
            0xD => MessageType::FlexData,
            0xE => MessageType::ReservedE,
            0xF => MessageType::Stream,
            _ => MessageType::Utility,
        }
    }

    pub fn word_count(&self) -> usize {
        match self {
            MessageType::Utility | MessageType::System | MessageType::Midi1ChannelVoice => 1,
            MessageType::SysEx7 | MessageType::Midi2ChannelVoice => 2,
            MessageType::Data | MessageType::FlexData | MessageType::Stream => 4,
            MessageType::Reserved8 | MessageType::Reserved9 | MessageType::ReservedA => 2, // 64-bit
            MessageType::ReservedB | MessageType::ReservedC => 3, // 96-bit
            MessageType::ReservedE => 4, // 128-bit
            _ => 1,
        }
    }
}

impl From<u8> for MessageType {
    fn from(val: u8) -> Self {
       Self::from_u8(val)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Ump {
    pub data: [u32; 4],
}

impl Ump {
    pub fn new() -> Self {
        Ump { data: [0; 4] }
    }

    pub fn message_type(&self) -> MessageType {
        let mt = (self.data[0] >> 28) as u8;
        MessageType::from(mt)
    }

    pub fn set_message_type(&mut self, mt: MessageType) {
        self.data[0] &= 0x0FFFFFFF;
        self.data[0] |= (mt as u8 as u32) << 28;
    }

    pub fn group(&self) -> u8 {
        ((self.data[0] >> 24) & 0xF) as u8
    }

    pub fn set_group(&mut self, group: u8) {
        self.data[0] &= 0xF0FFFFFF;
        self.data[0] |= ((group as u32) & 0xF) << 24;
    }

    pub fn status(&self) -> u8 {
        ((self.data[0] >> 16) & 0xF0) as u8
    }

    pub fn channel(&self) -> u8 {
        ((self.data[0] >> 16) & 0x0F) as u8
    }
}

impl Default for Ump {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Ump {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ump({:08X}, {:08X}, {:08X}, {:08X})", self.data[0], self.data[1], self.data[2], self.data[3])
    }
}

impl fmt::Display for Ump {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
