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
    fn test_midi2_per_note_pitch_bend() {
        let group = 1;
        let channel = 2;
        let note = 60;
        let value: u32 = 0x80000000;
        let ump = UmpFactory::midi2_per_note_pitch_bend(group, channel, note, value);

        assert_eq!(ump.message_type(), MessageType::Midi2ChannelVoice);
        assert_eq!(ump.group(), group);
        assert_eq!(ump.channel(), channel);
        assert_eq!(ump.status(), PITCH_BEND_PERNOTE);
        assert_eq!(ump.data[0], 0x41623C00);
        assert_eq!(ump.data[1], value);
    }

    #[test]
    fn test_midi2_per_note_cc() {
        let group = 1;
        let channel = 2;
        let note = 60;
        let index = 5;
        let value: u32 = 0x12345678;
        let ump = UmpFactory::midi2_per_note_cc(group, channel, note, index, value);

        assert_eq!(ump.message_type(), MessageType::Midi2ChannelVoice);
        assert_eq!(ump.group(), group);
        assert_eq!(ump.channel(), channel);
        assert_eq!(ump.status(), NRPN_PERNOTE);
        // MT=0x4, Grp=1, Status=0x1, Ch=2, Note=0x3C, Index=0x05
        assert_eq!(ump.data[0], 0x41123C05);
        assert_eq!(ump.data[1], value);
    }

    #[test]
    fn test_midi2_per_note_rpn() {
        let group = 1;
        let channel = 2;
        let note = 60;
        let index = 5;
        let value: u32 = 0x12345678;
        let ump = UmpFactory::midi2_per_note_rpn(group, channel, note, index, value);

        assert_eq!(ump.message_type(), MessageType::Midi2ChannelVoice);
        assert_eq!(ump.group(), group);
        assert_eq!(ump.channel(), channel);
        assert_eq!(ump.status(), RPN_PERNOTE);
        // MT=0x4, Grp=1, Status=0x0, Ch=2, Note=0x3C, Index=0x05
        assert_eq!(ump.data[0], 0x41023C05);
        assert_eq!(ump.data[1], value);
    }

    #[test]
    fn test_midi2_per_note_manage() {
        let group = 1;
        let channel = 2;
        let note = 60;
        let option_flags = 3; // bit 0 (detach) + bit 1 (reset)
        let ump = UmpFactory::midi2_per_note_manage(group, channel, note, option_flags);

        assert_eq!(ump.message_type(), MessageType::Midi2ChannelVoice);
        assert_eq!(ump.group(), group);
        assert_eq!(ump.channel(), channel);
        assert_eq!(ump.status(), PERNOTE_MANAGE);
        // MT=0x4, Grp=1, Status=0xF, Ch=2, Note=0x3C, Flags=0x03
        assert_eq!(ump.data[0], 0x41F23C03);
        assert_eq!(ump.data[1], 0);
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

        // Function Block Discovery
        let fb_disc = StreamFactory::function_block_discovery(3, 0x01);
        assert_eq!(fb_disc.message_type(), MessageType::Stream);
        assert_eq!(fb_disc.data[0], 0xF0100301);
        assert_eq!(fb_disc.data[1], 0);

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

        // Function Block Struct & Helper Methods
        let fb = FunctionBlock::new(5, FunctionBlockDirection::Bidirectional, 2, 4)
            .midi_ci_version(1)
            .is_midi1(0)
            .max_s8_streams(16);
        assert_eq!(fb.fb_idx, 5);
        assert!(fb.active);
        assert_eq!(fb.direction, FunctionBlockDirection::Bidirectional);
        assert!(fb.sender);
        assert!(fb.receiver);
        assert_eq!(fb.first_group, 2);
        assert_eq!(fb.group_length, 4);
        assert_eq!(fb.midi_ci_version, 1);

        let fb_ump = fb.to_info_notification();
        assert_eq!(fb_ump.message_type(), MessageType::Stream);
        assert_eq!(
            (fb_ump.data[0] >> 16) & 0x3FF,
            STATUS_FUNCTION_BLOCK_INFO_NOTIFICATION as u32
        );
        assert_eq!((fb_ump.data[0] >> 15) & 0x1, 1); // active
        assert_eq!((fb_ump.data[0] >> 8) & 0x7F, 5); // fb_idx
        assert_eq!(fb_ump.data[0] & 0x3, 2); // direction bidirectional
        assert_eq!((fb_ump.data[1] >> 24) & 0x1F, 2); // first group
        assert_eq!((fb_ump.data[1] >> 16) & 0x1F, 4); // group length
        assert_eq!((fb_ump.data[1] >> 8) & 0x7F, 1); // midi_ci_version
        assert_eq!(fb_ump.data[1] & 0xFF, 16); // max_s8_streams

        let disc_req = FunctionBlock::discovery_request(5, 0xFF);
        assert_eq!(disc_req.data[0], 0xF01005FF);

        // Direction enum from_u8 check
        assert_eq!(
            FunctionBlockDirection::from_u8(0),
            FunctionBlockDirection::Input
        );
        assert_eq!(
            FunctionBlockDirection::from_u8(1),
            FunctionBlockDirection::Output
        );
        assert_eq!(
            FunctionBlockDirection::from_u8(2),
            FunctionBlockDirection::Bidirectional
        );
        assert_eq!(
            FunctionBlockDirection::from_u8(3),
            FunctionBlockDirection::Bidirectional
        );
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

    #[test]
    fn test_poly_pressure_scaling() {
        use crate::utils::scale_up;
        // MT=0x2 Poly Pressure 7-bit to 32-bit scaling check
        let scaled_0 = scale_up(0, 7, 32);
        assert_eq!(scaled_0, 0);

        let scaled_127 = scale_up(127, 7, 32);
        assert_eq!(scaled_127, 0xFFFFFFFF);

        let scaled_64 = scale_up(64, 7, 32);
        assert_eq!(scaled_64, 0x80000000);
    }

    #[test]
    fn test_lossless_m1cvm() {
        use crate::ump::{Midi1ChannelVoice, Ump};

        let mut ump = Ump::new();
        // MT=2, group=3, status=0xA0 (Poly Pressure), channel=4, note=60, pressure=100
        ump.data[0] = 0x23A43C64;
        let cvm = Midi1ChannelVoice::try_from(&ump).expect("should decode MT=2");
        assert_eq!(cvm.group, 3);
        assert_eq!(cvm.status, 0xA0);
        assert_eq!(cvm.channel, 4);
        assert_eq!(cvm.data1, 60);
        assert_eq!(cvm.data2, 100);

        // MT=4 should fail
        let mut ump4 = Ump::new();
        ump4.data[0] = 0x40903C00;
        assert!(Midi1ChannelVoice::try_from(&ump4).is_err());
    }

    #[test]
    fn test_flex_data_creation_roundtrip() {
        use crate::messages::UmpFactory;
        use crate::ump::MessageType;
        use crate::utils::{FLEXDATA_COMMON, FLEXDATA_COMMON_TEMPO, FLEXDATA_LYRIC};

        // Header check
        let header = UmpFactory::flex_header(1, 0, 1, 2, FLEXDATA_COMMON, FLEXDATA_COMMON_TEMPO);
        assert_eq!(header, 0xD1120000);

        // Tempo test matching upstream mtDFlexTempo(1, 2, 0, 1, 0x12345678)
        let tempo = UmpFactory::flex_tempo(1, 2, 0, 1, 0x12345678);
        assert_eq!(tempo.message_type(), MessageType::FlexData);
        assert_eq!(tempo.group(), 1);
        assert_eq!(tempo.data[0], 0xD1120000);
        assert_eq!(tempo.data[1], 0x12345678);
        assert_eq!(tempo.data[2], 0);
        assert_eq!(tempo.data[3], 0);

        // Time Signature test matching upstream mtDFlexTimeSig(1, 2, 0, 1, 4, 4, 8)
        let timesig = UmpFactory::flex_time_sig(1, 2, 0, 1, 4, 4, 8);
        assert_eq!(timesig.message_type(), MessageType::FlexData);
        assert_eq!(timesig.data[0], 0xD1120001);
        assert_eq!(timesig.data[1], 0x04040800);
        assert_eq!(timesig.data[2], 0);
        assert_eq!(timesig.data[3], 0);

        // Metronome test matching upstream mtDFlexMetronome(1, 2, 0, 1, 10, 11, 12, 13, 14, 15)
        let metronome = UmpFactory::flex_metronome(1, 2, 0, 1, 10, 11, 12, 13, 14, 15);
        assert_eq!(metronome.message_type(), MessageType::FlexData);
        assert_eq!(metronome.data[0], 0xD1120002);
        assert_eq!(metronome.data[1], 0x0A0B0C0D);
        assert_eq!(metronome.data[2], 0x0E0F0000);
        assert_eq!(metronome.data[3], 0);

        // Key Signature test matching upstream mtDFlexKeySig(1, 2, 0, 1, 5, 2)
        let keysig = UmpFactory::flex_key_sig(1, 2, 0, 1, 5, 2);
        assert_eq!(keysig.message_type(), MessageType::FlexData);
        assert_eq!(keysig.data[0], 0xD1120005);
        assert_eq!(keysig.data[1], 0x05020000);
        assert_eq!(keysig.data[2], 0);
        assert_eq!(keysig.data[3], 0);

        // Chord test matching upstream mtDFlexChord(1, 2, 0, 1, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2)
        let chord = UmpFactory::flex_chord(
            1, 2, 0, 1, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2,
        );
        assert_eq!(chord.message_type(), MessageType::FlexData);
        assert_eq!(chord.data[0], 0xD1120006);
        assert_eq!(chord.data[1], 0x12034567);
        assert_eq!(chord.data[2], 0x89AB0000);
        assert_eq!(chord.data[3], 0xCD0EF012);

        // Text test matching upstream mtDFlexText(1, 2, 0, 1, FLEXDATA_LYRIC, 0, text, 5)
        let text = b"Hello";
        let text_ump = UmpFactory::flex_text(1, 2, 0, 1, FLEXDATA_LYRIC, 0, text);
        assert_eq!(text_ump.message_type(), MessageType::FlexData);
        assert_eq!(text_ump.data[0], 0xD1120200);
        assert_eq!(text_ump.data[1], 0x48656C6C);
        assert_eq!(text_ump.data[2], 0x6F000000);
        assert_eq!(text_ump.data[3], 0);

        // Lyric text helper
        let lyric_ump = UmpFactory::flex_lyric(1, 2, 0, 1, 0, text);
        assert_eq!(lyric_ump.data, text_ump.data);

        // Performance text helper
        let perf_ump = UmpFactory::flex_performance(1, 2, 0, 1, 0, text);
        assert_eq!(perf_ump.data[0], 0xD1120100);
        assert_eq!(perf_ump.data[1], 0x48656C6C);
        assert_eq!(perf_ump.data[2], 0x6F000000);
        assert_eq!(perf_ump.data[3], 0);
    }

    #[test]
    fn test_ci_7bit_packing() {
        use crate::ci::*;

        assert_eq!(pack_7bit_u16(0x3FFF), [0x7F, 0x7F]);
        assert_eq!(unpack_7bit_u16(&[0x7F, 0x7F]), 0x3FFF);
        assert_eq!(unpack_7bit_u16(&[0x05, 0x01]), 5 | (1 << 7));

        let muid: u32 = 0x05A3B2C1 & 0x0FFFFFFF;
        let packed = pack_7bit_u32(muid);
        let unpacked = unpack_7bit_u32(&packed);
        assert_eq!(unpacked, muid);
    }

    #[test]
    fn test_ci_header_roundtrip() {
        use crate::ci::*;

        let hdr = CiHeader::new(0x7F, MIDICI_DISCOVERY, 2, 0x12345, 0x6789A);
        let bytes = hdr.to_bytes();
        assert_eq!(bytes[0], S7UNIVERSAL_NRT);
        assert_eq!(bytes[1], 0x7F);
        assert_eq!(bytes[2], S7MIDICI);
        assert_eq!(bytes[3], MIDICI_DISCOVERY);
        assert_eq!(bytes[4], 2);

        let parsed = CiHeader::parse(&bytes).expect("should parse header");
        assert_eq!(parsed, hdr);
    }

    #[test]
    fn test_ci_discovery_inquiry_and_reply() {
        use crate::ci::*;

        let dev_info = CiDeviceInfo {
            manufacturer_id: [0x01, 0x02, 0x03],
            family_id: [0x10, 0x20],
            model_id: [0x30, 0x40],
            version: [0x01, 0x00, 0x00, 0x00],
            ci_support: 0x07,
            max_sysex: 512,
            output_path_id: 1,
            fb_idx: 2,
        };

        // Inquiry
        let inq_payload = CiDiscovery::build_inquiry_payload(0x1234, &dev_info, 2);
        assert_eq!(inq_payload.len(), 30);
        let (inq_hdr, inq_dev) =
            CiDiscovery::parse_discovery(&inq_payload).expect("should parse inquiry");
        assert_eq!(inq_hdr.source_muid, 0x1234);
        assert_eq!(inq_hdr.destination_muid, M2_CI_BROADCAST);
        assert_eq!(inq_dev.manufacturer_id, [0x01, 0x02, 0x03]);
        assert_eq!(inq_dev.output_path_id, 1);

        // Reply
        let reply_payload = CiDiscovery::build_reply_payload(0x5678, 0x1234, &dev_info, 2);
        assert_eq!(reply_payload.len(), 31);
        let (rep_hdr, rep_dev) =
            CiDiscovery::parse_discovery(&reply_payload).expect("should parse reply");
        assert_eq!(rep_hdr.source_muid, 0x5678);
        assert_eq!(rep_hdr.destination_muid, 0x1234);
        assert_eq!(rep_dev.fb_idx, 2);
    }

    #[test]
    fn test_ci_initiator_and_responder_state_machine() {
        use crate::ci::*;
        use crate::sysex::SysEx7Receiver;

        let initiator_info = CiDeviceInfo {
            manufacturer_id: [0x00, 0x01, 0x02],
            family_id: [0x01, 0x00],
            model_id: [0x02, 0x00],
            version: [0x01, 0x00, 0x00, 0x00],
            ci_support: 0x07,
            max_sysex: 1024,
            output_path_id: 0,
            fb_idx: 0,
        };

        let responder_info = CiDeviceInfo {
            manufacturer_id: [0x03, 0x04, 0x05],
            family_id: [0x03, 0x00],
            model_id: [0x04, 0x00],
            version: [0x02, 0x00, 0x00, 0x00],
            ci_support: 0x07,
            max_sysex: 1024,
            output_path_id: 0,
            fb_idx: 1,
        };

        let mut initiator = CiInitiator::new(0x11111, initiator_info);
        let mut responder = CiResponder::new(0x22222, responder_info);

        assert_eq!(initiator.state(), CiInitiatorState::Uninitialized);
        assert_eq!(responder.state(), CiResponderState::Idle);

        // 1. Initiator starts discovery
        let (inq_buf, inq_len) = initiator.start_discovery(2);
        assert_eq!(initiator.state(), CiInitiatorState::DiscoverySent);

        // Transmit Inquiry over UMP chunker & receiver simulation
        let chunker = CiDiscovery::create_inquiry_ump_chunker(0, &inq_buf[..inq_len]);
        let mut rx_inquiry = SysEx7Receiver::<64>::new();
        for ump in chunker {
            let _ = rx_inquiry.process_ump(&ump);
        }
        assert_eq!(rx_inquiry.payload(), &inq_buf[..inq_len]);

        // 2. Responder receives inquiry and generates reply
        let (reply_buf, reply_len) = responder
            .handle_discovery_inquiry(rx_inquiry.payload())
            .expect("should generate discovery reply");
        assert_eq!(responder.state(), CiResponderState::Discovered);

        // 3. Initiator receives discovery reply
        let res = initiator.process_message(&reply_buf[..reply_len]);
        assert_eq!(res, Ok(CiInitiatorState::DiscoveryReceived));
        assert_eq!(initiator.remote_muid(), Some(0x22222));

        // 4. Protocol negotiation: Initiator inquires
        let (proto_inq, proto_len) = CiProtocolNegotiation::build_inquiry_payload(
            0x11111,
            0x22222,
            1,
            &[MIDI_PROTOCOL_2, MIDI_PROTOCOL_1],
        );
        let (proto_rep, rep_len) = responder
            .handle_protocol_inquiry(&proto_inq[..proto_len])
            .expect("should handle protocol inquiry");
        assert_eq!(responder.state(), CiResponderState::ProtocolNegotiating);

        // 5. Initiator processes protocol reply
        let res = initiator.process_message(&proto_rep[..rep_len]);
        assert_eq!(res, Ok(CiInitiatorState::ProtocolNegotiating));

        // 6. Protocol confirmation
        let confirm_payload =
            CiProtocolNegotiation::build_protocol_confirm_payload(0x22222, 0x11111, 1);
        let res = initiator.process_message(&confirm_payload);
        assert_eq!(res, Ok(CiInitiatorState::Negotiated));
    }

    #[test]
    fn test_ci_protocol_negotiation_messages() {
        use crate::ci::*;

        let (inq, len) = CiProtocolNegotiation::build_inquiry_payload(
            100,
            200,
            2,
            &[MIDI_PROTOCOL_2, MIDI_PROTOCOL_1],
        );
        assert_eq!(len, 25);
        assert_eq!(inq[13], 2); // authority
        assert_eq!(inq[14], 2); // num protocols
        assert_eq!(&inq[15..20], &MIDI_PROTOCOL_2);
        assert_eq!(&inq[20..25], &MIDI_PROTOCOL_1);

        let set_proto =
            CiProtocolNegotiation::build_set_protocol_payload(100, 200, 2, MIDI_PROTOCOL_2);
        assert_eq!(set_proto.len(), 19);
        assert_eq!(&set_proto[14..19], &MIDI_PROTOCOL_2);

        let test_proto = CiProtocolNegotiation::build_protocol_test_payload(100, 200, 2);
        assert_eq!(test_proto.len(), 62);
        assert_eq!(test_proto[13], 2);
        assert_eq!(test_proto[14], 0);
        assert_eq!(test_proto[61], 47);

        let test_resp = CiProtocolNegotiation::build_protocol_test_responder_payload(200, 100, 2);
        assert_eq!(test_resp.len(), 62);
        assert_eq!(test_resp[14], 0);
        assert_eq!(test_resp[61], 47);
    }

    #[test]
    fn test_ci_profile_configuration_messages() {
        use crate::ci::*;
        use crate::sysex::SysEx7Chunker;

        let src_muid = 0x12345;
        let dst_muid = 0x6789A;
        let p_standard = ProfileId::standard(1, 2, 1, 0);
        assert!(p_standard.is_standard());
        assert_eq!(p_standard.raw, [0x7E, 1, 2, 1, 0]);

        // 1. Profile Inquiry
        let inq = CiProfile::build_inquiry_payload(src_muid, dst_muid, 0x7F, 1);
        assert_eq!(inq.len(), 13);
        let hdr = CiHeader::parse(&inq).expect("valid header");
        assert_eq!(hdr.ci_type, MIDICI_PROFILE_INQUIRY);

        // 2. Profile Inquiry Reply
        let p_custom = ProfileId::new([0x01, 0x02, 0x03, 0x04, 0x05]);
        assert!(!p_custom.is_standard());
        let (rep, rep_len) = CiProfile::build_inquiry_reply_payload(
            dst_muid,
            src_muid,
            0x7F,
            1,
            &[p_standard],
            &[p_custom],
        );
        assert_eq!(rep_len, 13 + 2 + 5 + 2 + 5);
        let rep_hdr = CiHeader::parse(&rep[..rep_len]).expect("valid reply header");
        assert_eq!(rep_hdr.ci_type, MIDICI_PROFILE_INQUIRYREPLY);

        // 3. Set Profile On (CI v1 = 18 bytes, CI v2 = 20 bytes)
        let (set_on_v1, len_v1) =
            CiProfile::build_set_profile_on(src_muid, dst_muid, 0x7F, 1, p_standard, 1);
        assert_eq!(len_v1, 18);
        let (hdr_on_v1, parsed_p1, ch1) =
            CiProfile::parse_profile_message(&set_on_v1[..len_v1]).expect("parse v1");
        assert_eq!(hdr_on_v1.ci_type, MIDICI_PROFILE_SETON);
        assert_eq!(parsed_p1, p_standard);
        assert_eq!(ch1, 0);

        let (set_on_v2, len_v2) =
            CiProfile::build_set_profile_on(src_muid, dst_muid, 0x7F, 2, p_standard, 4);
        assert_eq!(len_v2, 20);
        let (hdr_on_v2, parsed_p2, ch2) =
            CiProfile::parse_profile_message(&set_on_v2[..len_v2]).expect("parse v2");
        assert_eq!(hdr_on_v2.ci_type, MIDICI_PROFILE_SETON);
        assert_eq!(parsed_p2, p_standard);
        assert_eq!(ch2, 4);

        // 4. Set Profile Off
        let (set_off, off_len) =
            CiProfile::build_set_profile_off(src_muid, dst_muid, 0x7F, 2, p_standard);
        assert_eq!(off_len, 20);
        let (hdr_off, parsed_off, _) =
            CiProfile::parse_profile_message(&set_off[..off_len]).expect("parse off");
        assert_eq!(hdr_off.ci_type, MIDICI_PROFILE_SETOFF);
        assert_eq!(parsed_off, p_standard);

        // 5. Profile Specific Data
        let test_data = [0x11, 0x22, 0x33, 0x44];
        let (data_msg, data_len) = CiProfile::build_profile_specific_data(
            src_muid, dst_muid, 0x7F, 2, p_standard, &test_data,
        );
        assert_eq!(data_len, 22 + 4);
        let data_hdr = CiHeader::parse(&data_msg[..data_len]).expect("parse data msg");
        assert_eq!(data_hdr.ci_type, MIDICI_PROFILE_SPECIFIC_DATA);

        // 6. SysEx7 chunking roundtrip of Profile On payload
        let chunker = SysEx7Chunker::new(2, &set_on_v2[..len_v2]);
        let umps: alloc::vec::Vec<_> = chunker.collect();
        assert_eq!(umps.len(), 4); // 20 bytes -> ceil(20 / 6) = 4 packets
    }

    #[test]
    fn test_ci_profile_state_machine_handshake() {
        use crate::ci::*;

        let dev_info = CiDeviceInfo::default();
        let mut initiator = CiInitiator::new(0x11111, dev_info);
        let mut responder = CiResponder::new(0x22222, dev_info);

        let p1 = ProfileId::standard(1, 2, 1, 0);
        let p2 = ProfileId::new([0x01, 0x02, 0x03, 0x04, 0x05]);
        assert!(responder.add_supported_profile(p1));
        assert!(responder.add_supported_profile(p2));

        // Advance to Negotiated
        let (inq_buf, inq_len) = initiator.start_discovery(2);
        let (reply_buf, reply_len) = responder
            .handle_discovery_inquiry(&inq_buf[..inq_len])
            .unwrap();
        initiator.process_message(&reply_buf[..reply_len]).unwrap();

        let (proto_inq, proto_len) = CiProtocolNegotiation::build_inquiry_payload(
            0x11111,
            0x22222,
            1,
            &[MIDI_PROTOCOL_2, MIDI_PROTOCOL_1],
        );
        let (proto_rep, rep_len) = responder
            .handle_protocol_inquiry(&proto_inq[..proto_len])
            .unwrap();
        initiator.process_message(&proto_rep[..rep_len]).unwrap();

        let confirm_payload =
            CiProtocolNegotiation::build_protocol_confirm_payload(0x22222, 0x11111, 1);
        initiator.process_message(&confirm_payload).unwrap();
        assert_eq!(initiator.state(), CiInitiatorState::Negotiated);

        // Initiator requests Profile Inquiry
        let prof_inq = CiProfile::build_inquiry_payload(0x11111, 0x22222, 0x7F, 2);
        let (prof_reply, prof_reply_len) = responder
            .handle_profile_inquiry(&prof_inq)
            .expect("should handle profile inquiry");
        assert_eq!(responder.state(), CiResponderState::ProfileConfiguring);
        assert_eq!(prof_reply_len, 13 + 2 + 2 + 10); // 0 enabled, 2 disabled (p1 and p2)
        assert_eq!(prof_reply[3], MIDICI_PROFILE_INQUIRYREPLY);

        // Initiator sends Set Profile On for p1
        let (set_on, set_on_len) =
            CiProfile::build_set_profile_on(0x11111, 0x22222, 0x7F, 2, p1, 1);
        let (en_report, en_report_len) = responder
            .handle_set_profile_on(&set_on[..set_on_len])
            .expect("should enable profile");
        assert_eq!(responder.state(), CiResponderState::ProfileConfiguring);

        // Initiator processes Profile Enabled Report
        initiator
            .process_message(&en_report[..en_report_len])
            .unwrap();
        assert_eq!(initiator.state(), CiInitiatorState::ProfileConfiguring);
        assert_eq!(initiator.enabled_profiles().len(), 1);
        assert_eq!(initiator.enabled_profiles()[0], Some(p1));

        // Responder inquiry now reports p1 enabled and p2 disabled
        let (prof_reply2, rep2_len) = responder
            .handle_profile_inquiry(&prof_inq)
            .expect("reply again");
        assert_eq!(rep2_len, 13 + 2 + 5 + 2 + 5);
        assert_eq!(prof_reply2[3], MIDICI_PROFILE_INQUIRYREPLY);

        // Turn off p1
        let (set_off, set_off_len) =
            CiProfile::build_set_profile_off(0x11111, 0x22222, 0x7F, 2, p1);
        let (dis_report, dis_report_len) = responder
            .handle_set_profile_off(&set_off[..set_off_len])
            .expect("should disable profile");
        initiator
            .process_message(&dis_report[..dis_report_len])
            .unwrap();
        assert_eq!(initiator.enabled_profiles().len(), 0);
    }

    #[test]
    fn test_transport_and_mock_transport() {
        use crate::stream::StreamFactory;
        use crate::transport::{MockTransport, Transport, TransportError};

        let mut transport = MockTransport::<16>::new();
        assert_eq!(transport.len(), 0);
        assert!(transport.is_empty());

        let disc = StreamFactory::endpoint_discovery(1, 1, 0xFF);
        let notify = StreamFactory::stream_config_notify(2, true, true);
        let batch = [disc, notify];

        // Send batch
        assert!(transport.send(&batch).is_ok());
        assert_eq!(transport.len(), 2);

        // Receive sequentially
        let r1 = transport.recv().expect("recv ok").expect("some packet");
        assert_eq!(r1, disc);
        assert_eq!(transport.len(), 1);

        let r2 = transport.recv().expect("recv ok").expect("some packet");
        assert_eq!(r2, notify);
        assert_eq!(transport.len(), 0);

        // Receive from empty returns None
        assert_eq!(transport.recv().expect("recv ok"), None);

        // Buffer full error
        let full_batch = [disc; 17];
        assert_eq!(
            transport.send(&full_batch),
            Err(TransportError::TxBufferFull)
        );

        // Disconnected error
        transport.set_connected(false);
        assert_eq!(transport.send(&[disc]), Err(TransportError::NotConnected));
        assert_eq!(transport.recv(), Err(TransportError::NotConnected));
    }

    #[test]
    fn test_ci_pe_json_tokenizer_and_header() {
        use crate::ci_pe::{JsonTokenType, JsonTokenizer, PeHeader};

        let json = b"{\"resource\":\"DeviceInfo\",\"resId\":\"dev1\",\"action\":\"get\",\"status\":200,\"cacheTime\":60}";
        let mut tok = JsonTokenizer::new(json);

        assert_eq!(tok.next(), Some(JsonTokenType::StartObject));
        assert_eq!(tok.next(), Some(JsonTokenType::Key("resource")));
        assert_eq!(tok.next(), Some(JsonTokenType::StringValue("DeviceInfo")));
        assert_eq!(tok.next(), Some(JsonTokenType::Key("resId")));
        assert_eq!(tok.next(), Some(JsonTokenType::StringValue("dev1")));
        assert_eq!(tok.next(), Some(JsonTokenType::Key("action")));
        assert_eq!(tok.next(), Some(JsonTokenType::StringValue("get")));
        assert_eq!(tok.next(), Some(JsonTokenType::Key("status")));
        assert_eq!(tok.next(), Some(JsonTokenType::NumberValue(200)));
        assert_eq!(tok.next(), Some(JsonTokenType::Key("cacheTime")));
        assert_eq!(tok.next(), Some(JsonTokenType::NumberValue(60)));
        assert_eq!(tok.next(), Some(JsonTokenType::EndObject));
        assert_eq!(tok.next(), None);

        // Header view parse
        let hdr = PeHeader::parse(json);
        assert_eq!(hdr.resource, Some("DeviceInfo"));
        assert_eq!(hdr.res_id, Some("dev1"));
        assert_eq!(hdr.action, Some("get"));
        assert_eq!(hdr.status, Some(200));
        assert_eq!(hdr.cache_time, Some(60));
    }

    #[test]
    fn test_ci_pe_transaction_manager() {
        use crate::ci_pe::PeTransactionManager;

        let mut mgr = PeTransactionManager::<4>::new();
        assert_eq!(mgr.active_count(), 0);

        let _id1 = mgr.start_request(0x1000, 10).expect("req 1");
        let id2 = mgr.start_request(0x2000, 12).expect("req 2");
        let _id3 = mgr.start_request(0x3000, 15).expect("req 3");
        let _id4 = mgr.start_request(0x4000, 20).expect("req 4");

        assert_eq!(mgr.active_count(), 4);
        // Exceeding capacity returns None
        assert_eq!(mgr.start_request(0x5000, 22), None);

        // Complete request 2
        assert!(mgr.complete_request(id2, 0x2000));
        assert_eq!(mgr.active_count(), 3);

        // Matching unknown request fails
        assert!(!mgr.complete_request(99, 0x9999));

        // Timeouts
        mgr.clean_timeouts(25, 11); // id1 (10) timed out since 25 - 10 = 15 >= 11
        assert_eq!(mgr.active_count(), 2);
    }

    #[test]
    fn test_ci_pe_subscription_manager() {
        use crate::ci_pe::{hash_resource, SubscriptionManager};

        let mut mgr = SubscriptionManager::<4>::new();
        let hash_dev = hash_resource("DeviceInfo");
        let hash_modes = hash_resource("Mode");

        assert_ne!(hash_dev, hash_modes);

        assert!(mgr.add_subscription(0x111, hash_dev));
        assert!(mgr.add_subscription(0x222, hash_dev));
        assert!(mgr.add_subscription(0x333, hash_modes));
        assert_eq!(mgr.len(), 3);

        // Query subscribers
        let mut subs = [0_u32; 4];
        let n = mgr.get_subscribers(hash_dev, &mut subs);
        assert_eq!(n, 2);
        assert_eq!(&subs[..2], &[0x111, 0x222]);

        // Remove subscription
        assert!(mgr.remove_subscription(0x111, hash_dev));
        assert_eq!(mgr.len(), 2);
        let n2 = mgr.get_subscribers(hash_dev, &mut subs);
        assert_eq!(n2, 1);
        assert_eq!(subs[0], 0x222);
    }

    #[test]
    fn test_ci_pe_message_serialization_and_chunking() {
        use crate::ci_pe::{CiPe, PeReplyChunker, MIDICI_PE_GET, MIDICI_PE_GETREPLY};
        use crate::sysex::SysEx7Receiver;

        let src_muid = 0x11111;
        let dst_muid = 0x22222;
        let json_header = b"{\"resource\":\"ResourceList\",\"status\":200}";

        // 1. Serialize Header-Only Get message
        let mut buf = [0_u8; 128];
        let len = CiPe::serialize_pe_header_only(
            &mut buf,
            MIDICI_PE_GET,
            2,
            src_muid,
            dst_muid,
            7,
            json_header,
        )
        .expect("serialize pe get");

        let (parsed_hdr, pe_msg) = CiPe::parse_pe_message(&buf[..len]).expect("parse pe message");
        assert_eq!(parsed_hdr.ci_type, MIDICI_PE_GET);
        assert_eq!(parsed_hdr.source_muid, src_muid);
        assert_eq!(pe_msg.request_id, 7);
        assert_eq!(pe_msg.header.resource, Some("ResourceList"));
        assert_eq!(pe_msg.header.status, Some(200));
        assert_eq!(pe_msg.body, None);

        // 2. Serialize Message with Body (Get Reply)
        let body = b"{\"resources\":[\"DeviceInfo\",\"ChannelList\"]}";
        let mut reply_buf = [0_u8; 256];
        let rep_len = CiPe::serialize_pe_with_body(
            &mut reply_buf,
            MIDICI_PE_GETREPLY,
            2,
            dst_muid,
            src_muid,
            7,
            json_header,
            1,
            1,
            body,
        )
        .expect("serialize get reply");

        let (_, rep_msg) = CiPe::parse_pe_message(&reply_buf[..rep_len]).expect("parse reply");
        assert_eq!(rep_msg.request_id, 7);
        assert_eq!(rep_msg.body, Some(&body[..]));

        // 3. SysEx7 chunking and reassembly
        let chunker = PeReplyChunker::new_sysex7(3, &reply_buf[..rep_len]);
        let mut rx = SysEx7Receiver::<256>::new();
        for ump in chunker {
            let _ = rx.process_ump(&ump);
        }
        assert_eq!(rx.payload(), &reply_buf[..rep_len]);
    }

    #[test]
    fn test_ci_process_inquiry_messages() {
        use crate::ci_pe::{CiProcessInquiry, MIDICI_PI_CAPABILITY, MIDICI_PI_MM_REPORT};

        // PI Capabilities Request & Reply
        let req = CiProcessInquiry::build_capabilities_request(0x10, 0x20, 2);
        assert_eq!(req[3], MIDICI_PI_CAPABILITY);

        let rep = CiProcessInquiry::build_capabilities_reply(0x20, 0x10, 2, 0x03);
        assert_eq!(rep[13], 0x03);

        // MIDI Message Report
        let report =
            CiProcessInquiry::build_midi_message_report(0x10, 0x20, 0x7F, 2, 0, 0x7E, 0x7F, 0x7F);
        assert_eq!(report[3], MIDICI_PI_MM_REPORT);
        assert_eq!(report[13], 0); // MDC
        assert_eq!(report[14], 0x7E); // systemBitmap
        assert_eq!(report[16], 0x7F); // chanContBitmap
        assert_eq!(report[17], 0x7F); // chanNoteBitmap
    }

    #[test]
    fn test_phase6_usb_descriptors_and_packet_framing() {
        use crate::transport::usb::{
            GroupTerminalBlock, GroupTerminalBlockType, GroupTerminalProtocol,
            UsbMidi2HeaderDescriptor, UsbMidiPacketPacker,
        };

        // 1. Group Terminal Block Descriptor Serialization & Parsing
        let gtb = GroupTerminalBlock::new(1, GroupTerminalBlockType::Bidirectional, 0, 4)
            .protocol(GroupTerminalProtocol::Midi2_0)
            .max_sysex8_streams(8)
            .string_index(2);

        let mut gtb_buf = [0_u8; 13];
        gtb.serialize(&mut gtb_buf);
        assert_eq!(gtb_buf[0], 13);
        assert_eq!(gtb_buf[1], 0x24);
        assert_eq!(gtb_buf[2], 0x03);
        assert_eq!(gtb_buf[3], 1);
        assert_eq!(gtb_buf[4], 0); // Bidirectional
        assert_eq!(gtb_buf[5], 0); // First group
        assert_eq!(gtb_buf[6], 4); // Group count
        assert_eq!(gtb_buf[7], 2); // String idx
        assert_eq!(gtb_buf[8], 2); // MIDI 2.0
        assert_eq!(gtb_buf[9], 8); // Max SysEx8 streams

        let parsed_gtb = GroupTerminalBlock::parse(&gtb_buf).expect("parse gtb");
        assert_eq!(parsed_gtb, gtb);

        // 2. USB-MIDI 2.0 Header Descriptor
        let hdr = UsbMidi2HeaderDescriptor::new(13 + 7);
        let mut hdr_buf = [0_u8; 7];
        hdr.serialize(&mut hdr_buf);
        assert_eq!(hdr_buf[0], 7);
        assert_eq!(hdr_buf[1], 0x24);
        assert_eq!(hdr_buf[2], 0x01);
        assert_eq!(hdr_buf[3], 0x00);
        assert_eq!(hdr_buf[4], 0x02); // 0x0200
        assert_eq!(hdr_buf[5], 20);
        assert_eq!(hdr_buf[6], 0);

        let parsed_hdr = UsbMidi2HeaderDescriptor::parse(&hdr_buf).expect("parse header");
        assert_eq!(parsed_hdr, hdr);

        // 3. Zero-Copy Packet Packing and Unpacking
        let mut ump1 = crate::ump::Ump::new();
        ump1.data[0] = 0x20903C64; // 1 word
        let mut ump2 = crate::ump::Ump::new();
        ump2.data[0] = 0x40903C00; // 2 words
        ump2.data[1] = 0x12340000;

        let umps = [ump1, ump2];
        let mut raw_usb_buf = [0_u8; 64];
        let bytes_written =
            UsbMidiPacketPacker::pack_umps(&umps, &mut raw_usb_buf).expect("pack umps");
        assert_eq!(bytes_written, (1 + 2) * 4); // 12 bytes

        // Unpack sequentially
        let (unpacked1, len1) =
            UsbMidiPacketPacker::unpack_ump(&raw_usb_buf[..bytes_written]).expect("unpack 1");
        assert_eq!(len1, 4);
        assert_eq!(unpacked1.data[0], ump1.data[0]);

        let (unpacked2, len2) =
            UsbMidiPacketPacker::unpack_ump(&raw_usb_buf[len1..bytes_written]).expect("unpack 2");
        assert_eq!(len2, 8);
        assert_eq!(unpacked2.data[0], ump2.data[0]);
        assert_eq!(unpacked2.data[1], ump2.data[1]);
    }

    #[test]
    fn test_phase6_serial_bytestream_to_ump_and_interleaving() {
        use crate::transport::serial::{ByteStreamToUmp, UmpToByteStream};

        let mut bs_to_ump = ByteStreamToUmp::new(2);

        // 1. MIDI 1.0 Note On (3 bytes: 0x90, 0x3C, 0x64) with running status (0x3E, 0x50)
        let stream = [
            0x90, 0x3C, 0x64, // Note On Ch0, Note 60, Vel 100
            0xF8, // Real-time Timing Clock interleaved!
            0x3E, 0x50, // Running status Note On Ch0, Note 62, Vel 80
        ];

        for &b in &stream {
            bs_to_ump.process_byte(b);
        }

        assert_eq!(bs_to_ump.len(), 3);

        // First packet: Note On
        let u1 = bs_to_ump.pop_ump().expect("msg 1");
        assert_eq!(u1.message_type(), MessageType::Midi1ChannelVoice);
        assert_eq!(u1.group(), 2);
        assert_eq!((u1.data[0] >> 16) & 0xFF, 0x90);
        assert_eq!((u1.data[0] >> 8) & 0x7F, 60);
        assert_eq!(u1.data[0] & 0x7F, 100);

        // Second packet: Interleaved Timing Clock (MT=0x1)
        let u_rt = bs_to_ump.pop_ump().expect("rt msg");
        assert_eq!(u_rt.message_type(), MessageType::System);
        assert_eq!((u_rt.data[0] >> 16) & 0xFF, 0xF8);

        // Third packet: Running status Note On
        let u2 = bs_to_ump.pop_ump().expect("msg 2");
        assert_eq!(u2.message_type(), MessageType::Midi1ChannelVoice);
        assert_eq!(u2.group(), 2);
        assert_eq!((u2.data[0] >> 16) & 0xFF, 0x90);
        assert_eq!((u2.data[0] >> 8) & 0x7F, 62);
        assert_eq!(u2.data[0] & 0x7F, 80);

        // 2. SysEx7 chunking from serial bytes
        let sysex_stream = [0xF0, 0x01, 0x02, 0x03, 0xF7];
        for &b in &sysex_stream {
            bs_to_ump.process_byte(b);
        }
        assert_eq!(bs_to_ump.len(), 1);
        let u_sysex = bs_to_ump.pop_ump().expect("sysex msg");
        assert_eq!(u_sysex.message_type(), MessageType::SysEx7);
        assert_eq!((u_sysex.data[0] >> 20) & 0xF, 0); // Complete
        assert_eq!((u_sysex.data[0] >> 16) & 0xF, 3); // 3 bytes
        assert_eq!((u_sysex.data[0] >> 8) & 0x7F, 1);
        assert_eq!(u_sysex.data[0] & 0x7F, 2);
        assert_eq!((u_sysex.data[1] >> 24) & 0x7F, 3);

        // 3. UmpToByteStream: Down-conversion of MIDI 2.0 Note On & SysEx7
        let mut ump_to_bs = UmpToByteStream::new();
        ump_to_bs.enable_running_status = true;

        // Process Note On (MT=4, Grp 0, Ch 1, Note 60, Vel 0x8000 -> MIDI 1.0 Vel 64)
        let m2_note_on = UmpFactory::midi2_note_on(0, 1, 60, 0, 0x8000, 0);
        ump_to_bs.process_ump(&m2_note_on);

        // Reconstruct bytes
        let mut out_bytes = alloc::vec::Vec::new();
        while let Some(b) = ump_to_bs.pop_byte() {
            out_bytes.push(b);
        }
        assert_eq!(out_bytes, alloc::vec![0x91, 60, 64]);

        // Process SysEx7 UMP back to bytes
        ump_to_bs.process_ump(&u_sysex);
        let mut sysex_out = alloc::vec::Vec::new();
        while let Some(b) = ump_to_bs.pop_byte() {
            sysex_out.push(b);
        }
        assert_eq!(sysex_out, alloc::vec![0xF0, 0x01, 0x02, 0x03, 0xF7]);
    }

    #[test]
    fn test_phase6_dma_packet_queue_and_slice_leasing() {
        use crate::transport::queue::PacketQueue;
        use crate::transport::Transport;

        let mut queue = PacketQueue::<8>::new();
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.capacity(), 7);
        assert!(queue.is_empty());
        assert!(!queue.is_full());

        let mut ump = crate::ump::Ump::new();
        ump.data[0] = 0x20903C64;

        // Push and Pop
        assert!(queue.push(ump).is_ok());
        assert_eq!(queue.len(), 1);
        let popped = queue.pop().expect("pop ump");
        assert_eq!(popped, ump);
        assert_eq!(queue.len(), 0);

        // DMA Slice Leasing: grant_write and commit_write
        let (ptr_write, available_write) = queue.grant_write();
        assert_eq!(available_write, 7);
        unsafe {
            let slice = core::slice::from_raw_parts_mut(ptr_write, available_write);
            for (i, item) in slice.iter_mut().enumerate().take(3) {
                item.data[0] = 0x20900000 | (i as u32);
            }
        }
        queue.commit_write(3);
        assert_eq!(queue.len(), 3);

        // DMA Slice Leasing: grant_read and commit_read
        let (ptr_read, available_read) = queue.grant_read();
        assert_eq!(available_read, 3);
        unsafe {
            let slice = core::slice::from_raw_parts(ptr_read, available_read);
            assert_eq!(slice[0].data[0], 0x20900000);
            assert_eq!(slice[1].data[0], 0x20900001);
            assert_eq!(slice[2].data[0], 0x20900002);
        }
        queue.commit_read(3);
        assert_eq!(queue.len(), 0);

        // Transport trait implementation
        let batch = [ump; 4];
        assert!(queue.send(&batch).is_ok());
        assert_eq!(queue.len(), 4);
        let r = queue.recv().expect("recv ok").expect("some packet");
        assert_eq!(r, ump);
        assert_eq!(queue.len(), 3);
    }
}
