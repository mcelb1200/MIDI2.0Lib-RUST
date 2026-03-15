#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::buffer::UmpStreamParser;
    use crate::messages::UmpFactory;
    use crate::ump::MessageType;
    use crate::utils::*;

    extern crate alloc;
    use alloc::vec;

    #[test]
    fn test_noop() {
        let noop = UmpFactory::noop();
        assert_eq!(noop.data, [0, 0, 0, 0]);
        assert_eq!(noop.message_type(), MessageType::Utility);
    }

    #[test]
    fn test_ump_group_getter_setter() {
        // Test getter
        let mut ump = crate::ump::Ump::new();
        // Set bits [24:27] to 0xA (group 10)
        ump.data[0] = 0x0A000000;
        assert_eq!(ump.group(), 10);

        // Test setter
        let mut ump = crate::ump::Ump::new();
        ump.set_group(5);
        assert_eq!(ump.group(), 5);
        assert_eq!(ump.data[0], 0x05000000);

        // Test setter edge case (values > 15 should be masked to 4 bits)
        let mut ump = crate::ump::Ump::new();
        // 255 (0xFF) should be masked to 15 (0xF)
        ump.set_group(255);
        assert_eq!(ump.group(), 15);
        assert_eq!(ump.data[0], 0x0F000000);

        // 16 (0x10) should be masked to 0 (0x0)
        let mut ump = crate::ump::Ump::new();
        ump.set_group(16);
        assert_eq!(ump.group(), 0);
        assert_eq!(ump.data[0], 0x00000000);

        // Ensure setting group doesn't overwrite other bits
        let mut ump = crate::ump::Ump::new();
        ump.data[0] = 0xF0FFFFFF; // Set all other bits
        ump.set_group(3);
        assert_eq!(ump.group(), 3);
        assert_eq!(ump.data[0], 0xF3FFFFFF); // Only bits [24:27] should change
    }

    #[test]
    fn test_message_creation_midi1() {
        let note_on = UmpFactory::midi1_note_on(0, 1, 60, 100);
        let w = note_on.data[0];

        assert_eq!((w >> 28) & 0xF, 0x2); // MT=2
        assert_eq!((w >> 24) & 0xF, 0x0); // Group=0
        assert_eq!((w >> 16) & 0xF0, 0x90); // Status=NoteOn
        assert_eq!((w >> 16) & 0x0F, 0x1); // Channel=1
        assert_eq!((w >> 8) & 0x7F, 60); // Note=60
        assert_eq!(w & 0x7F, 100); // Velocity=100

        assert_eq!(note_on.message_type(), MessageType::Midi1ChannelVoice);
        assert_eq!(note_on.group(), 0);
        assert_eq!(note_on.status(), 0x90);
        assert_eq!(note_on.channel(), 1);
    }

    #[test]
    fn test_message_creation_midi2() {
        let note_on = UmpFactory::midi2_note_on(0, 1, 60, 0, 12345, 0);
        let w1 = note_on.data[0];
        let w2 = note_on.data[1];

        assert_eq!((w1 >> 28) & 0xF, 0x4); // MT=4
        assert_eq!((w1 >> 24) & 0xF, 0x0); // Group=0
        assert_eq!((w1 >> 16) & 0xF0, 0x90); // Status=NoteOn
        assert_eq!((w1 >> 16) & 0x0F, 0x1); // Channel=1
        assert_eq!((w1 >> 8) & 0xFF, 60); // Note=60
        assert_eq!(w2 >> 16, 12345); // Velocity

        assert_eq!(note_on.message_type(), MessageType::Midi2ChannelVoice);
    }

    #[test]
    fn test_midi2_pitch_bend() {
        let group = 2;
        let channel = 5;
        let value: u32 = 0x12345678;
        let pb = UmpFactory::midi2_pitch_bend(group, channel, value);

        assert_eq!(pb.message_type(), MessageType::Midi2ChannelVoice);
        assert_eq!(pb.group(), group);
        assert_eq!(pb.channel(), channel);
        assert_eq!(pb.status(), PITCH_BEND);
        assert_eq!(pb.data[1], value);

        // Explicit check of the first word
        let w1 = pb.data[0];
        assert_eq!((w1 >> 28) & 0xF, 0x4); // MT=4
        assert_eq!((w1 >> 24) & 0xF, u32::from(group));
        assert_eq!((w1 >> 16) & 0xF0, 0xE0); // Status=PitchBend
        assert_eq!((w1 >> 16) & 0x0F, u32::from(channel));
    }

    #[test]
    fn test_midi2_channel_pressure() {
        let group = 3;
        let channel = 7;
        let pressure: u32 = 0x87654321;
        let cp = UmpFactory::midi2_channel_pressure(group, channel, pressure);

        // Validate basic properties
        assert_eq!(cp.message_type(), MessageType::Midi2ChannelVoice);
        assert_eq!(cp.group(), group);
        assert_eq!(cp.channel(), channel);
        assert_eq!(cp.status(), CHANNEL_PRESSURE);
        assert_eq!(cp.data[1], pressure);

        // Explicit check of the first word layout
        let w1 = cp.data[0];
        assert_eq!((w1 >> 28) & 0xF, 0x4); // MT=4
        assert_eq!((w1 >> 24) & 0xF, group as u32);
        assert_eq!((w1 >> 16) & 0xF0, 0xD0); // Status=Channel Pressure
        assert_eq!((w1 >> 16) & 0x0F, channel as u32);

        // Edge case: Test out-of-bounds group and channel
        // Values should be masked to 4 bits (e.g. 17 & 0xF = 1)
        let oob_group = 17;   // 10001 in binary -> masks to 0001 (1)
        let oob_channel = 18; // 10010 in binary -> masks to 0010 (2)
        let max_pressure = u32::MAX;
        let cp_edge = UmpFactory::midi2_channel_pressure(oob_group, oob_channel, max_pressure);

        assert_eq!(cp_edge.group(), 1);
        assert_eq!(cp_edge.channel(), 2);
        assert_eq!(cp_edge.data[1], max_pressure);
    }

    #[test]
    fn test_stream_parser() {
        let data = vec![
            0x20903C64, // MIDI 1.0 Note On
            0x40903C00, 0x12340000, // MIDI 2.0 Note On
        ];

        let mut parser = UmpStreamParser::new(data.into_iter());

        let msg1 = parser.next().unwrap();
        assert_eq!(msg1.message_type(), MessageType::Midi1ChannelVoice);
        assert_eq!(msg1.data[0], 0x20903C64);

        let msg2 = parser.next().unwrap();
        assert_eq!(msg2.message_type(), MessageType::Midi2ChannelVoice);
        assert_eq!(msg2.data[0], 0x40903C00);
        assert_eq!(msg2.data[1], 0x12340000);

        assert!(parser.next().is_none());
    }

    #[test]
    fn test_scaling() {
        // Scale 127 (7-bit) to 16-bit
        let val = scale_up(127, 7, 16);
        assert_eq!(val, 0xFFFF);

        // Scale 64 (7-bit) to 16-bit
        // 0x40 -> 0x8000
        let val = scale_up(64, 7, 16);
        // exact center behavior might differ slightly depending on implementation but let's check basic
        assert!(val > 0x7F00 && val < 0x8100);

        // Scale down
        let val = scale_down(0xFFFF, 16, 7);
        assert_eq!(val, 127);
    }

    #[test]
    fn test_scaling_edge_cases() {
        // Test scale_up invalid inputs (should return 0, not panic)
        assert_eq!(scale_up(100, 0, 32), 0); // src_bits = 0
        assert_eq!(scale_up(100, 33, 32), 0); // src_bits > 32
        assert_eq!(scale_up(100, 32, 33), 0); // dst_bits > 32

        // Test scale_down invalid inputs
        assert_eq!(scale_down(100, 33, 32), 0); // src_bits > 32
        assert_eq!(scale_down(100, 32, 33), 0); // dst_bits > 32

        // Test shift boundary
        // scale_down where src_bits - dst_bits == 32
        assert_eq!(scale_down(100, 32, 0), 0);
    }

    #[test]
    fn test_scale_up_panic() {
        // This should not panic
        let res = scale_up(1, 1, 32);
        assert_eq!(res, 0xFFFFFFFF);
    }

    #[test]
    fn test_message_type_from_u8_valid() {
        assert_eq!(MessageType::from_u8(0x0), MessageType::Utility);
        assert_eq!(MessageType::from_u8(0x1), MessageType::System);
        assert_eq!(MessageType::from_u8(0x2), MessageType::Midi1ChannelVoice);
        assert_eq!(MessageType::from_u8(0x3), MessageType::SysEx7);
        assert_eq!(MessageType::from_u8(0x4), MessageType::Midi2ChannelVoice);
        assert_eq!(MessageType::from_u8(0x5), MessageType::Data);
        assert_eq!(MessageType::from_u8(0x6), MessageType::Reserved6);
        assert_eq!(MessageType::from_u8(0x7), MessageType::Reserved7);
        assert_eq!(MessageType::from_u8(0x8), MessageType::Reserved8);
        assert_eq!(MessageType::from_u8(0x9), MessageType::Reserved9);
        assert_eq!(MessageType::from_u8(0xA), MessageType::ReservedA);
        assert_eq!(MessageType::from_u8(0xB), MessageType::ReservedB);
        assert_eq!(MessageType::from_u8(0xC), MessageType::ReservedC);
        assert_eq!(MessageType::from_u8(0xD), MessageType::FlexData);
        assert_eq!(MessageType::from_u8(0xE), MessageType::ReservedE);
        assert_eq!(MessageType::from_u8(0xF), MessageType::Stream);
    }

    #[test]
    fn test_message_type_from_u8_out_of_bounds() {
        // Because `from_u8` uses `val & 0xF`, values outside the 0-15 range
        // should wrap around and still return valid message types instead of panicking.

        // 16 (0x10) -> 0x10 & 0xF = 0x0
        assert_eq!(MessageType::from_u8(16), MessageType::Utility);

        // 31 (0x1F) -> 0x1F & 0xF = 0xF
        assert_eq!(MessageType::from_u8(31), MessageType::Stream);

        // 100 (0x64) -> 0x64 & 0xF = 0x4
        assert_eq!(MessageType::from_u8(100), MessageType::Midi2ChannelVoice);

        // 255 (0xFF) -> 0xFF & 0xF = 0xF
        assert_eq!(MessageType::from_u8(255), MessageType::Stream);
    }
}
