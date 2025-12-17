use crate::ump::{UmpMessage, MessageType};

pub struct UmpStreamParser<I>
where
    I: Iterator<Item = u32>,
{
    iter: I,
}

impl<I> UmpStreamParser<I>
where
    I: Iterator<Item = u32>,
{
    pub fn new(iter: I) -> Self {
        Self { iter }
    }
}

impl<I> Iterator for UmpStreamParser<I>
where
    I: Iterator<Item = u32>,
{
    type Item = UmpMessage;

    fn next(&mut self) -> Option<Self::Item> {
        let w1 = self.iter.next()?;
        let message_type = ((w1 >> 28) & 0xF) as u8;
        let mt = MessageType::from(message_type);

        match mt {
            MessageType::Utility | MessageType::System | MessageType::Midi1ChannelVoice => {
                Some(UmpMessage::Ump32(w1))
            }
            MessageType::Midi2ChannelVoice | MessageType::SysEx7 => {
                // SysEx7 (MT=0x3) is 64-bit. Wait, C++ says MT3 is Sysex7 and it returns array<uint32_t, 2>
                // So it consumes 2 words.
                let w2 = self.iter.next().unwrap_or(0); // Should probably handle partial streams better
                Some(UmpMessage::Ump64(w1, w2))
            }
            MessageType::Data | MessageType::FlexData => {
                // MT=0x5 (Data) and MT=0xD (Flex Data) are 128-bit
                let w2 = self.iter.next().unwrap_or(0);
                let w3 = self.iter.next().unwrap_or(0);
                let w4 = self.iter.next().unwrap_or(0);
                Some(UmpMessage::Ump128(w1, w2, w3, w4))
            }
            MessageType::Stream => {
                 // MT=0xF (Stream) is 128-bit
                let w2 = self.iter.next().unwrap_or(0);
                let w3 = self.iter.next().unwrap_or(0);
                let w4 = self.iter.next().unwrap_or(0);
                Some(UmpMessage::Ump128(w1, w2, w3, w4))
            }
            _ => {
                // Unknown type, assuming 32-bit to consume progress but this is risky
                Some(UmpMessage::Ump32(w1))
            }
        }
    }
}
