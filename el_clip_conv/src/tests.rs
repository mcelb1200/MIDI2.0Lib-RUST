#[cfg(test)]
mod unit_tests {
    use crate::converter::{convert_smf_to_midiclip, ConverterConfig};
    use crate::smf::SmfFile;
    use el_midi2::midiclip::{ClipTimeFormat, MultiTrackSequencer};
    use el_midi2::ump::MessageType;

    /// Helper to write SMF1 variable-length quantity into a vector.
    fn push_vlq(vec: &mut Vec<u8>, mut val: u32) {
        let mut temp = [0_u8; 4];
        let mut count = 0;
        temp[count] = (val & 0x7F) as u8;
        count += 1;
        val >>= 7;

        while val > 0 && count < 4 {
            temp[count] = ((val & 0x7F) as u8) | 0x80;
            count += 1;
            val >>= 7;
        }

        for i in (0..count).rev() {
            vec.push(temp[i]);
        }
    }

    /// Helper to create a valid SMF Format 0 file with tempo, track name, and notes.
    fn create_test_smf0(ppqn: u16) -> Vec<u8> {
        let mut bytes = Vec::new();

        // MThd chunk
        bytes.extend_from_slice(b"MThd");
        bytes.extend_from_slice(&6_u32.to_be_bytes());
        bytes.extend_from_slice(&0_u16.to_be_bytes()); // Format 0
        bytes.extend_from_slice(&1_u16.to_be_bytes()); // 1 Track
        bytes.extend_from_slice(&ppqn.to_be_bytes()); // Division PPQN

        // Track data
        let mut trk_data = Vec::new();

        // Track name meta-event: delta 0, FF 03 09 "TestTrack"
        push_vlq(&mut trk_data, 0);
        trk_data.push(0xFF);
        trk_data.push(0x03);
        let track_name = b"TestTrack";
        push_vlq(&mut trk_data, track_name.len() as u32);
        trk_data.extend_from_slice(track_name);

        // Tempo meta-event: delta 0, FF 03 03 07 A1 20 (500,000 us = 120 BPM)
        push_vlq(&mut trk_data, 0);
        trk_data.push(0xFF);
        trk_data.push(0x51);
        trk_data.push(0x03);
        trk_data.extend_from_slice(&500_000_u32.to_be_bytes()[1..4]);

        // Note On: delta 0, 90 3C 64 (Channel 0, Note 60, Velocity 100)
        push_vlq(&mut trk_data, 0);
        trk_data.extend_from_slice(&[0x90, 60, 100]);

        // Running status Note On: delta ppqn (480), running status -> 40 50 (Channel 0, Note 64, Velocity 80)
        push_vlq(&mut trk_data, ppqn as u32);
        trk_data.extend_from_slice(&[64, 80]);

        // Note Off: delta ppqn (480), 80 3C 40 (Channel 0, Note 60, Velocity 64)
        push_vlq(&mut trk_data, ppqn as u32);
        trk_data.extend_from_slice(&[0x80, 60, 64]);

        // Note Off via Note On velocity 0: delta 0, 90 40 00 (Channel 0, Note 64, Velocity 0)
        push_vlq(&mut trk_data, 0);
        trk_data.extend_from_slice(&[0x90, 64, 0]);

        // End of track: delta 0, FF 2F 00
        push_vlq(&mut trk_data, 0);
        trk_data.extend_from_slice(&[0xFF, 0x2F, 0x00]);

        // MTrk chunk
        bytes.extend_from_slice(b"MTrk");
        bytes.extend_from_slice(&(trk_data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&trk_data);

        bytes
    }

    /// Helper to create a valid SMF Format 1 file with 2 tracks.
    fn create_test_smf1(ppqn: u16) -> Vec<u8> {
        let mut bytes = Vec::new();

        // MThd chunk: format 1, 2 tracks
        bytes.extend_from_slice(b"MThd");
        bytes.extend_from_slice(&6_u32.to_be_bytes());
        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.extend_from_slice(&2_u16.to_be_bytes());
        bytes.extend_from_slice(&ppqn.to_be_bytes());

        // Track 1 (Conductor track: tempo and track name)
        let mut trk1_data = Vec::new();
        push_vlq(&mut trk1_data, 0);
        trk1_data.push(0xFF);
        trk1_data.push(0x03);
        let name1 = b"Conductor";
        push_vlq(&mut trk1_data, name1.len() as u32);
        trk1_data.extend_from_slice(name1);

        // Tempo 1,000,000 us (60 BPM)
        push_vlq(&mut trk1_data, 0);
        trk1_data.push(0xFF);
        trk1_data.push(0x51);
        trk1_data.push(0x03);
        trk1_data.extend_from_slice(&1_000_000_u32.to_be_bytes()[1..4]);

        push_vlq(&mut trk1_data, 0);
        trk1_data.extend_from_slice(&[0xFF, 0x2F, 0x00]);

        bytes.extend_from_slice(b"MTrk");
        bytes.extend_from_slice(&(trk1_data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&trk1_data);

        // Track 2 (Notes track)
        let mut trk2_data = Vec::new();
        push_vlq(&mut trk2_data, 0);
        trk2_data.push(0xFF);
        trk2_data.push(0x03);
        let name2 = b"Notes";
        push_vlq(&mut trk2_data, name2.len() as u32);
        trk2_data.extend_from_slice(name2);

        // Note On at delta 240
        push_vlq(&mut trk2_data, 240);
        trk2_data.extend_from_slice(&[0x90, 72, 110]);

        // Note Off at delta 240 (tick 480)
        push_vlq(&mut trk2_data, 240);
        trk2_data.extend_from_slice(&[0x80, 72, 0]);

        push_vlq(&mut trk2_data, 0);
        trk2_data.extend_from_slice(&[0xFF, 0x2F, 0x00]);

        bytes.extend_from_slice(b"MTrk");
        bytes.extend_from_slice(&(trk2_data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&trk2_data);

        bytes
    }

    #[test]
    fn test_smf0_to_midiclip_midi1() {
        let raw_smf = create_test_smf0(480);
        let smf = SmfFile::parse(&raw_smf).expect("parse smf0");
        assert_eq!(smf.header.format, 0);
        assert_eq!(smf.tracks.len(), 1);

        let config = ConverterConfig {
            group: 0,
            midi2: false,
        };

        let clip_bytes = convert_smf_to_midiclip(&smf, &config).expect("convert smf0");

        // Verify output by parsing with MultiTrackSequencer
        let sequencer =
            MultiTrackSequencer::<4>::from_clip_file(&clip_bytes).expect("sequencer from clip");
        assert_eq!(sequencer.track_count(), 1);
        assert_eq!(
            sequencer.time_format(),
            ClipTimeFormat::TicksPerQuarterNote(480)
        );

        let events: Vec<_> = sequencer.collect();
        // Expected events:
        // 1. Flex Data Track Name ("TestTrack")
        // 2. Flex Data Tempo (500,000 us * 100 = 50,000,000 10-ns units = 120 BPM)
        // 3. Note On (60)
        // 4. Note On (64) via running status at tick 480
        // 5. Note Off (60) at tick 960
        // 6. Note Off (64) at tick 960
        assert_eq!(events.len(), 6);

        // Check track name UMP (Flex Data MT=0xD)
        assert_eq!(events[0].ump.message_type(), MessageType::FlexData);
        assert_eq!(events[0].tick_timestamp, 0);

        // Check tempo UMP
        assert_eq!(events[1].ump.message_type(), MessageType::FlexData);
        assert_eq!(events[1].tick_timestamp, 0);
        assert_eq!(events[1].ump.data[1], 50_000_000);

        // Check MIDI 1.0 Note On (MT=0x2)
        assert_eq!(events[2].ump.message_type(), MessageType::Midi1ChannelVoice);
        assert_eq!(events[2].tick_timestamp, 0);
        // Word 0: MT=2, Group=0, Status=0x90, Note=60 (0x3C), Vel=100 (0x64)
        assert_eq!(events[2].ump.data[0], 0x20903C64);

        // Check running status Note On (64) at tick 480
        assert_eq!(events[3].ump.message_type(), MessageType::Midi1ChannelVoice);
        assert_eq!(events[3].tick_timestamp, 480);
        assert_eq!(events[3].ump.data[0], 0x20904050);

        // Check Note Off (60) at tick 960
        assert_eq!(events[4].ump.message_type(), MessageType::Midi1ChannelVoice);
        assert_eq!(events[4].tick_timestamp, 960);
        assert_eq!(events[4].ump.data[0], 0x20803C40);

        // Check Note Off (64, vel 0) at tick 960
        assert_eq!(events[5].ump.message_type(), MessageType::Midi1ChannelVoice);
        assert_eq!(events[5].tick_timestamp, 960);
        assert_eq!(events[5].ump.data[0], 0x20804000);
    }

    #[test]
    fn test_smf1_to_midiclip_midi2_scaling() {
        let raw_smf = create_test_smf1(480);
        let smf = SmfFile::parse(&raw_smf).expect("parse smf1");
        assert_eq!(smf.header.format, 1);
        assert_eq!(smf.tracks.len(), 2);

        let config = ConverterConfig {
            group: 2,
            midi2: true,
        };

        let clip_bytes = convert_smf_to_midiclip(&smf, &config).expect("convert smf1");

        // Verify output with MultiTrackSequencer
        let sequencer =
            MultiTrackSequencer::<4>::from_clip_file(&clip_bytes).expect("sequencer from clip");
        assert_eq!(sequencer.track_count(), 2);
        assert_eq!(
            sequencer.time_format(),
            ClipTimeFormat::TicksPerQuarterNote(480)
        );

        let events: Vec<_> = sequencer.collect();
        assert_eq!(events.len(), 5);

        // Event 0: Track 0 Track Name ("Conductor") at tick 0
        assert_eq!(events[0].track_idx, 0);
        assert_eq!(events[0].tick_timestamp, 0);

        // Event 1: Track 0 Tempo (1,000,000 us -> 100,000,000 10-ns -> 60 BPM) at tick 0
        assert_eq!(events[1].track_idx, 0);
        assert_eq!(events[1].tick_timestamp, 0);
        assert_eq!(events[1].ump.data[1], 100_000_000);

        // Event 2: Track 1 Track Name ("Notes") at tick 0
        assert_eq!(events[2].track_idx, 1);
        assert_eq!(events[2].tick_timestamp, 0);

        // Event 3: Track 1 Note On (72, vel 110) at tick 240
        // At 60 BPM and 480 PPQN, 240 ticks = 500,000 us
        assert_eq!(events[3].track_idx, 1);
        assert_eq!(events[3].tick_timestamp, 240);
        assert_eq!(events[3].us_timestamp, 500_000);
        assert_eq!(events[3].ump.message_type(), MessageType::Midi2ChannelVoice);
        assert_eq!(events[3].ump.group(), 2);
        // Note 72 is 0x48. Status is Note On (0x9). Group is 2.
        assert_eq!((events[3].ump.data[0] >> 24) & 0xFF, 0x42);
        assert_eq!((events[3].ump.data[0] >> 16) & 0xFF, 0x90);
        assert_eq!((events[3].ump.data[0] >> 8) & 0xFF, 0x48);
        // 7-bit velocity 110 scaled to 16-bit: (110 << 9) | (110 << 2) | (110 >> 5) = 56320 | 440 | 3 = 56763 (0xDDD3)
        let expected_vel16 = el_core::utils::scale_up(110, 7, 16);
        assert_eq!((events[3].ump.data[1] >> 16) as u16, expected_vel16 as u16);

        // Event 4: Track 1 Note Off (72, vel 0) at tick 480
        // At 60 BPM, 480 ticks = 1,000,000 us
        assert_eq!(events[4].track_idx, 1);
        assert_eq!(events[4].tick_timestamp, 480);
        assert_eq!(events[4].us_timestamp, 1_000_000);
        assert_eq!(events[4].ump.message_type(), MessageType::Midi2ChannelVoice);
        assert_eq!(events[4].ump.group(), 2);
    }

    #[test]
    fn test_all_smf_controllers_and_bend() {
        let mut trk_data = Vec::new();
        // CC #7 value 100
        push_vlq(&mut trk_data, 10);
        trk_data.extend_from_slice(&[0xB0, 7, 100]);
        // Poly Pressure note 60 value 90
        push_vlq(&mut trk_data, 10);
        trk_data.extend_from_slice(&[0xA0, 60, 90]);
        // Program Change 42
        push_vlq(&mut trk_data, 10);
        trk_data.extend_from_slice(&[0xC0, 42]);
        // Channel Pressure 80
        push_vlq(&mut trk_data, 10);
        trk_data.extend_from_slice(&[0xD0, 80]);
        // Pitch Bend 8192 (center: lsb 0, msb 64)
        push_vlq(&mut trk_data, 10);
        trk_data.extend_from_slice(&[0xE0, 0x00, 0x40]);
        // End of track
        push_vlq(&mut trk_data, 0);
        trk_data.extend_from_slice(&[0xFF, 0x2F, 0x00]);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"MThd");
        bytes.extend_from_slice(&6_u32.to_be_bytes());
        bytes.extend_from_slice(&0_u16.to_be_bytes());
        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.extend_from_slice(&480_u16.to_be_bytes());

        bytes.extend_from_slice(b"MTrk");
        bytes.extend_from_slice(&(trk_data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&trk_data);

        let smf = SmfFile::parse(&bytes).expect("parse smf");

        // Convert with MIDI 2.0
        let config = ConverterConfig {
            group: 1,
            midi2: true,
        };
        let clip_bytes = convert_smf_to_midiclip(&smf, &config).expect("convert");
        let seq =
            MultiTrackSequencer::<2>::from_clip_file(&clip_bytes).expect("sequencer from clip");
        let events: Vec<_> = seq.collect();
        assert_eq!(events.len(), 5);

        // CC: MT=0x4, CC index 7
        assert_eq!(events[0].ump.message_type(), MessageType::Midi2ChannelVoice);
        // Poly Pressure: MT=0x4
        assert_eq!(events[1].ump.message_type(), MessageType::Midi2ChannelVoice);
        // Program Change: MT=0x4
        assert_eq!(events[2].ump.message_type(), MessageType::Midi2ChannelVoice);
        // Channel Pressure: MT=0x4
        assert_eq!(events[3].ump.message_type(), MessageType::Midi2ChannelVoice);
        // Pitch Bend center: MT=0x4, scaled 14-bit to 32-bit (8192 -> 0x80000000)
        assert_eq!(events[4].ump.message_type(), MessageType::Midi2ChannelVoice);
        assert_eq!(events[4].ump.data[1], 0x8000_0000);
    }
}
