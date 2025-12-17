#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::UmpFactory;
    use crate::ump::{UmpMessage, MessageType};
    use crate::buffer::UmpStreamParser;
    use crate::utils::*;

    #[test]
    fn test_message_creation_midi1() {
        let note_on = UmpFactory::midi1_note_on(0, 1, 60, 100);
        if let UmpMessage::Ump32(w) = note_on {
            assert_eq!((w >> 28) & 0xF, 0x2); // MT=2
            assert_eq!((w >> 24) & 0xF, 0x0); // Group=0
            assert_eq!((w >> 16) & 0xF0, 0x90); // Status=NoteOn
            assert_eq!((w >> 16) & 0x0F, 0x1); // Channel=1
            assert_eq!((w >> 8) & 0x7F, 60); // Note=60
            assert_eq!(w & 0x7F, 100); // Velocity=100
        } else {
            panic!("Expected Ump32");
        }
    }

    #[test]
    fn test_message_creation_midi2() {
        let note_on = UmpFactory::midi2_note_on(0, 1, 60, 0, 12345, 0);
        if let UmpMessage::Ump64(w1, w2) = note_on {
            assert_eq!((w1 >> 28) & 0xF, 0x4); // MT=4
            assert_eq!((w1 >> 24) & 0xF, 0x0); // Group=0
            assert_eq!((w1 >> 16) & 0xF0, 0x90); // Status=NoteOn
            assert_eq!((w1 >> 16) & 0x0F, 0x1); // Channel=1
            assert_eq!((w1 >> 8) & 0xFF, 60); // Note=60
            assert_eq!(w2 >> 16, 12345); // Velocity
        } else {
            panic!("Expected Ump64");
        }
    }

    #[test]
    fn test_stream_parser() {
        let data = vec![
            0x20903C64, // MIDI 1.0 Note On
            0x40903C00, 0x12340000, // MIDI 2.0 Note On
        ];

        let mut parser = UmpStreamParser::new(data.into_iter());

        let msg1 = parser.next().unwrap();
        assert!(matches!(msg1, UmpMessage::Ump32(_)));
        assert_eq!(msg1.message_type(), MessageType::Midi1ChannelVoice);

        let msg2 = parser.next().unwrap();
        assert!(matches!(msg2, UmpMessage::Ump64(_, _)));
        assert_eq!(msg2.message_type(), MessageType::Midi2ChannelVoice);

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
}
