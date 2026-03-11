#[cfg(test)]
mod tests {
    use crate::ump::{MessageType, Ump};

    #[test]
    fn test_ump_default() {
        let ump = Ump::default();
        assert_eq!(ump.data, [0, 0, 0, 0]);
        assert_eq!(ump.message_type(), MessageType::Utility);
        assert_eq!(ump.word_count(), 1);
        assert_eq!(ump.group(), 0);
    }

    #[test]
    fn test_ump_new() {
        // Group 5, MT=4 (MIDI 2.0 Voice)
        let ump = Ump::new(0x45000000, 0x12345678, 0, 0);
        assert_eq!(ump.data[0], 0x45000000);
        assert_eq!(ump.message_type(), MessageType::Midi2ChannelVoice);
        assert_eq!(ump.group(), 5);
        assert_eq!(ump.word_count(), 2);
    }

    #[test]
    fn test_word_counts() {
        assert_eq!(MessageType::Utility.word_count(), 1);
        assert_eq!(MessageType::System.word_count(), 1);
        assert_eq!(MessageType::Data64.word_count(), 2);
        assert_eq!(MessageType::Data128.word_count(), 4);
        assert_eq!(MessageType::UmpStream.word_count(), 4);
    }
}
