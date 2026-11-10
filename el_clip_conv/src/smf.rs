//! SMF1 (Standard MIDI File 1.0) parser.
//!
//! Parses SMF1 headers (`MThd`) and track events (`MTrk`), including delta times (VLQ),
//! running status, MIDI channel voice messages, Meta-events, and SysEx.

/// Time division from an SMF1 header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmfDivision {
    /// Metric ticks per quarter note (PPQN).
    TicksPerQuarterNote(u16),
    /// SMPTE format: (fps_negative, ticks_per_frame).
    Smpte { fps: u8, subframes: u8 },
}

/// Parsed SMF1 header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmfHeader {
    /// Format: 0 (single track) or 1 (multi-track). Format 2 is sequential tracks.
    pub format: u16,
    /// Number of tracks.
    pub num_tracks: u16,
    /// Time division.
    pub division: SmfDivision,
}

/// An individual parsed SMF1 track event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmfEventKind {
    /// Channel Voice: Note Off (channel, note, velocity)
    NoteOff { channel: u8, note: u8, velocity: u8 },
    /// Channel Voice: Note On (channel, note, velocity)
    NoteOn { channel: u8, note: u8, velocity: u8 },
    /// Channel Voice: Poly Pressure (channel, note, pressure)
    PolyPressure { channel: u8, note: u8, pressure: u8 },
    /// Channel Voice: Control Change (channel, controller, value)
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
    /// Channel Voice: Program Change (channel, program)
    ProgramChange { channel: u8, program: u8 },
    /// Channel Voice: Channel Pressure (channel, pressure)
    ChannelPressure { channel: u8, pressure: u8 },
    /// Channel Voice: Pitch Bend (channel, 14-bit value)
    PitchBend { channel: u8, value: u16 },
    /// Meta Event: Tempo (microseconds per quarter note)
    MetaTempo { us_per_quarter: u32 },
    /// Meta Event: Track Name (text payload)
    MetaTrackName { text: Vec<u8> },
    /// Meta Event: End of Track
    MetaEndOfTrack,
    /// Other Meta Event (meta_type, payload)
    MetaOther { meta_type: u8, data: Vec<u8> },
    /// SysEx event (0xF0 or 0xF7, payload without 0xF0 if escaped, or full raw bytes)
    SysEx { data: Vec<u8> },
}

/// Timed SMF1 event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmfEvent {
    pub delta_time: u32,
    pub kind: SmfEventKind,
}

/// Parsed SMF1 file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmfFile {
    pub header: SmfHeader,
    pub tracks: Vec<Vec<SmfEvent>>,
}

/// Error types when parsing SMF1 files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmfParseError {
    UnexpectedEof,
    InvalidHeaderMagic,
    InvalidHeaderLength,
    InvalidTrackMagic,
    InvalidVlq,
    InvalidEventData,
}

impl SmfFile {
    /// Parses an SMF1 binary file from a byte slice.
    pub fn parse(bytes: &[u8]) -> Result<Self, SmfParseError> {
        // Parse MThd header
        if bytes.len() < 14 {
            return Err(SmfParseError::UnexpectedEof);
        }

        if &bytes[0..4] != b"MThd" {
            return Err(SmfParseError::InvalidHeaderMagic);
        }

        let header_len = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
        if header_len < 6 || bytes.len() < 8 + header_len {
            return Err(SmfParseError::InvalidHeaderLength);
        }

        let format = u16::from_be_bytes([bytes[8], bytes[9]]);
        let num_tracks = u16::from_be_bytes([bytes[10], bytes[11]]);
        let division_raw = u16::from_be_bytes([bytes[12], bytes[13]]);

        let division = if (division_raw & 0x8000) == 0 {
            SmfDivision::TicksPerQuarterNote(division_raw)
        } else {
            let fps = ((!division_raw >> 8) as u8) & 0x7F;
            let subframes = (division_raw & 0xFF) as u8;
            SmfDivision::Smpte { fps, subframes }
        };

        let header = SmfHeader {
            format,
            num_tracks,
            division,
        };

        let mut cursor = 8 + header_len;
        let mut tracks = Vec::with_capacity(num_tracks as usize);

        for _ in 0..num_tracks {
            if cursor + 8 > bytes.len() {
                break;
            }

            if &bytes[cursor..cursor + 4] != b"MTrk" {
                return Err(SmfParseError::InvalidTrackMagic);
            }

            let track_len = u32::from_be_bytes([
                bytes[cursor + 4],
                bytes[cursor + 5],
                bytes[cursor + 6],
                bytes[cursor + 7],
            ]) as usize;
            cursor += 8;

            let track_end = cursor
                .checked_add(track_len)
                .filter(|&end| end <= bytes.len())
                .ok_or(SmfParseError::UnexpectedEof)?;

            let track_data = &bytes[cursor..track_end];
            let events = parse_track(track_data)?;
            tracks.push(events);

            cursor = track_end;
        }

        Ok(SmfFile { header, tracks })
    }
}

/// Reads a Variable-Length Quantity (VLQ) from `bytes` starting at `cursor`.
/// Returns `(value, new_cursor)`.
fn read_vlq(bytes: &[u8], mut cursor: usize) -> Result<(u32, usize), SmfParseError> {
    let mut val: u32 = 0;
    let mut count = 0;

    while cursor < bytes.len() && count < 4 {
        let b = bytes[cursor];
        cursor += 1;
        count += 1;

        val = (val << 7) | ((b & 0x7F) as u32);
        if (b & 0x80) == 0 {
            return Ok((val, cursor));
        }
    }

    if count >= 4 {
        Err(SmfParseError::InvalidVlq)
    } else {
        Err(SmfParseError::UnexpectedEof)
    }
}

/// Parses the payload of an `MTrk` chunk into a vector of `SmfEvent`.
fn parse_track(bytes: &[u8]) -> Result<Vec<SmfEvent>, SmfParseError> {
    let mut events = Vec::new();
    let mut cursor = 0;
    let mut running_status: Option<u8> = None;

    while cursor < bytes.len() {
        let (delta_time, new_cursor) = read_vlq(bytes, cursor)?;
        cursor = new_cursor;

        if cursor >= bytes.len() {
            return Err(SmfParseError::UnexpectedEof);
        }

        let first_byte = bytes[cursor];

        if first_byte == 0xFF {
            // Meta Event
            cursor += 1;
            if cursor >= bytes.len() {
                return Err(SmfParseError::UnexpectedEof);
            }
            let meta_type = bytes[cursor];
            cursor += 1;

            let (len, new_cursor) = read_vlq(bytes, cursor)?;
            let len = len as usize;
            cursor = new_cursor;

            if cursor + len > bytes.len() {
                return Err(SmfParseError::UnexpectedEof);
            }
            let data = &bytes[cursor..cursor + len];
            cursor += len;

            // Running status is canceled by Meta events or SysEx in MIDI specs
            running_status = None;

            let kind = match meta_type {
                0x51 if len == 3 => {
                    let us = ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | (data[2] as u32);
                    SmfEventKind::MetaTempo { us_per_quarter: us }
                }
                0x03 => SmfEventKind::MetaTrackName {
                    text: data.to_vec(),
                },
                0x2F => SmfEventKind::MetaEndOfTrack,
                _ => SmfEventKind::MetaOther {
                    meta_type,
                    data: data.to_vec(),
                },
            };

            events.push(SmfEvent { delta_time, kind });
        } else if first_byte == 0xF0 || first_byte == 0xF7 {
            // SysEx Event
            cursor += 1;
            let (len, new_cursor) = read_vlq(bytes, cursor)?;
            let len = len as usize;
            cursor = new_cursor;

            if cursor + len > bytes.len() {
                return Err(SmfParseError::UnexpectedEof);
            }
            let data = bytes[cursor..cursor + len].to_vec();
            cursor += len;

            running_status = None;
            events.push(SmfEvent {
                delta_time,
                kind: SmfEventKind::SysEx { data },
            });
        } else {
            // Channel Voice or mode message
            let (status, is_running) = if (first_byte & 0x80) != 0 {
                cursor += 1;
                running_status = Some(first_byte);
                (first_byte, false)
            } else if let Some(rs) = running_status {
                (rs, true)
            } else {
                return Err(SmfParseError::InvalidEventData);
            };

            let _ = is_running;
            let event_type = status & 0xF0;
            let channel = status & 0x0F;

            match event_type {
                0x80 => {
                    // Note Off: 2 data bytes
                    if cursor + 2 > bytes.len() {
                        return Err(SmfParseError::UnexpectedEof);
                    }
                    let note = bytes[cursor] & 0x7F;
                    let velocity = bytes[cursor + 1] & 0x7F;
                    cursor += 2;
                    events.push(SmfEvent {
                        delta_time,
                        kind: SmfEventKind::NoteOff {
                            channel,
                            note,
                            velocity,
                        },
                    });
                }
                0x90 => {
                    // Note On: 2 data bytes (velocity 0 is Note Off)
                    if cursor + 2 > bytes.len() {
                        return Err(SmfParseError::UnexpectedEof);
                    }
                    let note = bytes[cursor] & 0x7F;
                    let velocity = bytes[cursor + 1] & 0x7F;
                    cursor += 2;
                    if velocity == 0 {
                        events.push(SmfEvent {
                            delta_time,
                            kind: SmfEventKind::NoteOff {
                                channel,
                                note,
                                velocity: 0,
                            },
                        });
                    } else {
                        events.push(SmfEvent {
                            delta_time,
                            kind: SmfEventKind::NoteOn {
                                channel,
                                note,
                                velocity,
                            },
                        });
                    }
                }
                0xA0 => {
                    // Poly Pressure: 2 data bytes
                    if cursor + 2 > bytes.len() {
                        return Err(SmfParseError::UnexpectedEof);
                    }
                    let note = bytes[cursor] & 0x7F;
                    let pressure = bytes[cursor + 1] & 0x7F;
                    cursor += 2;
                    events.push(SmfEvent {
                        delta_time,
                        kind: SmfEventKind::PolyPressure {
                            channel,
                            note,
                            pressure,
                        },
                    });
                }
                0xB0 => {
                    // Control Change: 2 data bytes
                    if cursor + 2 > bytes.len() {
                        return Err(SmfParseError::UnexpectedEof);
                    }
                    let controller = bytes[cursor] & 0x7F;
                    let value = bytes[cursor + 1] & 0x7F;
                    cursor += 2;
                    events.push(SmfEvent {
                        delta_time,
                        kind: SmfEventKind::ControlChange {
                            channel,
                            controller,
                            value,
                        },
                    });
                }
                0xC0 => {
                    // Program Change: 1 data byte
                    if cursor + 1 > bytes.len() {
                        return Err(SmfParseError::UnexpectedEof);
                    }
                    let program = bytes[cursor] & 0x7F;
                    cursor += 1;
                    events.push(SmfEvent {
                        delta_time,
                        kind: SmfEventKind::ProgramChange { channel, program },
                    });
                }
                0xD0 => {
                    // Channel Pressure: 1 data byte
                    if cursor + 1 > bytes.len() {
                        return Err(SmfParseError::UnexpectedEof);
                    }
                    let pressure = bytes[cursor] & 0x7F;
                    cursor += 1;
                    events.push(SmfEvent {
                        delta_time,
                        kind: SmfEventKind::ChannelPressure { channel, pressure },
                    });
                }
                0xE0 => {
                    // Pitch Bend: 2 data bytes (lsb, msb)
                    if cursor + 2 > bytes.len() {
                        return Err(SmfParseError::UnexpectedEof);
                    }
                    let lsb = (bytes[cursor] & 0x7F) as u16;
                    let msb = (bytes[cursor + 1] & 0x7F) as u16;
                    let value = lsb | (msb << 7);
                    cursor += 2;
                    events.push(SmfEvent {
                        delta_time,
                        kind: SmfEventKind::PitchBend { channel, value },
                    });
                }
                _ => {
                    return Err(SmfParseError::InvalidEventData);
                }
            }
        }
    }

    Ok(events)
}
