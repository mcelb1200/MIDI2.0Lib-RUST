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
    fn test_midi2_channel_pressure() {
        let group = 3;
        let channel = 7;
        let pressure: u32 = 0x87654321;
        let cp = VoiceBuilder::midi2_channel_pressure(group, channel, pressure);

        // Validate basic properties
        assert_eq!(cp.message_type(), MessageType::Midi2ChannelVoice);
        assert_eq!(cp.group(), group);
        assert_eq!(cp.channel(), channel);
        assert_eq!(cp.status(), crate::utils::CHANNEL_PRESSURE);
        assert_eq!(cp.data[1], pressure);

        // Explicit check of the first word layout
        let w1 = cp.data[0];
        assert_eq!((w1 >> 28) & 0xF, 0x4); // MT=4
        assert_eq!((w1 >> 24) & 0xF, group as u32);
        assert_eq!((w1 >> 16) & 0xF0, 0xD0); // Status=Channel Pressure
        assert_eq!((w1 >> 16) & 0x0F, channel as u32);

        // Edge case: Test out-of-bounds group and channel
        // Values should be masked to 4 bits (e.g. 17 & 0xF = 1)
        let oob_group = 17; // 10001 in binary -> masks to 0001 (1)
        let oob_channel = 18; // 10010 in binary -> masks to 0010 (2)
        let max_pressure = u32::MAX;
        let cp_edge = VoiceBuilder::midi2_channel_pressure(oob_group, oob_channel, max_pressure);

        assert_eq!(cp_edge.group(), 1);
        assert_eq!(cp_edge.channel(), 2);
        assert_eq!(cp_edge.data[1], max_pressure);
    }

    #[test]
    fn test_noop() {
        let ump = UtilityBuilder::noop();
        assert_eq!(ump.message_type(), MessageType::Utility);
        assert_eq!(ump.data, [0, 0, 0, 0]);
    }
}
