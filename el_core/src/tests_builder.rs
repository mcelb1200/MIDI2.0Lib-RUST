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
    fn test_input_masking() {
        // Pass out-of-bounds values (e.g., 0xFF where only 7 or 4 bits are expected)
        let ump = VoiceBuilder::midi1_note_on(0xFF, 0xFF, 0xFF, 0xFF);
        // group should be masked to 0xF, channel to 0xF, note to 0x7F, velocity to 0x7F
        // group 0xF << 24 = 0x0F000000
        // channel 0xF << 16 = 0x000F0000
        // note 0x7F << 8 = 0x00007F00
        // velocity 0x7F = 0x0000007F
        // Base MT1 note on w1 is 0x20900000
        // Combined: 0x2F9F7F7F
        assert_eq!(ump.data[0], 0x2F9F7F7F);

        let ump2 = VoiceBuilder::midi2_note_on(0xFF, 0xFF, 0xFF, 0xFF, 0xFFFF, 0xFFFF);
        // MT=0x4, group=0xF, status=0x9, channel=0xF, note=0x7F, attr_type=0xFF
        // Combined w1: 0x4F9F7FFF
        assert_eq!(ump2.data[0], 0x4F9F7FFF);
    }
}
