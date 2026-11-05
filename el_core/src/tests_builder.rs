#[cfg(test)]
mod tests {
    use crate::builder::{FlexBuilder, UtilityBuilder, VoiceBuilder};
    use crate::ump::MessageType;
    use crate::utils::{FLEXDATA_COMMON, FLEXDATA_COMMON_TEMPO, FLEXDATA_LYRIC};

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
    fn test_midi1_pitch_bend() {
        // Center pitch bend in MIDI 1 is 8192 (MSB=0x40, LSB=0x00)
        let ump = VoiceBuilder::midi1_pitch_bend(0, 0, 8192);
        assert_eq!(ump.message_type(), MessageType::Midi1ChannelVoice);
        assert_eq!(ump.group(), 0);
        assert_eq!(ump.data[0], 0x20E00040);

        // Min
        let ump2 = VoiceBuilder::midi1_pitch_bend(1, 2, 0);
        assert_eq!(ump2.data[0], 0x21E20000);

        // Max (14-bit)
        let ump3 = VoiceBuilder::midi1_pitch_bend(15, 15, 16383);
        assert_eq!(ump3.data[0], 0x2FEF7F7F);
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

        let ump3 = VoiceBuilder::midi1_pitch_bend(0xFF, 0xFF, 0xFFFF);
        // group -> 0xF, channel -> 0xF, value -> split to 0x7F, 0x7F
        // MT=0x2, Status=0xE, w1 = 0x2FEF7F7F
        assert_eq!(ump3.data[0], 0x2FEF7F7F);
    }

    #[test]
    fn test_flex_builder() {
        // Header
        let header = FlexBuilder::header(1, 0, 1, 2, FLEXDATA_COMMON, FLEXDATA_COMMON_TEMPO);
        assert_eq!(header, 0xD1120000);

        // Tempo
        let tempo = FlexBuilder::tempo(1, 2, 0, 1, 0x12345678);
        assert_eq!(tempo.message_type(), MessageType::ReservedD);
        assert_eq!(tempo.group(), 1);
        assert_eq!(tempo.data[0], 0xD1120000);
        assert_eq!(tempo.data[1], 0x12345678);
        assert_eq!(tempo.data[2], 0);
        assert_eq!(tempo.data[3], 0);

        // Time Signature
        let timesig = FlexBuilder::time_sig(1, 2, 0, 1, 4, 4, 8);
        assert_eq!(timesig.message_type(), MessageType::ReservedD);
        assert_eq!(timesig.data[0], 0xD1120001);
        assert_eq!(timesig.data[1], 0x04040800);
        assert_eq!(timesig.data[2], 0);
        assert_eq!(timesig.data[3], 0);

        // Metronome
        let metronome = FlexBuilder::metronome(1, 2, 0, 1, 10, 11, 12, 13, 14, 15);
        assert_eq!(metronome.message_type(), MessageType::ReservedD);
        assert_eq!(metronome.data[0], 0xD1120002);
        assert_eq!(metronome.data[1], 0x0A0B0C0D);
        assert_eq!(metronome.data[2], 0x0E0F0000);
        assert_eq!(metronome.data[3], 0);

        // Key Signature
        let keysig = FlexBuilder::key_sig(1, 2, 0, 1, 5, 2);
        assert_eq!(keysig.message_type(), MessageType::ReservedD);
        assert_eq!(keysig.data[0], 0xD1120005);
        assert_eq!(keysig.data[1], 0x05020000);
        assert_eq!(keysig.data[2], 0);
        assert_eq!(keysig.data[3], 0);

        // Chord
        let chord = FlexBuilder::chord(
            1, 2, 0, 1, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2,
        );
        assert_eq!(chord.message_type(), MessageType::ReservedD);
        assert_eq!(chord.data[0], 0xD1120006);
        assert_eq!(chord.data[1], 0x12034567);
        assert_eq!(chord.data[2], 0x89AB0000);
        assert_eq!(chord.data[3], 0xCD0EF012);

        // Text
        let text = b"Hello";
        let text_ump = FlexBuilder::text(1, 2, 0, 1, FLEXDATA_LYRIC, 0, text);
        assert_eq!(text_ump.message_type(), MessageType::ReservedD);
        assert_eq!(text_ump.data[0], 0xD1120200);
        assert_eq!(text_ump.data[1], 0x48656C6C);
        assert_eq!(text_ump.data[2], 0x6F000000);
        assert_eq!(text_ump.data[3], 0);

        // Lyric text helper
        let lyric_ump = FlexBuilder::lyric(1, 2, 0, 1, 0, text);
        assert_eq!(lyric_ump.data, text_ump.data);

        // Performance text helper
        let perf_ump = FlexBuilder::performance(1, 2, 0, 1, 0, text);
        assert_eq!(perf_ump.data[0], 0xD1120100);
        assert_eq!(perf_ump.data[1], 0x48656C6C);
        assert_eq!(perf_ump.data[2], 0x6F000000);
        assert_eq!(perf_ump.data[3], 0);
    }
}
