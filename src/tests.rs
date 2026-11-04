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
    fn test_midi1_program_change() {
        let group = 3;
        let channel = 10;
        let program = 42;
        let pc = UmpFactory::midi1_program_change(group, channel, program);

        assert_eq!(pc.message_type(), MessageType::Midi1ChannelVoice);
        assert_eq!(pc.group(), group);
        assert_eq!(pc.channel(), channel);
        assert_eq!(pc.status(), PROGRAM_CHANGE);

        let w1 = pc.data[0];
        assert_eq!((w1 >> 28) & 0xF, 0x2); // MT=2
        assert_eq!((w1 >> 8) & 0x7F, program as u32);
    }

    #[test]
    fn test_midi2_program_change() {
        let group = 1;
        let channel = 2;
        let program = 64;
        let bank_msb = 0x12;
        let bank_lsb = 0x34;

        // Test with bank valid
        let pc =
            UmpFactory::midi2_program_change(group, channel, program, true, bank_msb, bank_lsb);
        assert_eq!(pc.message_type(), MessageType::Midi2ChannelVoice);
        assert_eq!(pc.group(), group);
        assert_eq!(pc.channel(), channel);
        assert_eq!(pc.status(), PROGRAM_CHANGE);

        let w1 = pc.data[0];
        let w2 = pc.data[1];

        assert_eq!((w1 >> 28) & 0xF, 0x4); // MT=4
        assert_eq!((w1 >> 16) & 0xF0, 0xC0); // Status=0xC0
        assert_eq!(w1 & 0x1, 1); // Bank Valid bit

        assert_eq!((w2 >> 24) & 0x7F, program as u32);
        assert_eq!((w2 >> 8) & 0x7F, bank_msb as u32);
        assert_eq!(w2 & 0x7F, bank_lsb as u32);

        // Test without bank valid
        let pc_no_bank =
            UmpFactory::midi2_program_change(group, channel, program, false, bank_msb, bank_lsb);
        let w1_no = pc_no_bank.data[0];
        let w2_no = pc_no_bank.data[1];

        assert_eq!(w1_no & 0x1, 0); // Bank Valid bit should be 0
        assert_eq!((w2_no >> 24) & 0x7F, program as u32);
        assert_eq!(w2_no & 0xFFFFFF, 0); // Bank and index should be 0

        // Test masking
        let pc_masked = UmpFactory::midi2_program_change(0, 0, 0xFF, true, 0xFF, 0xFF);
        let w2_masked = pc_masked.data[1];
        assert_eq!((w2_masked >> 24) & 0x7F, 0x7F);
        assert_eq!((w2_masked >> 8) & 0x7F, 0x7F);
        assert_eq!(w2_masked & 0x7F, 0x7F);
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
        let oob_group = 17; // 10001 in binary -> masks to 0001 (1)
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

    #[test]
    fn test_mtc_quarter_frame() {
        let group = 5;
        let data = 0x42;
        let mtc = UmpFactory::mtc_quarter_frame(group, data);

        assert_eq!(mtc.message_type(), MessageType::System);
        assert_eq!(mtc.group(), group);
        assert_eq!(mtc.status(), 0xF0); // Ump::status() returns high nibble

        let w = mtc.data[0];
        assert_eq!((w >> 28) & 0xF, 0x1); // MT=1
        assert_eq!((w >> 24) & 0xF, group as u32);
        assert_eq!((w >> 16) & 0xFF, TIMING_CODE as u32);
        assert_eq!((w >> 8) & 0x7F, data as u32);
        assert_eq!(w & 0x7F, 0);

        // Test masking
        let mtc_masked = UmpFactory::mtc_quarter_frame(0xFF, 0xFF);
        assert_eq!(mtc_masked.group(), 0xF);
        let w_masked = mtc_masked.data[0];
        // Ensure data byte is masked to 7 bits (0x7F)
        assert_eq!((w_masked >> 8) & 0xFF, 0x7F);
    }

    #[test]
    fn test_sysex7_empty_payload() {
        use crate::sysex::{SysEx7Chunker, SysEx7Format, SysEx7Receiver, SysEx7ReceiverState};

        let chunker = SysEx7Chunker::new(2, &[]);
        let packets: alloc::vec::Vec<_> = chunker.collect();
        assert_eq!(packets.len(), 1);

        let ump = packets[0];
        assert_eq!(ump.message_type(), MessageType::SysEx7);
        assert_eq!(ump.group(), 2);
        let format = (ump.data[0] >> 20) & 0xF;
        assert_eq!(format, SysEx7Format::Complete as u32);
        let num_bytes = (ump.data[0] >> 16) & 0xF;
        assert_eq!(num_bytes, 0);

        let mut receiver = SysEx7Receiver::<64>::new();
        let res = receiver.process_ump(&ump);
        assert_eq!(res, Ok(SysEx7ReceiverState::Complete));
        assert_eq!(receiver.payload(), &[]);
        assert_eq!(receiver.group(), 2);
    }

    #[test]
    fn test_sysex7_single_packet_complete() {
        use crate::sysex::{SysEx7Chunker, SysEx7Format, SysEx7Receiver, SysEx7ReceiverState};

        let payload = [0x7E, 0x7F, 0x09, 0x01];
        let mut chunker = SysEx7Chunker::new(0, &payload);
        let ump = chunker.next().expect("Expected 1 packet");
        assert!(chunker.next().is_none());

        assert_eq!(ump.message_type(), MessageType::SysEx7);
        assert_eq!(ump.group(), 0);
        let format = (ump.data[0] >> 20) & 0xF;
        assert_eq!(format, SysEx7Format::Complete as u32);
        let num_bytes = (ump.data[0] >> 16) & 0xF;
        assert_eq!(num_bytes, 4);

        let mut receiver = SysEx7Receiver::<128>::new();
        assert_eq!(
            receiver.process_ump(&ump),
            Ok(SysEx7ReceiverState::Complete)
        );
        assert_eq!(receiver.payload(), &payload);
        assert_eq!(receiver.group(), 0);
    }

    #[test]
    fn test_sysex7_multi_packet_roundtrip() {
        use crate::sysex::{SysEx7Chunker, SysEx7Format, SysEx7Receiver, SysEx7ReceiverState};

        // 15 bytes requires 3 packets: 6 + 6 + 3
        let payload: [u8; 15] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F,
        ];

        let chunker = SysEx7Chunker::new(3, &payload);
        let packets: alloc::vec::Vec<_> = chunker.collect();
        assert_eq!(packets.len(), 3);

        // Check format flags
        assert_eq!((packets[0].data[0] >> 20) & 0xF, SysEx7Format::Start as u32);
        assert_eq!((packets[0].data[0] >> 16) & 0xF, 6);

        assert_eq!(
            (packets[1].data[0] >> 20) & 0xF,
            SysEx7Format::Continue as u32
        );
        assert_eq!((packets[1].data[0] >> 16) & 0xF, 6);

        assert_eq!((packets[2].data[0] >> 20) & 0xF, SysEx7Format::End as u32);
        assert_eq!((packets[2].data[0] >> 16) & 0xF, 3);

        // Reassemble with receiver
        let mut receiver = SysEx7Receiver::<64>::new();
        assert_eq!(receiver.state(), SysEx7ReceiverState::Idle);

        assert_eq!(
            receiver.process_ump(&packets[0]),
            Ok(SysEx7ReceiverState::Receiving)
        );
        assert_eq!(
            receiver.process_ump(&packets[1]),
            Ok(SysEx7ReceiverState::Receiving)
        );
        assert_eq!(
            receiver.process_ump(&packets[2]),
            Ok(SysEx7ReceiverState::Complete)
        );

        assert_eq!(receiver.payload(), &payload);
        assert_eq!(receiver.group(), 3);
    }

    #[test]
    fn test_sysex7_receiver_overflow_and_errors() {
        use crate::sysex::{
            build_sysex7_ump, SysEx7Format, SysEx7Receiver, SysEx7ReceiverError,
            SysEx7ReceiverState,
        };

        let mut receiver = SysEx7Receiver::<10>::new();
        let payload = [1, 2, 3, 4, 5, 6];
        let ump1 = build_sysex7_ump(1, SysEx7Format::Start, &payload);
        let ump2 = build_sysex7_ump(1, SysEx7Format::Continue, &payload); // total 12 > 10

        assert_eq!(
            receiver.process_ump(&ump1),
            Ok(SysEx7ReceiverState::Receiving)
        );
        assert_eq!(
            receiver.process_ump(&ump2),
            Err(SysEx7ReceiverError::BufferOverflow)
        );
        assert_eq!(receiver.state(), SysEx7ReceiverState::Idle);

        // Out-of-order error: Continue without Start
        assert_eq!(
            receiver.process_ump(&ump2),
            Err(SysEx7ReceiverError::UnexpectedFormat)
        );

        // Group mismatch error
        let mut receiver2 = SysEx7Receiver::<32>::new();
        assert_eq!(
            receiver2.process_ump(&ump1),
            Ok(SysEx7ReceiverState::Receiving)
        );
        let ump_diff_group = build_sysex7_ump(2, SysEx7Format::End, &[1, 2]);
        assert_eq!(
            receiver2.process_ump(&ump_diff_group),
            Err(SysEx7ReceiverError::GroupMismatch)
        );
    }

    #[test]
    fn test_sysex8_empty_payload() {
        use crate::sysex::{SysEx8Chunker, SysEx8Format, SysEx8Receiver, SysEx8ReceiverState};

        let chunker = SysEx8Chunker::new(4, 0x1A, &[]);
        let packets: alloc::vec::Vec<_> = chunker.collect();
        assert_eq!(packets.len(), 1);

        let ump = packets[0];
        assert_eq!(ump.message_type(), MessageType::Data);
        assert_eq!(ump.group(), 4);
        let format = (ump.data[0] >> 20) & 0xF;
        assert_eq!(format, SysEx8Format::Complete as u32);
        let num_bytes = (ump.data[0] >> 16) & 0xF;
        assert_eq!(num_bytes, 1); // stream_id only
        let stream_id = (ump.data[0] >> 8) & 0xFF;
        assert_eq!(stream_id, 0x1A);

        let mut receiver = SysEx8Receiver::<64>::new();
        let res = receiver.process_ump(&ump);
        assert_eq!(res, Ok(SysEx8ReceiverState::Complete));
        assert_eq!(receiver.payload(), &[]);
        assert_eq!(receiver.group(), 4);
        assert_eq!(receiver.stream_id(), 0x1A);
    }

    #[test]
    fn test_sysex8_single_packet_complete() {
        use crate::sysex::{SysEx8Chunker, SysEx8Format, SysEx8Receiver, SysEx8ReceiverState};

        // 13 8-bit bytes (exact single packet max payload)
        let payload: [u8; 13] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
        ];
        let mut chunker = SysEx8Chunker::new(1, 0xAA, &payload);
        let ump = chunker.next().expect("Expected 1 packet");
        assert!(chunker.next().is_none());

        assert_eq!(ump.message_type(), MessageType::Data);
        assert_eq!(ump.group(), 1);
        let format = (ump.data[0] >> 20) & 0xF;
        assert_eq!(format, SysEx8Format::Complete as u32);
        let num_bytes = (ump.data[0] >> 16) & 0xF;
        assert_eq!(num_bytes, 14); // 13 payload + 1 stream_id
        let stream_id = (ump.data[0] >> 8) & 0xFF;
        assert_eq!(stream_id, 0xAA);

        // Verify word layout matches C++ mt5Sysex8:
        // w0 = 0x510EAA01
        // w1 = 0x02030405
        // w2 = 0x06070809
        // w3 = 0x0A0B0C0D
        assert_eq!(ump.data[0], 0x510EAA01);
        assert_eq!(ump.data[1], 0x02030405);
        assert_eq!(ump.data[2], 0x06070809);
        assert_eq!(ump.data[3], 0x0A0B0C0D);

        let mut receiver = SysEx8Receiver::<128>::new();
        assert_eq!(
            receiver.process_ump(&ump),
            Ok(SysEx8ReceiverState::Complete)
        );
        assert_eq!(receiver.payload(), &payload);
        assert_eq!(receiver.group(), 1);
        assert_eq!(receiver.stream_id(), 0xAA);
    }

    #[test]
    fn test_sysex8_multi_packet_roundtrip() {
        use crate::sysex::{SysEx8Chunker, SysEx8Format, SysEx8Receiver, SysEx8ReceiverState};

        // 30 bytes requires 3 packets: 13 + 13 + 4
        let payload: [u8; 30] = [
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x20,
            0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x30, 0x31,
            0x32, 0x33,
        ];

        let chunker = SysEx8Chunker::new(5, 0x42, &payload);
        let packets: alloc::vec::Vec<_> = chunker.collect();
        assert_eq!(packets.len(), 3);

        // Packet 0: Start, 13 bytes payload (total 14)
        assert_eq!((packets[0].data[0] >> 20) & 0xF, SysEx8Format::Start as u32);
        assert_eq!((packets[0].data[0] >> 16) & 0xF, 14);
        assert_eq!((packets[0].data[0] >> 8) & 0xFF, 0x42);

        // Packet 1: Continue, 13 bytes payload (total 14)
        assert_eq!(
            (packets[1].data[0] >> 20) & 0xF,
            SysEx8Format::Continue as u32
        );
        assert_eq!((packets[1].data[0] >> 16) & 0xF, 14);
        assert_eq!((packets[1].data[0] >> 8) & 0xFF, 0x42);

        // Packet 2: End, 4 bytes payload (total 5)
        assert_eq!((packets[2].data[0] >> 20) & 0xF, SysEx8Format::End as u32);
        assert_eq!((packets[2].data[0] >> 16) & 0xF, 5);
        assert_eq!((packets[2].data[0] >> 8) & 0xFF, 0x42);

        // Reassemble with receiver
        let mut receiver = SysEx8Receiver::<64>::new();
        assert_eq!(receiver.state(), SysEx8ReceiverState::Idle);

        assert_eq!(
            receiver.process_ump(&packets[0]),
            Ok(SysEx8ReceiverState::Receiving)
        );
        assert_eq!(
            receiver.process_ump(&packets[1]),
            Ok(SysEx8ReceiverState::Receiving)
        );
        assert_eq!(
            receiver.process_ump(&packets[2]),
            Ok(SysEx8ReceiverState::Complete)
        );

        assert_eq!(receiver.payload(), &payload);
        assert_eq!(receiver.group(), 5);
        assert_eq!(receiver.stream_id(), 0x42);
    }

    #[test]
    fn test_sysex8_receiver_overflow_and_errors() {
        use crate::sysex::{
            build_sysex8_ump, SysEx8Format, SysEx8Receiver, SysEx8ReceiverError,
            SysEx8ReceiverState,
        };

        let mut receiver = SysEx8Receiver::<20>::new();
        let payload = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13];
        let ump1 = build_sysex8_ump(1, 0x10, SysEx8Format::Start, &payload);
        let ump2 = build_sysex8_ump(1, 0x10, SysEx8Format::Continue, &payload); // total 26 > 20

        assert_eq!(
            receiver.process_ump(&ump1),
            Ok(SysEx8ReceiverState::Receiving)
        );
        assert_eq!(
            receiver.process_ump(&ump2),
            Err(SysEx8ReceiverError::BufferOverflow)
        );
        assert_eq!(receiver.state(), SysEx8ReceiverState::Idle);

        // Out-of-order error: Continue without Start
        assert_eq!(
            receiver.process_ump(&ump2),
            Err(SysEx8ReceiverError::UnexpectedFormat)
        );

        // Group mismatch error
        let mut receiver2 = SysEx8Receiver::<64>::new();
        assert_eq!(
            receiver2.process_ump(&ump1),
            Ok(SysEx8ReceiverState::Receiving)
        );
        let ump_diff_group = build_sysex8_ump(2, 0x10, SysEx8Format::End, &[1, 2]);
        assert_eq!(
            receiver2.process_ump(&ump_diff_group),
            Err(SysEx8ReceiverError::GroupMismatch)
        );

        // Stream ID mismatch error
        let mut receiver3 = SysEx8Receiver::<64>::new();
        assert_eq!(
            receiver3.process_ump(&ump1),
            Ok(SysEx8ReceiverState::Receiving)
        );
        let ump_diff_stream = build_sysex8_ump(1, 0x99, SysEx8Format::End, &[1, 2]);
        assert_eq!(
            receiver3.process_ump(&ump_diff_stream),
            Err(SysEx8ReceiverError::StreamIdMismatch)
        );

        // Non-SysEx8 packet error (e.g. SysEx7 MT=0x3)
        let ump_not_sysex8 = crate::ump::Ump::new();
        assert_eq!(
            receiver3.process_ump(&ump_not_sysex8),
            Err(SysEx8ReceiverError::NotSysEx8Packet)
        );
    }

    #[test]
    fn test_stream_message_builders() {
        use crate::stream::*;

        // Endpoint Discovery
        let disc = StreamFactory::endpoint_discovery(1, 1, 0x1F);
        assert_eq!(disc.message_type(), MessageType::Stream);
        assert_eq!(disc.data[0], 0xF0000101);
        assert_eq!(disc.data[1], 0x1F000000);

        // Endpoint Info Notification
        let info = StreamFactory::endpoint_info_notify(1, 1, 4, true, true, false, true);
        assert_eq!(info.message_type(), MessageType::Stream);
        assert_eq!(info.data[0], 0xF0010101);
        assert_eq!(info.data[1], (4 << 24) | (1 << 9) | (1 << 8) | 1);

        // Device Info Notification
        let dev = StreamFactory::endpoint_device_info_notify(
            [0x01, 0x02, 0x03],
            [0x10, 0x20],
            [0x30, 0x40],
            [0x01, 0x00, 0x00, 0x00],
        );
        assert_eq!(dev.message_type(), MessageType::Stream);
        assert_eq!(dev.data[0], 0xF0020000);
        assert_eq!(dev.data[1], 0x00010203);
        assert_eq!(dev.data[2], 0x10203040);
        assert_eq!(dev.data[3], 0x01000000);

        // Stream Config Request & Notify
        let req = StreamFactory::stream_config_request(2, true, false);
        assert_eq!(req.data[0], 0xF0050202);

        let notif = StreamFactory::stream_config_notify(2, true, false);
        assert_eq!(notif.data[0], 0xF0060202);

        // Sequence Markers
        let seq_start = StreamFactory::start_of_sequence();
        assert_eq!(seq_start.data[0], 0xF0200000);

        let eof = StreamFactory::end_of_file();
        assert_eq!(eof.data[0], 0xF0210000);

        // Function Block Info Notification
        let fb_info =
            StreamFactory::function_block_info_notify(2, true, 2, true, true, 0, 1, 1, 0, 8);
        assert_eq!(fb_info.message_type(), MessageType::Stream);
        assert_eq!(
            (fb_info.data[0] >> 16) & 0x3FF,
            STATUS_FUNCTION_BLOCK_INFO_NOTIFICATION as u32
        );
        assert_eq!((fb_info.data[1] >> 24) & 0xFF, 0); // first group
        assert_eq!((fb_info.data[1] >> 16) & 0xFF, 1); // group length
    }

    #[test]
    fn test_stream_text_chunker_and_receiver() {
        use crate::stream::*;

        // Endpoint Name roundtrip
        let name = b"Grand Synth Pro 2.0";
        let chunker = StreamTextChunker::endpoint_name(name);
        let packets: alloc::vec::Vec<_> = chunker.collect();
        // 19 bytes total: packet 1 holds 14 bytes, packet 2 holds 5 bytes
        assert_eq!(packets.len(), 2);

        let mut receiver = StreamTextReceiver::<64>::new();
        assert_eq!(
            receiver.process_ump(&packets[0]),
            Ok(StreamTextReceiverState::Receiving)
        );
        assert_eq!(
            receiver.process_ump(&packets[1]),
            Ok(StreamTextReceiverState::Complete)
        );
        assert_eq!(receiver.text(), name);
        assert_eq!(receiver.status(), STATUS_ENDPOINT_NAME_NOTIFICATION);
        assert_eq!(receiver.function_block_index(), None);

        // Function Block Name roundtrip (single packet complete)
        let fb_name = b"Main Engine";
        let fb_chunker = StreamTextChunker::function_block_name(3, fb_name);
        let fb_packets: alloc::vec::Vec<_> = fb_chunker.collect();
        assert_eq!(fb_packets.len(), 1);

        let mut fb_receiver = StreamTextReceiver::<32>::new();
        assert_eq!(
            fb_receiver.process_ump(&fb_packets[0]),
            Ok(StreamTextReceiverState::Complete)
        );
        assert_eq!(fb_receiver.text(), fb_name);
        assert_eq!(
            fb_receiver.status(),
            STATUS_FUNCTION_BLOCK_NAME_NOTIFICATION
        );
        assert_eq!(fb_receiver.function_block_index(), Some(3));

        // Error handling: buffer overflow
        let mut small_receiver = StreamTextReceiver::<8>::new();
        assert_eq!(
            small_receiver.process_ump(&packets[0]),
            Err(StreamTextReceiverError::BufferOverflow)
        );

        // Error handling: unexpected continue format
        let mut err_receiver = StreamTextReceiver::<64>::new();
        assert_eq!(
            err_receiver.process_ump(&packets[1]),
            Err(StreamTextReceiverError::UnexpectedFormat)
        );
    }
}
