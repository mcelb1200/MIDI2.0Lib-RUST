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

    #[test]
    fn test_ump_group_getter_setter() {
        // Test getter
        let mut ump = Ump::default();
        // Set bits [24:27] to 0xA (group 10)
        ump.data[0] = 0x0A000000;
        assert_eq!(ump.group(), 10);

        // Test setter
        let mut ump = Ump::default();
        ump.set_group(5);
        assert_eq!(ump.group(), 5);
        assert_eq!(ump.data[0], 0x05000000);

        // Test setter edge case (values > 15 should be masked to 4 bits)
        let mut ump = Ump::default();
        // 255 (0xFF) should be masked to 15 (0xF)
        ump.set_group(255);
        assert_eq!(ump.group(), 15);
        assert_eq!(ump.data[0], 0x0F000000);

        // 16 (0x10) should be masked to 0 (0x0)
        let mut ump = Ump::default();
        ump.set_group(16);
        assert_eq!(ump.group(), 0);
        assert_eq!(ump.data[0], 0x00000000);

        // Ensure setting group doesn't overwrite other bits
        let mut ump = Ump::default();
        ump.data[0] = 0xF0FFFFFF; // Set all other bits
        ump.set_group(3);
        assert_eq!(ump.group(), 3);
        assert_eq!(ump.data[0], 0xF3FFFFFF); // Only bits [24:27] should change
    }
}
