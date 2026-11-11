#[cfg(test)]
mod dump_integration_tests {
    use crate::decoder::{decode_ump, DecoderConfig};
    use el_core::builder::{SystemBuilder, UtilityBuilder, VoiceBuilder};
    use el_core::ump::Ump;

    #[test]
    fn test_decoder_all_message_types() {
        let no_color_cfg = DecoderConfig {
            color: false,
            verbose: true,
        };

        // Utility: JR Clock
        let jr = UtilityBuilder::jitter_reduction_clock(0, 100);
        let out = decode_ump(&jr, &no_color_cfg);
        assert!(out.contains("Utility"));
        assert!(out.contains("JR Clock"));
        assert!(out.contains("3200 µs"));

        // System: Timing Clock
        let clock = SystemBuilder::timing_clock(1);
        let out = decode_ump(&clock, &no_color_cfg);
        assert!(out.contains("System"));
        assert!(out.contains("Grp:  1"));
        assert!(out.contains("Timing Clock"));

        // MIDI 1.0: Note On
        let m1_note = VoiceBuilder::midi1_note_on(2, 3, 60, 90);
        let out = decode_ump(&m1_note, &no_color_cfg);
        assert!(out.contains("Midi1ChannelVoice"));
        assert!(out.contains("Note On: Ch 3, Note 60, Velocity 90"));

        // MIDI 2.0: Note On
        let m2_note = VoiceBuilder::midi2_note_on(0, 5, 72, 0, 0x8000, 0);
        let out = decode_ump(&m2_note, &no_color_cfg);
        assert!(out.contains("Midi2ChannelVoice"));
        assert!(out.contains("MIDI 2.0 Note On: Ch 5, Note 72, Vel16 32768"));

        // MIDI 2.0: CC
        let m2_cc = VoiceBuilder::midi2_cc(0, 1, 11, 0x7FFFFFFF);
        let out = decode_ump(&m2_cc, &no_color_cfg);
        assert!(out.contains("MIDI 2.0 Control Change: Ch 1, CC #11"));

        // SysEx7 Complete (MT=0x3, Status=0x0, Count=4, Payload: 7E 7F 06 01)
        // word 0: 0x3004_7E7F, word 1: 0x0601_0000
        let sysex7 = Ump::new(0x30047E7F, 0x06010000, 0, 0);
        let out = decode_ump(&sysex7, &no_color_cfg);
        assert!(out.contains("SysEx7"));
        assert!(out.contains("Device Identity Inquiry"));

        // MDS Header (MT=0x5, Status 0x8)
        let mds_hdr = Ump::new(0x50810100, 0x00040001, 0x12345678, 0x00010002);
        let out = decode_ump(&mds_hdr, &no_color_cfg);
        assert!(out.contains("MDS Header"));

        // Flex Data Common Tempo (MT=0xD, Bank 0x00, Status 0x00)
        let flex_tempo = Ump::new(0xD0000000, 50_000_000, 0, 0);
        let out = decode_ump(&flex_tempo, &no_color_cfg);
        assert!(out.contains("Flex Data Common Tempo: 120.00 BPM"));

        // Stream: Endpoint Info (MT=0xF, Status 0x001)
        let stream_info = Ump::new(0xF0010101, 0x04000000, 0, 0);
        let out = decode_ump(&stream_info, &no_color_cfg);
        assert!(out.contains("Endpoint Info"));
    }

    #[test]
    fn test_decoder_color_formatting() {
        let color_cfg = DecoderConfig {
            color: true,
            verbose: false,
        };

        let note = VoiceBuilder::midi1_note_on(0, 0, 60, 100);
        let out = decode_ump(&note, &color_cfg);
        // Assert ANSI escape sequences are present
        assert!(out.contains("\x1b["));
        assert!(out.contains("MIDI 1.0 (0x2)"));
    }
}
