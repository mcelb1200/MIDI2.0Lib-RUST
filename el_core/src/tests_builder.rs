#[cfg(test)]
mod tests {
    use crate::builder::{UtilityBuilder, VoiceBuilder};
    use crate::ump::MessageType;

    #[test]
    fn test_midi1_note_on() {
        let ump = VoiceBuilder::midi1_note_on(5, 2, 60, 100);
        assert_eq!(ump.message_type(), MessageType::Midi1ChannelVoice);
        assert_eq!(ump.group(), 5);
        // Note On Status=0x90, Channel=0x02, Note=0x3C, Vel=0x64
        assert_eq!(ump.data[0], 0x25923C64);
        assert_eq!(ump.data[1], 0);
    }

    #[test]
    fn test_midi2_note_on() {
        let ump = VoiceBuilder::midi2_note_on(1, 0, 60, 0, 0x8000, 0);
        assert_eq!(ump.message_type(), MessageType::Midi2ChannelVoice);
        assert_eq!(ump.group(), 1);
        // MT=0x4, Grp=0x1, Status=0x9, Ch=0x0, Note=0x3C, AttrType=0x00
        assert_eq!(ump.data[0], 0x41903C00);
        assert_eq!(ump.data[1], 0x80000000);
    }

    #[test]
    fn test_midi2_pitch_bend() {
        // Center pitch bend in MIDI 2 is 0x8000_0000
        let ump = VoiceBuilder::midi2_pitch_bend(0, 1, 0x8000_0000);
        assert_eq!(ump.message_type(), MessageType::Midi2ChannelVoice);
        assert_eq!(ump.group(), 0);
        assert_eq!(ump.data[0], 0x40E10000);
        assert_eq!(ump.data[1], 0x80000000);
    }

    #[test]
    fn test_noop() {
        let ump = UtilityBuilder::noop();
        assert_eq!(ump.message_type(), MessageType::Utility);
        assert_eq!(ump.data, [0, 0, 0, 0]);
    }

    #[test]
    fn test_jr_clock() {
        let ump = UtilityBuilder::jitter_reduction_clock(3, 0x1234);
        assert_eq!(ump.message_type(), MessageType::Utility);
        assert_eq!(ump.group(), 3);
        // MT=0x0, Grp=0x3, Status=0x1, Timestamp=0x1234
        assert_eq!(ump.data[0], 0x03101234);

        // Test max timestamp and out-of-bounds group
        let ump_max = UtilityBuilder::jitter_reduction_clock(0xFF, 0xFFFF);
        assert_eq!(ump_max.group(), 0xF); // Masked to 4 bits
        assert_eq!(ump_max.data[0], 0x0F10FFFF);
    }

    #[test]
    fn test_jr_timestamp() {
        let ump = UtilityBuilder::jitter_reduction_timestamp(7, 0xABCD);
        assert_eq!(ump.message_type(), MessageType::Utility);
        assert_eq!(ump.group(), 7);
        // MT=0x0, Grp=0x7, Status=0x2, Timestamp=0xABCD
        assert_eq!(ump.data[0], 0x0720ABCD);

        // Test max timestamp and out-of-bounds group
        let ump_max = UtilityBuilder::jitter_reduction_timestamp(0x12, 0xFFFF);
        assert_eq!(ump_max.group(), 0x2); // Masked to 4 bits (0x12 & 0xF = 0x2)
        assert_eq!(ump_max.data[0], 0x0220FFFF);
    }
}
