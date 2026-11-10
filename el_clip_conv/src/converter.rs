//! Translation logic from SMF1 events to MIDI Clip UMP events (`MidiClipEvent`).

use crate::smf::{SmfDivision, SmfEvent, SmfEventKind, SmfFile};
use el_core::utils::scale_up;
use el_midi2::messages::{FlexDataChunker, UmpFactory};
use el_midi2::midiclip::{ClipTimeFormat, MidiClipBuilder, MidiClipEvent, MidiClipHeader};
use el_midi2::utils::{FLEXDATA_METADATA_TRACK, FLEXDATA_TRACK_NAME};

/// Configuration options for SMF1 to `.midiclip` conversion.
#[derive(Debug, Clone, Copy, Default)]
pub struct ConverterConfig {
    /// Target UMP Group (0..15).
    pub group: u8,
    /// Up-convert Channel Voice messages to MIDI 2.0 (MT=0x4).
    pub midi2: bool,
}

/// Errors encountered during SMF1 conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConvertError {
    TrackWriteFailed,
    InvalidGroup(u8),
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
