use crate::utils;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Utility = 0x0,
    System = 0x1,
    Midi1ChannelVoice = 0x2,
    SysEx7 = 0x3,
    Midi2ChannelVoice = 0x4,
    Data = 0x5,
    FlexData = 0xD,
    Stream = 0xF,
    Unknown,
}

impl From<u8> for MessageType {
    fn from(val: u8) -> Self {
        match val {
            0x0 => MessageType::Utility,
            0x1 => MessageType::System,
            0x2 => MessageType::Midi1ChannelVoice,
            0x3 => MessageType::SysEx7,
            0x4 => MessageType::Midi2ChannelVoice,
            0x5 => MessageType::Data,
            0xD => MessageType::FlexData,
            0xF => MessageType::Stream,
            _ => MessageType::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UmpMessage {
    Ump32(u32),
    Ump64(u32, u32),
    Ump96(u32, u32, u32),
    Ump128(u32, u32, u32, u32),
}

impl UmpMessage {
    pub fn message_type(&self) -> MessageType {
        let first_word = match self {
            UmpMessage::Ump32(w) => w,
            UmpMessage::Ump64(w, _) => w,
            UmpMessage::Ump96(w, _, _) => w,
            UmpMessage::Ump128(w, _, _, _) => w,
        };
        MessageType::from(((first_word >> 28) & 0xF) as u8)
    }

    pub fn group(&self) -> u8 {
        let first_word = match self {
            UmpMessage::Ump32(w) => w,
            UmpMessage::Ump64(w, _) => w,
            UmpMessage::Ump96(w, _, _) => w,
            UmpMessage::Ump128(w, _, _, _) => w,
        };
        ((first_word >> 24) & 0xF) as u8
    }

    pub fn status(&self) -> u8 {
        let first_word = match self {
            UmpMessage::Ump32(w) => w,
            UmpMessage::Ump64(w, _) => w,
            UmpMessage::Ump96(w, _, _) => w,
            UmpMessage::Ump128(w, _, _, _) => w,
        };
        ((first_word >> 16) & 0xF0) as u8
    }

    pub fn channel(&self) -> u8 {
         let first_word = match self {
            UmpMessage::Ump32(w) => w,
            UmpMessage::Ump64(w, _) => w,
            UmpMessage::Ump96(w, _, _) => w,
            UmpMessage::Ump128(w, _, _, _) => w,
        };
        ((first_word >> 16) & 0x0F) as u8
    }
}
