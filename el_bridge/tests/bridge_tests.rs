use el_bridge::queues::create_spsc_queue;
use el_bridge::translator::{ToMidi1Translator, ToUmpTranslator};
use el_core::builder::VoiceBuilder;

#[test]
fn test_14bit_cc_aggregation() {
    let mut translator = ToUmpTranslator::new(0);

    // CC 1 MSB = 64 (0x40)
    translator.process_byte(0xB0);
    translator.process_byte(1);
    translator.process_byte(64);
    assert!(translator.has_ump());
    let _ump1 = translator.pop_ump().unwrap();

    // CC 33 LSB = 32 (0x20)
    translator.process_byte(33); // using running status
    translator.process_byte(32);
    assert!(translator.has_ump());
    let ump2 = translator.pop_ump().unwrap();

    // The combined 14-bit value is (64 << 7) | 32 = 8224
    let expected_val = el_core::utils::scale_up(8224, 14, 32);
    assert_eq!(ump2.data[0], 0x40B00100);
    assert_eq!(ump2.data[1], expected_val);
}

#[test]
fn test_14bit_cc_splitting() {
    let mut translator = ToMidi1Translator::new();
    translator.filter_group = Some(0);

    // 14-bit value: 8224 (MSB=64, LSB=32) scaled up to 32-bit
    let val32 = el_core::utils::scale_up(8224, 14, 32);
    let ump = VoiceBuilder::midi2_cc(0, 0, 1, val32);

    translator.process_ump(&ump);

    let mut bytes = Vec::new();
    while let Some(b) = translator.pop_byte() {
        bytes.push(b);
    }

    // Should generate: CC 1 MSB 64, then CC 33 LSB 32
    assert_eq!(bytes, vec![0xB0, 1, 64, 0xB0, 33, 32]);
}

#[test]
fn test_nrpn_aggregation() {
    let mut translator = ToUmpTranslator::new(1);

    // NRPN sequence on Channel 2 (0xB2):
    // CC 99 = 0x12 (MSB)
    // CC 98 = 0x34 (LSB)
    // CC 6 = 0x56 (Data MSB)
    // CC 38 = 0x78 (Data LSB)
    translator.process_byte(0xB2);
    translator.process_byte(99);
    translator.process_byte(0x12);
    assert!(!translator.has_ump());

    translator.process_byte(98);
    translator.process_byte(0x34);
    assert!(!translator.has_ump());

    translator.process_byte(6);
    translator.process_byte(0x56);
    assert!(translator.has_ump());
    let _ump1 = translator.pop_ump().unwrap();

    translator.process_byte(38);
    translator.process_byte(0x78);
    assert!(translator.has_ump());
    let ump2 = translator.pop_ump().unwrap();

    // MT=0x4, Status=0x3 (NRPN), Group 1, Channel 2, Bank=0x12, Index=0x34
    // 0x41321234
    assert_eq!(ump2.data[0], 0x41321234);
    let val14 = (0x56_u32 << 7) | 0x78_u32;
    let val32 = el_core::utils::scale_up(val14, 14, 32);
    assert_eq!(ump2.data[1], val32);
}

#[test]
fn test_sysex_chunking_and_reassembly() {
    let mut to_ump = ToUmpTranslator::new(0);
    let mut to_midi1 = ToMidi1Translator::new();

    // Feed MIDI 1.0 SysEx: F0 01 02 03 04 05 06 07 08 09 F7
    let original = vec![
        0xF0, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0xF7,
    ];
    for &b in &original {
        to_ump.process_byte(b);
    }

    let mut umps = Vec::new();
    while let Some(ump) = to_ump.pop_ump() {
        umps.push(ump);
    }

    // 9 payload bytes chunked into 6 bytes (Start) + 3 bytes (End) = 2 UMPs
    assert_eq!(umps.len(), 2);
    // Packet 1: MT=3, format=1 (Start), len=6
    assert_eq!((umps[0].data[0] >> 20) & 0xF, 1);
    assert_eq!((umps[0].data[0] >> 16) & 0xF, 6);
    // Packet 2: MT=3, format=3 (End), len=3
    assert_eq!((umps[1].data[0] >> 20) & 0xF, 3);
    assert_eq!((umps[1].data[0] >> 16) & 0xF, 3);

    // Pass UMPs back into to_midi1
    for ump in &umps {
        to_midi1.process_ump(ump);
    }

    let mut roundtrip = Vec::new();
    while let Some(b) = to_midi1.pop_byte() {
        roundtrip.push(b);
    }

    assert_eq!(roundtrip, original);
}

#[test]
fn test_realtime_interleaving_and_running_status() {
    let mut translator = ToUmpTranslator::new(0);

    // Send Note On status 0x90, note 60, then interleave Timing Clock (0xF8), then velocity 100
    translator.process_byte(0x90);
    translator.process_byte(60);
    translator.process_byte(0xF8); // interleaved clock
    assert!(translator.has_ump());
    let clock_ump = translator.pop_ump().unwrap();
    assert_eq!(clock_ump.data[0], 0x10F80000);

    translator.process_byte(100); // velocity completes NoteOn
    assert!(translator.has_ump());
    let note_ump = translator.pop_ump().unwrap();
    assert_eq!(note_ump.data[0], 0x40903C00);

    // Running status: next note without status byte
    translator.process_byte(62);
    translator.process_byte(90);
    assert!(translator.has_ump());
    let note2_ump = translator.pop_ump().unwrap();
    assert_eq!(note2_ump.data[0], 0x40903E00);
}

#[test]
fn test_lockfree_spsc_queue() {
    let (mut prod, mut cons) = create_spsc_queue::<u32>(16);
    assert!(cons.is_empty());
    assert!(!prod.is_full());

    for i in 0..10 {
        assert!(prod.try_push(i).is_ok());
    }
    assert_eq!(cons.len(), 10);

    for i in 0..10 {
        assert_eq!(cons.try_pop(), Some(i));
    }
    assert!(cons.is_empty());
}
