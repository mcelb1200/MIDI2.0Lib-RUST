#[cfg(test)]
mod tests {
    use crate::parser::UmpStreamParser;
    use crate::ump::MessageType;

    #[test]
    fn test_parser_single_word() {
        let data = [0x20903C64];
        let mut parser = UmpStreamParser::new(&data);

        if let Some(ump) = parser.next() {
            assert_eq!(ump.message_type(), MessageType::Midi1ChannelVoice);
            assert_eq!(ump.data[0], 0x20903C64);
            assert_eq!(ump.data[1], 0);
        } else {
            panic!("Expected UMP"); // Panic is allowed in tests, just avoiding expect/unwrap in lib
        }

        assert!(parser.next().is_none());
    }

    #[test]
    fn test_parser_multi_word() {
        // MT=0x4 is 2 words (MIDI 2.0 Voice)
        let data = [0x40903C00, 0x80000000];
        let mut parser = UmpStreamParser::new(&data);

        if let Some(ump) = parser.next() {
            assert_eq!(ump.message_type(), MessageType::Midi2ChannelVoice);
            assert_eq!(ump.data[0], 0x40903C00);
            assert_eq!(ump.data[1], 0x80000000);
        } else {
            panic!("Expected UMP");
        }

        assert!(parser.next().is_none());
    }

    #[test]
    fn test_parser_truncation() {
        // Provide only 1 word of a 2-word message (MT=0x4)
        let data = [0x40903C00];
        let mut parser = UmpStreamParser::new(&data);

        // It must return None to prevent parsing corrupted partial packets
        assert!(parser.next().is_none());
    }
}
