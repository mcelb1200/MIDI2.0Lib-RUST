//! Tests for native OS virtual endpoint transports (ALSA, CoreMIDI, Windows MIDI Services).

use el_bridge::native::alsa::AlsaVirtualEndpoint;
use el_bridge::native::coremidi::{
    CoreMidiVirtualEndpoint, MidiEventList, K_MIDI_PROTOCOL_1_0, K_MIDI_PROTOCOL_2_0,
};
use el_bridge::native::winmidi::{WinMidiPacket, WindowsMidiVirtualEndpoint};
use el_bridge::native::{NativeEndpointTransport, NativeVirtualEndpoint};
use el_bridge::translator::{ToMidi1Translator, ToUmpTranslator};
use el_core::builder::VoiceBuilder;

#[test]
fn test_alsa_endpoint_creation_and_loopback() {
    let mut ep = AlsaVirtualEndpoint::new_simulated("test_alsa_ep");
    assert_eq!(ep.name(), "test_alsa_ep");
    assert!(ep.is_active());
    assert!(ep.is_simulated());
    assert_eq!(ep.client_id(), 128);
    assert_eq!(ep.port_id(), 0);

    // Send MIDI 2.0 Note On UMP
    let note_on = VoiceBuilder::midi2_note_on(0, 1, 60, 0, 0x9000, 0);
    assert!(ep.send_ump(&note_on).is_ok());

    // Poll UMP back
    let received = ep.poll_ump().unwrap();
    assert!(received.is_some());
    let u = received.unwrap();
    assert_eq!(u.data[0], note_on.data[0]);
    assert_eq!(u.data[1], note_on.data[1]);

    // Empty poll
    assert_eq!(ep.poll_ump().unwrap(), None);
}

#[test]
fn test_alsa_event_encoding_and_decoding() {
    let ep = AlsaVirtualEndpoint::new_simulated("test_encoder");
    let cc_ump = VoiceBuilder::midi2_cc(2, 5, 10, 0x12345678);

    let ev = ep.encode_ump_event(&cc_ump);
    assert_eq!(ev.ump[0], cc_ump.data[0]);
    assert_eq!(ev.ump[1], cc_ump.data[1]);
    assert_eq!(ev.flags, el_bridge::native::alsa::SND_SEQ_EVENT_UMP);
    assert_eq!(
        ev.dest.client,
        el_bridge::native::alsa::SND_SEQ_ADDRESS_SUBSCRIBERS
    );

    let decoded = AlsaVirtualEndpoint::decode_ump_event(&ev);
    assert_eq!(decoded.data[0], cc_ump.data[0]);
    assert_eq!(decoded.data[1], cc_ump.data[1]);
}

#[test]
fn test_coremidi_event_list_and_packets() {
    let mut ep = CoreMidiVirtualEndpoint::new("test_coremidi").unwrap();
    assert_eq!(ep.name(), "test_coremidi");
    assert_eq!(ep.protocol(), K_MIDI_PROTOCOL_2_0);
    assert!(ep.is_active());

    // 64-bit Pitch Bend UMP
    let pb_ump = VoiceBuilder::midi2_pitch_bend(0, 0, 0x80000000);
    let event_list = MidiEventList::from_ump(&pb_ump, 1000);
    assert_eq!(event_list.protocol, K_MIDI_PROTOCOL_2_0);
    assert_eq!(event_list.packet[0].word_count, 2);
    assert_eq!(event_list.packet[0].words[0], pb_ump.data[0]);
    assert_eq!(event_list.packet[0].words[1], pb_ump.data[1]);

    // Receive injected event list (simulating external DAW/input)
    ep.receive_event_list(&event_list);
    let polled = ep.poll_ump().unwrap().unwrap();
    assert_eq!(polled.data[0], pb_ump.data[0]);
    assert_eq!(polled.data[1], pb_ump.data[1]);

    // Direct endpoint send: goes to tx_queue and does NOT loop back to poll_ump
    let note_off = VoiceBuilder::midi2_note_off(0, 0, 60, 0, 0x4000, 0);
    assert!(ep.send_ump(&note_off).is_ok());
    assert_eq!(ep.poll_ump().unwrap(), None); // No echo!

    let sent = ep.pop_sent_ump().unwrap();
    assert_eq!(sent.data[0], note_off.data[0]);
    assert_eq!(sent.data[1], note_off.data[1]);
}

#[test]
fn test_coremidi_protocol_versions() {
    let ep1 = CoreMidiVirtualEndpoint::with_protocol("m1_ep", K_MIDI_PROTOCOL_1_0).unwrap();
    assert_eq!(ep1.protocol(), K_MIDI_PROTOCOL_1_0);

    let ep2 = CoreMidiVirtualEndpoint::with_protocol("m2_ep", K_MIDI_PROTOCOL_2_0).unwrap();
    assert_eq!(ep2.protocol(), K_MIDI_PROTOCOL_2_0);
}

#[test]
fn test_windows_midi_services_packets() {
    let mut ep = WindowsMidiVirtualEndpoint::new("test_winmidi").unwrap();
    assert_eq!(ep.name(), "test_winmidi");
    assert!(ep.device_id().contains("test_winmidi"));
    assert!(ep.is_active());

    let rpn_ump = VoiceBuilder::midi2_per_note_rpn(0, 0, 60, 2, 0xAABBCCDD);
    let win_pkt = WinMidiPacket::from_ump(&rpn_ump, 50_000);
    assert_eq!(win_pkt.timestamp_100ns, 50_000);
    assert_eq!(win_pkt.word_count, 2);
    assert_eq!(win_pkt.words[0], rpn_ump.data[0]);
    assert_eq!(win_pkt.words[1], rpn_ump.data[1]);

    // Receive simulated external packet
    ep.receive_packet(win_pkt.clone());
    let polled = ep.poll_ump().unwrap().unwrap();
    assert_eq!(polled.data[0], rpn_ump.data[0]);
    assert_eq!(polled.data[1], rpn_ump.data[1]);

    // Direct endpoint send: goes to tx_queue and does NOT echo to poll_ump
    let nrpn_ump = VoiceBuilder::midi2_nrpn(0, 0, 3, 4, 0x11223344);
    assert!(ep.send_ump(&nrpn_ump).is_ok());
    assert_eq!(ep.poll_ump().unwrap(), None); // No echo!

    let sent = ep.pop_sent_ump().unwrap();
    assert_eq!(sent.data[0], nrpn_ump.data[0]);
    assert_eq!(sent.data[1], nrpn_ump.data[1]);
}

#[test]
fn test_native_virtual_endpoint_factory() {
    let mut ep = NativeVirtualEndpoint::create_default("factory_ep").unwrap();
    assert_eq!(ep.name(), "factory_ep");
    assert!(ep.is_active());

    let note_on = VoiceBuilder::midi1_note_on(0, 0, 64, 100);
    assert!(ep.send_ump(&note_on).is_ok());
    // On Linux ALSA (simulated loopback in headless test environments), sent packet loops back for local routing
    let polled = ep.poll_ump().unwrap().unwrap();
    assert_eq!(polled.data[0], note_on.data[0]);
}

#[test]
fn test_native_endpoint_with_translator_pipeline() {
    let mut native_ep = NativeVirtualEndpoint::create_default("pipeline_ep").unwrap();
    let mut to_ump = ToUmpTranslator::new(0);
    let mut to_midi1 = ToMidi1Translator::new();
    to_midi1.filter_group = Some(0);

    // Step 1: Input MIDI 1.0 Note On (0x90, 60, 100) -> ToUmpTranslator -> Native Virtual Endpoint
    to_ump.process_byte(0x90);
    to_ump.process_byte(60);
    to_ump.process_byte(100);
    assert!(to_ump.has_ump());
    let generated_ump = to_ump.pop_ump().unwrap();

    // Send into native endpoint
    assert!(native_ep.send_ump(&generated_ump).is_ok());

    // Step 2: Poll from native endpoint -> ToMidi1Translator -> MIDI 1.0 byte stream
    let polled_ump = native_ep.poll_ump().unwrap().unwrap();
    to_midi1.process_ump(&polled_ump);

    let mut out_bytes = Vec::new();
    while let Some(b) = to_midi1.pop_byte() {
        out_bytes.push(b);
    }

    assert_eq!(out_bytes, vec![0x90, 60, 100]);
}
