//! MIDI Clip File Specification (M2-116)
//!
//! Provides zero-allocation, `no_std` compatible parsing and generation of MIDI Clip Files (`.midiclip`).
//! MIDI Clip Files encapsulate Universal MIDI Packet (UMP) event streams with timestamps,
//! track chunks, and metadata headers ("SMF2").

use crate::ump::Ump;

/// The 4-byte ASCII magic identifier for MIDI Clip File headers ("SMF2").
pub const MIDI_CLIP_MAGIC: [u8; 4] = *b"SMF2";

/// The 4-byte ASCII identifier for track chunks ("MTrk" or "DTrk").
pub const MIDI_CLIP_TRACK_MAGIC: [u8; 4] = *b"MTrk";

/// Time format used in the MIDI Clip File header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipTimeFormat {
    /// Metric ticks per primary quarter note (PQN).
    TicksPerQuarterNote(u16),
    /// SMPTE timecode format: (fps_negative_format, ticks_per_frame).
    Smpte { fps: u8, subframes: u8 },
}

/// Header chunk of a MIDI Clip File (M2-116).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiClipHeader {
    /// Format type: 0 = Single track clip, 1 = Multi-track sequence.
    pub format: u16,
    /// Number of track chunks in the clip.
    pub num_tracks: u16,
    /// Time division configuration.
    pub time_format: ClipTimeFormat,
}

impl MidiClipHeader {
    /// Creates a new `MidiClipHeader` with specified tracks and ticks per quarter note.
    #[must_use]
    pub const fn new_ticks(format: u16, num_tracks: u16, ppqn: u16) -> Self {
        Self {
            format,
            num_tracks,
            time_format: ClipTimeFormat::TicksPerQuarterNote(ppqn),
        }
    }

    /// Parses a `MidiClipHeader` from a byte slice.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Option<(Self, usize)> {
        if bytes.len() < 14 {
            return None;
        }

        if bytes[0..4] != MIDI_CLIP_MAGIC {
            return None;
        }

        let chunk_len = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
        if chunk_len < 6 || bytes.len() < 8 + chunk_len {
            return None;
        }

        let format = u16::from_be_bytes([bytes[8], bytes[9]]);
        let num_tracks = u16::from_be_bytes([bytes[10], bytes[11]]);
        let time_div = u16::from_be_bytes([bytes[12], bytes[13]]);

        let time_format = if (time_div & 0x8000) == 0 {
            ClipTimeFormat::TicksPerQuarterNote(time_div)
        } else {
            let fps = ((!time_div >> 8) as u8) & 0x7F;
            let subframes = (time_div & 0xFF) as u8;
            ClipTimeFormat::Smpte { fps, subframes }
        };

        Some((
            Self {
                format,
                num_tracks,
                time_format,
            },
            8 + chunk_len,
        ))
    }

    /// Serializes this header into a 14-byte array.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 14] {
        let mut out = [0_u8; 14];
        out[0..4].copy_from_slice(&MIDI_CLIP_MAGIC);
        out[4..8].copy_from_slice(&6_u32.to_be_bytes());
        out[8..10].copy_from_slice(&self.format.to_be_bytes());
        out[10..12].copy_from_slice(&self.num_tracks.to_be_bytes());

        let div_raw = match self.time_format {
            ClipTimeFormat::TicksPerQuarterNote(ppqn) => ppqn & 0x7FFF,
            ClipTimeFormat::Smpte { fps, subframes } => {
                0x8000 | (((fps as u16) & 0x7F) << 8) | (subframes as u16)
            }
        };
        out[12..14].copy_from_slice(&div_raw.to_be_bytes());
        out
    }
}

/// A timed UMP event within a MIDI Clip track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiClipEvent {
    /// Delta timestamp in ticks or subframes.
    pub delta_time: u32,
    /// The Universal MIDI Packet payload.
    pub ump: Ump,
}

impl MidiClipEvent {
    /// Creates a new `MidiClipEvent`.
    #[must_use]
    #[inline]
    pub const fn new(delta_time: u32, ump: Ump) -> Self {
        Self { delta_time, ump }
    }
}

/// A zero-allocation iterator that parses a track payload into `MidiClipEvent` items.
#[derive(Debug, Clone)]
pub struct MidiClipTrackParser<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> MidiClipTrackParser<'a> {
    /// Creates a new `MidiClipTrackParser` from the track data slice.
    #[must_use]
    #[inline]
    pub const fn new(track_payload: &'a [u8]) -> Self {
        Self {
            bytes: track_payload,
            cursor: 0,
        }
    }

    /// Reads a variable-length quantity (VLQ) integer.
    fn read_vlq(&mut self) -> Option<u32> {
        let mut val: u32 = 0;
        let mut bytes_read = 0;

        while self.cursor < self.bytes.len() && bytes_read < 4 {
            let b = self.bytes[self.cursor];
            self.cursor += 1;
            bytes_read += 1;

            val = (val << 7) | ((b & 0x7F) as u32);
            if (b & 0x80) == 0 {
                return Some(val);
            }
        }
        None
    }
}

impl<'a> Iterator for MidiClipTrackParser<'a> {
    type Item = MidiClipEvent;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor >= self.bytes.len() {
            return None;
        }

        let delta_time = self.read_vlq()?;
        if self.cursor + 4 > self.bytes.len() {
            return None;
        }

        let w0 = u32::from_be_bytes([
            self.bytes[self.cursor],
            self.bytes[self.cursor + 1],
            self.bytes[self.cursor + 2],
            self.bytes[self.cursor + 3],
        ]);
        self.cursor += 4;

        let mt = (w0 >> 28) & 0xF;
        let word_count = match mt {
            0x0 | 0x1 | 0x2 | 0x6 | 0x7 => 1,
            0x3 | 0x4 | 0x8 | 0x9 | 0xA => 2,
            0xB | 0xC => 3,
            _ => 4,
        };

        let mut data = [w0, 0, 0, 0];
        #[allow(clippy::needless_range_loop)]
        for i in 1..word_count {
            if self.cursor + 4 > self.bytes.len() {
                return None;
            }
            let w = u32::from_be_bytes([
                self.bytes[self.cursor],
                self.bytes[self.cursor + 1],
                self.bytes[self.cursor + 2],
                self.bytes[self.cursor + 3],
            ]);
            self.cursor += 4;
            data[i] = w;
        }

        Some(MidiClipEvent {
            delta_time,
            ump: Ump { data },
        })
    }
}

/// Helper for building `.midiclip` binary files.
pub struct MidiClipBuilder;

impl MidiClipBuilder {
    /// Writes a variable-length quantity (VLQ) integer into `buf`.
    pub fn write_vlq(mut val: u32, buf: &mut [u8]) -> Option<usize> {
        if buf.is_empty() {
            return None;
        }

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

        if buf.len() < count {
            return None;
        }

        for i in 0..count {
            buf[i] = temp[count - 1 - i];
        }
        Some(count)
    }

    /// Serializes a track of `MidiClipEvent` items into `buf`.
    pub fn write_track(events: &[MidiClipEvent], buf: &mut [u8]) -> Option<usize> {
        if buf.len() < 8 {
            return None;
        }

        let mut cursor = 8;

        for ev in events {
            let vlq_len = Self::write_vlq(ev.delta_time, &mut buf[cursor..])?;
            cursor += vlq_len;

            let word_count = ev.ump.word_count();
            if cursor + (word_count * 4) > buf.len() {
                return None;
            }

            for i in 0..word_count {
                let w_bytes = ev.ump.data[i].to_be_bytes();
                buf[cursor..cursor + 4].copy_from_slice(&w_bytes);
                cursor += 4;
            }
        }

        let track_payload_len = (cursor - 8) as u32;
        buf[0..4].copy_from_slice(&MIDI_CLIP_TRACK_MAGIC);
        buf[4..8].copy_from_slice(&track_payload_len.to_be_bytes());

        Some(cursor)
    }
}
