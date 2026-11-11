//! Translation logic between SMF1 events and MIDI Clip UMP events (`MidiClipEvent`).

use crate::smf::{SmfDivision, SmfEvent, SmfEventKind, SmfFile, SmfHeader};
use el_core::utils::{scale_down, scale_up};
use el_midi2::messages::{FlexDataChunker, FlexDataReceiver, FlexDataReceiverState, UmpFactory};
use el_midi2::midiclip::{
    ClipTimeFormat, MidiClipBuilder, MidiClipEvent, MidiClipHeader, MidiClipTrackParser,
    MIDI_CLIP_TRACK_MAGIC,
};
use el_midi2::ump::MessageType;
use el_midi2::utils::{
    FLEXDATA_COMMON, FLEXDATA_COMMON_TEMPO, FLEXDATA_METADATA_TRACK, FLEXDATA_TRACK_NAME,
};

/// Configuration options for SMF1 to `.midiclip` conversion.
#[derive(Debug, Clone, Copy, Default)]
pub struct ConverterConfig {
    /// Target UMP Group (0..15).
    pub group: u8,
    /// Up-convert Channel Voice messages to MIDI 2.0 (MT=0x4).
    pub midi2: bool,
}

/// Errors encountered during SMF1 <-> MIDI Clip conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConvertError {
    TrackWriteFailed,
    InvalidGroup(u8),
    InvalidClipFile,
}

/// Converts an in-memory `SmfFile` into serialized `.midiclip` binary bytes.
pub fn convert_smf_to_midiclip(
    smf: &SmfFile,
    config: &ConverterConfig,
) -> Result<Vec<u8>, ConvertError> {
    if config.group > 15 {
        return Err(ConvertError::InvalidGroup(config.group));
    }

    let time_format = match smf.header.division {
        SmfDivision::TicksPerQuarterNote(ppqn) => ClipTimeFormat::TicksPerQuarterNote(ppqn),
        SmfDivision::Smpte { fps, subframes } => ClipTimeFormat::Smpte { fps, subframes },
    };

    let header = MidiClipHeader {
        format: smf.header.format,
        num_tracks: smf.tracks.len() as u16,
        time_format,
    };

    let mut output = Vec::new();
    let header_bytes = header.to_bytes();
    output.extend_from_slice(&header_bytes);

    for track in &smf.tracks {
        let clip_events = convert_track(track, config);
        // Estimate buffer capacity for serialized track:
        // 8-byte chunk header + up to (4 bytes VLQ + 16 bytes UMP) per event.
        let needed_cap = 8 + clip_events.len() * 20 + 64;
        let mut track_buf = vec![0_u8; needed_cap];

        let written = MidiClipBuilder::write_track(&clip_events, &mut track_buf)
            .ok_or(ConvertError::TrackWriteFailed)?;

        output.extend_from_slice(&track_buf[..written]);
    }

    Ok(output)
}

/// Converts a single SMF1 track into a sequence of `MidiClipEvent` items.
pub fn convert_track(events: &[SmfEvent], config: &ConverterConfig) -> Vec<MidiClipEvent> {
    let mut clip_events = Vec::new();
    let group = config.group & 0xF;

    for ev in events {
        let delta = ev.delta_time;
        match &ev.kind {
            SmfEventKind::NoteOff {
                channel,
                note,
                velocity,
            } => {
                let ch = channel & 0xF;
                let ump = if config.midi2 {
                    let vel16 = scale_up(*velocity as u32, 7, 16) as u16;
                    UmpFactory::midi2_note_off(group, ch, *note, 0, vel16, 0)
                } else {
                    UmpFactory::midi1_note_off(group, ch, *note, *velocity)
                };
                clip_events.push(MidiClipEvent::new(delta, ump));
            }
            SmfEventKind::NoteOn {
                channel,
                note,
                velocity,
            } => {
                let ch = channel & 0xF;
                let ump = if config.midi2 {
                    let vel16 = scale_up(*velocity as u32, 7, 16) as u16;
                    UmpFactory::midi2_note_on(group, ch, *note, 0, vel16, 0)
                } else {
                    UmpFactory::midi1_note_on(group, ch, *note, *velocity)
                };
                clip_events.push(MidiClipEvent::new(delta, ump));
            }
            SmfEventKind::PolyPressure {
                channel,
                note,
                pressure,
            } => {
                let ch = channel & 0xF;
                let ump = if config.midi2 {
                    let press32 = scale_up(*pressure as u32, 7, 32);
                    UmpFactory::midi2_poly_pressure(group, ch, *note, press32)
                } else {
                    UmpFactory::midi1_poly_pressure(group, ch, *note, *pressure)
                };
                clip_events.push(MidiClipEvent::new(delta, ump));
            }
            SmfEventKind::ControlChange {
                channel,
                controller,
                value,
            } => {
                let ch = channel & 0xF;
                let ump = if config.midi2 {
                    let val32 = scale_up(*value as u32, 7, 32);
                    UmpFactory::midi2_control_change(group, ch, *controller, val32)
                } else {
                    UmpFactory::midi1_control_change(group, ch, *controller, *value)
                };
                clip_events.push(MidiClipEvent::new(delta, ump));
            }
            SmfEventKind::ProgramChange { channel, program } => {
                let ch = channel & 0xF;
                let ump = if config.midi2 {
                    UmpFactory::midi2_program_change(group, ch, *program, false, 0, 0)
                } else {
                    UmpFactory::midi1_program_change(group, ch, *program)
                };
                clip_events.push(MidiClipEvent::new(delta, ump));
            }
            SmfEventKind::ChannelPressure { channel, pressure } => {
                let ch = channel & 0xF;
                let ump = if config.midi2 {
                    let press32 = scale_up(*pressure as u32, 7, 32);
                    UmpFactory::midi2_channel_pressure(group, ch, press32)
                } else {
                    UmpFactory::midi1_channel_pressure(group, ch, *pressure)
                };
                clip_events.push(MidiClipEvent::new(delta, ump));
            }
            SmfEventKind::PitchBend { channel, value } => {
                let ch = channel & 0xF;
                let ump = if config.midi2 {
                    let bend32 = scale_up(*value as u32, 14, 32);
                    UmpFactory::midi2_pitch_bend(group, ch, bend32)
                } else {
                    UmpFactory::midi1_pitch_bend(group, ch, *value)
                };
                clip_events.push(MidiClipEvent::new(delta, ump));
            }
            SmfEventKind::MetaTempo { us_per_quarter } => {
                // Flex Data Common Tempo: 10-ns units per quarter note = microseconds * 100
                let num_10ns_pqn = us_per_quarter.saturating_mul(100);
                let tempo_ump = UmpFactory::flex_tempo(group, 0, 0, 0, num_10ns_pqn);
                clip_events.push(MidiClipEvent::new(delta, tempo_ump));
            }
            SmfEventKind::MetaTrackName { text } => {
                // Flex Data Track Metadata Name (MT=0xD, Bank 0x04, Status 0x00)
                // Chunked with FlexDataChunker across 12-byte packets.
                // First UMP packet takes the event's delta time; subsequent chunks carry delta time 0.
                let chunker = FlexDataChunker::new(
                    group,
                    0,
                    0, // addrs: channel
                    FLEXDATA_METADATA_TRACK,
                    FLEXDATA_TRACK_NAME,
                    text,
                );
                let mut is_first = true;
                for ump in chunker {
                    let d = if is_first {
                        is_first = false;
                        delta
                    } else {
                        0
                    };
                    clip_events.push(MidiClipEvent::new(d, ump));
                }
            }
            SmfEventKind::MetaEndOfTrack => {
                // End of track marker does not need explicit UMP event
            }
            SmfEventKind::MetaOther { .. } => {
                // Non-standard or auxiliary SMF meta events skipped in M2-116
            }
            SmfEventKind::SysEx { .. } => {
                // Ignore raw legacy SysEx in basic SMF1 translation or could be expanded to SysEx7 UMP
            }
        }
    }

    clip_events
}

/// Converts a serialized `.midiclip` binary file into an in-memory `SmfFile`.
pub fn convert_midiclip_to_smf(clip_bytes: &[u8]) -> Result<SmfFile, ConvertError> {
    let (header, hdr_len) =
        MidiClipHeader::parse(clip_bytes).ok_or(ConvertError::InvalidClipFile)?;

    let division = match header.time_format {
        ClipTimeFormat::TicksPerQuarterNote(ppqn) => SmfDivision::TicksPerQuarterNote(ppqn),
        ClipTimeFormat::Smpte { fps, subframes } => SmfDivision::Smpte { fps, subframes },
    };

    let smf_header = SmfHeader {
        format: header.format,
        num_tracks: header.num_tracks,
        division,
    };

    let mut tracks = Vec::new();
    let mut offset = hdr_len;

    while offset + 8 <= clip_bytes.len() {
        let chunk_id = &clip_bytes[offset..offset + 4];
        let chunk_len = u32::from_be_bytes([
            clip_bytes[offset + 4],
            clip_bytes[offset + 5],
            clip_bytes[offset + 6],
            clip_bytes[offset + 7],
        ]) as usize;
        offset += 8;

        if chunk_id == MIDI_CLIP_TRACK_MAGIC {
            let track_end = offset
                .checked_add(chunk_len)
                .filter(|&end| end <= clip_bytes.len())
                .ok_or(ConvertError::InvalidClipFile)?;
            let track_payload = &clip_bytes[offset..track_end];

            let smf_track = convert_clip_track_to_smf(track_payload);
            tracks.push(smf_track);
        }
        offset += chunk_len;
    }

    Ok(SmfFile {
        header: smf_header,
        tracks,
    })
}

/// Converts a single `.midiclip` track payload into a sequence of `SmfEvent` items.
pub fn convert_clip_track_to_smf(track_payload: &[u8]) -> Vec<SmfEvent> {
    let mut events = Vec::new();
    let parser = MidiClipTrackParser::new(track_payload);
    let mut flex_receiver = FlexDataReceiver::<256>::new();
    let mut pending_track_name_delta = 0_u32;

    for clip_ev in parser {
        let delta = clip_ev.delta_time;
        let ump = clip_ev.ump;

        match ump.message_type() {
            MessageType::Midi1ChannelVoice => {
                let status_byte = ((ump.data[0] >> 16) & 0xFF) as u8;
                let channel = status_byte & 0x0F;
                let opcode = status_byte & 0xF0;
                let note = ((ump.data[0] >> 8) & 0x7F) as u8;
                let val = (ump.data[0] & 0x7F) as u8;

                let kind = match opcode {
                    0x80 => SmfEventKind::NoteOff {
                        channel,
                        note,
                        velocity: val,
                    },
                    0x90 => {
                        if val == 0 {
                            SmfEventKind::NoteOff {
                                channel,
                                note,
                                velocity: 0,
                            }
                        } else {
                            SmfEventKind::NoteOn {
                                channel,
                                note,
                                velocity: val,
                            }
                        }
                    }
                    0xA0 => SmfEventKind::PolyPressure {
                        channel,
                        note,
                        pressure: val,
                    },
                    0xB0 => SmfEventKind::ControlChange {
                        channel,
                        controller: note,
                        value: val,
                    },
                    0xC0 => SmfEventKind::ProgramChange {
                        channel,
                        program: note,
                    },
                    0xD0 => SmfEventKind::ChannelPressure {
                        channel,
                        pressure: note,
                    },
                    0xE0 => {
                        let lsb = note as u16;
                        let msb = val as u16;
                        SmfEventKind::PitchBend {
                            channel,
                            value: lsb | (msb << 7),
                        }
                    }
                    _ => continue,
                };
                events.push(SmfEvent {
                    delta_time: delta,
                    kind,
                });
            }
            MessageType::Midi2ChannelVoice => {
                let status_byte = ((ump.data[0] >> 16) & 0xFF) as u8;
                let channel = status_byte & 0x0F;
                let opcode = status_byte & 0xF0;
                let note = ((ump.data[0] >> 8) & 0x7F) as u8;

                let kind = match opcode {
                    0x80 => {
                        let vel16 = (ump.data[1] >> 16) as u16;
                        let vel7 = scale_down(vel16 as u32, 16, 7) as u8;
                        SmfEventKind::NoteOff {
                            channel,
                            note,
                            velocity: vel7,
                        }
                    }
                    0x90 => {
                        let vel16 = (ump.data[1] >> 16) as u16;
                        let vel7 = scale_down(vel16 as u32, 16, 7) as u8;
                        if vel7 == 0 {
                            SmfEventKind::NoteOff {
                                channel,
                                note,
                                velocity: 0,
                            }
                        } else {
                            SmfEventKind::NoteOn {
                                channel,
                                note,
                                velocity: vel7,
                            }
                        }
                    }
                    0xA0 => {
                        let press32 = ump.data[1];
                        let press7 = scale_down(press32, 32, 7) as u8;
                        SmfEventKind::PolyPressure {
                            channel,
                            note,
                            pressure: press7,
                        }
                    }
                    0xB0 => {
                        let val32 = ump.data[1];
                        let val7 = scale_down(val32, 32, 7) as u8;
                        SmfEventKind::ControlChange {
                            channel,
                            controller: note,
                            value: val7,
                        }
                    }
                    0xC0 => {
                        let program = ((ump.data[1] >> 24) & 0x7F) as u8;
                        SmfEventKind::ProgramChange { channel, program }
                    }
                    0xD0 => {
                        let press32 = ump.data[1];
                        let press7 = scale_down(press32, 32, 7) as u8;
                        SmfEventKind::ChannelPressure {
                            channel,
                            pressure: press7,
                        }
                    }
                    0xE0 => {
                        let bend32 = ump.data[1];
                        let bend14 = scale_down(bend32, 32, 14) as u16;
                        SmfEventKind::PitchBend {
                            channel,
                            value: bend14,
                        }
                    }
                    _ => continue,
                };
                events.push(SmfEvent {
                    delta_time: delta,
                    kind,
                });
            }
            MessageType::FlexData => {
                let status_bank = ((ump.data[0] >> 8) & 0xFF) as u8;
                let status = (ump.data[0] & 0xFF) as u8;

                if status_bank == FLEXDATA_COMMON && status == FLEXDATA_COMMON_TEMPO {
                    // Flex Data Common Tempo: 10-ns units per quarter note
                    let tempo_10ns = ump.data[1];
                    let us_per_quarter = if tempo_10ns > 0 {
                        (tempo_10ns / 100).max(1)
                    } else {
                        500_000
                    };
                    events.push(SmfEvent {
                        delta_time: delta,
                        kind: SmfEventKind::MetaTempo { us_per_quarter },
                    });
                } else if status_bank == FLEXDATA_METADATA_TRACK && status == FLEXDATA_TRACK_NAME {
                    // Track Name multi-packet reassembly
                    if flex_receiver.state() == FlexDataReceiverState::Idle {
                        pending_track_name_delta = delta;
                    }
                    if let Ok(st) = flex_receiver.process_ump(&ump) {
                        if st == FlexDataReceiverState::Complete {
                            let text = flex_receiver.text().to_vec();
                            events.push(SmfEvent {
                                delta_time: pending_track_name_delta,
                                kind: SmfEventKind::MetaTrackName { text },
                            });
                            flex_receiver.reset();
                        }
                    }
                }
            }
            MessageType::SysEx7 => {
                // SysEx7 conversion if desired (extract 7-bit bytes from MT=0x3)
                let status_nibble = (ump.data[0] >> 20) & 0xF;
                let num_bytes = ((ump.data[0] >> 16) & 0xF) as usize;
                if status_nibble == 0x0 {
                    // Complete SysEx7 packet
                    let mut sysex_bytes = Vec::with_capacity(num_bytes);
                    let raw_bytes = [
                        ((ump.data[0] >> 8) & 0xFF) as u8,
                        (ump.data[0] & 0xFF) as u8,
                        ((ump.data[1] >> 24) & 0xFF) as u8,
                        ((ump.data[1] >> 16) & 0xFF) as u8,
                        ((ump.data[1] >> 8) & 0xFF) as u8,
                        (ump.data[1] & 0xFF) as u8,
                    ];
                    for b in raw_bytes.iter().take(num_bytes.min(6)) {
                        sysex_bytes.push(*b);
                    }
                    events.push(SmfEvent {
                        delta_time: delta,
                        kind: SmfEventKind::SysEx { data: sysex_bytes },
                    });
                }
            }
            _ => {}
        }
    }

    events.push(SmfEvent {
        delta_time: 0,
        kind: SmfEventKind::MetaEndOfTrack,
    });

    events
}
