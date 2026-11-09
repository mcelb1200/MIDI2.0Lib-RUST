//! MIDI Clip File Specification (M2-116)
//!
//! Provides zero-allocation, `no_std` compatible parsing and generation of MIDI Clip Files (`.midiclip`).
//! MIDI Clip Files encapsulate Universal MIDI Packet (UMP) event streams with timestamps,
//! track chunks, and metadata headers ("SMF2").

use crate::ump::{MessageType, Ump};
use crate::utils::{FLEXDATA_COMMON, FLEXDATA_COMMON_TEMPO};

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

/// A scheduled UMP event emitted by the `MultiTrackSequencer`, containing timing and source metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledClipEvent {
    /// Absolute timestamp in cumulative ticks.
    pub tick_timestamp: u64,
    /// Absolute timestamp in microseconds (based on current tempo map).
    pub us_timestamp: u64,
    /// Originating track index (0-based).
    pub track_idx: usize,
    /// Universal MIDI Packet payload.
    pub ump: Ump,
}

/// Per-track runtime state within the multi-track sequencer.
#[derive(Debug, Clone)]
struct TrackState<'a> {
    parser: MidiClipTrackParser<'a>,
    active: bool,
    current_tick: u64,
    next_event: Option<MidiClipEvent>,
}

/// Error types returned by `MultiTrackSequencer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequencerError {
    /// Maximum track capacity exceeded.
    TrackLimitExceeded,
    /// Invalid header format or missing track chunks.
    InvalidClipFile,
}

/// Zero-allocation simultaneous multi-track sequencer and playback engine for MIDI Clip files (M2-116).
///
/// Merges multiple concurrent `MTrk` chunks into a single chronologically ordered UMP stream,
/// performing delta-time to microsecond/tick conversion with dynamic `FlexData` tempo map tracking.
#[derive(Debug, Clone)]
pub struct MultiTrackSequencer<'a, const MAX_TRACKS: usize = 16> {
    tracks: [Option<TrackState<'a>>; MAX_TRACKS],
    track_count: usize,
    time_format: ClipTimeFormat,
    current_tick: u64,
    current_us: u64,
    /// Active tempo in units of 10-nanoseconds per primary quarter note (M2-104 Flex Data tempo).
    /// Default is 50,000,000 (500,000 us / 120 BPM).
    tempo_10ns_pqn: u32,
    /// Tick position of the last tempo change.
    tempo_anchor_tick: u64,
    /// Microsecond position of the last tempo change.
    tempo_anchor_us: u64,
    /// Track data references for seeking/resetting.
    initial_track_payloads: [Option<&'a [u8]>; MAX_TRACKS],
}

impl<'a, const MAX_TRACKS: usize> MultiTrackSequencer<'a, MAX_TRACKS> {
    /// Default tempo in 10-nanosecond units per primary quarter note (120 BPM = 500,000 us = 50,000,000 units of 10ns).
    pub const DEFAULT_TEMPO_10NS_PQN: u32 = 50_000_000;

    /// Creates a new empty `MultiTrackSequencer` with the specified time format.
    #[must_use]
    pub const fn new(time_format: ClipTimeFormat) -> Self {
        assert!(MAX_TRACKS > 0, "MAX_TRACKS must be greater than zero");
        Self {
            tracks: [const { None }; MAX_TRACKS],
            track_count: 0,
            time_format,
            current_tick: 0,
            current_us: 0,
            tempo_10ns_pqn: Self::DEFAULT_TEMPO_10NS_PQN,
            tempo_anchor_tick: 0,
            tempo_anchor_us: 0,
            initial_track_payloads: [const { None }; MAX_TRACKS],
        }
    }

    /// Creates a sequencer pre-configured from a parsed `MidiClipHeader`.
    #[must_use]
    pub const fn from_header(header: &MidiClipHeader) -> Self {
        Self::new(header.time_format)
    }

    /// Initializes a sequencer by parsing a full `.midiclip` binary file and registering all track chunks.
    pub fn from_clip_file(file_bytes: &'a [u8]) -> Result<Self, SequencerError> {
        let (header, hdr_len) =
            MidiClipHeader::parse(file_bytes).ok_or(SequencerError::InvalidClipFile)?;
        let mut sequencer = Self::from_header(&header);

        let mut offset = hdr_len;
        while offset + 8 <= file_bytes.len() {
            let chunk_id = &file_bytes[offset..offset + 4];
            let chunk_len = u32::from_be_bytes([
                file_bytes[offset + 4],
                file_bytes[offset + 5],
                file_bytes[offset + 6],
                file_bytes[offset + 7],
            ]) as usize;
            offset += 8;

            if chunk_id == MIDI_CLIP_TRACK_MAGIC {
                let track_end = offset
                    .checked_add(chunk_len)
                    .filter(|&end| end <= file_bytes.len())
                    .ok_or(SequencerError::InvalidClipFile)?;
                let track_payload = &file_bytes[offset..track_end];
                sequencer.add_track(track_payload)?;
            }
            offset += chunk_len;
        }

        Ok(sequencer)
    }

    /// Adds a track payload slice to the sequencer.
    pub fn add_track(&mut self, track_payload: &'a [u8]) -> Result<usize, SequencerError> {
        if self.track_count >= MAX_TRACKS {
            return Err(SequencerError::TrackLimitExceeded);
        }

        let track_idx = self.track_count;
        let mut parser = MidiClipTrackParser::new(track_payload);
        let first_ev = parser.next();
        let active = first_ev.is_some();
        let initial_tick = first_ev.as_ref().map_or(0, |ev| ev.delta_time as u64);

        self.tracks[track_idx] = Some(TrackState {
            parser,
            active,
            current_tick: initial_tick,
            next_event: first_ev,
        });
        self.initial_track_payloads[track_idx] = Some(track_payload);
        self.track_count += 1;

        Ok(track_idx)
    }

    /// Returns the number of registered tracks.
    #[must_use]
    #[inline]
    pub const fn track_count(&self) -> usize {
        self.track_count
    }

    /// Returns the current time format.
    #[must_use]
    #[inline]
    pub const fn time_format(&self) -> ClipTimeFormat {
        self.time_format
    }

    /// Returns the active tempo in 10-nanosecond units per primary quarter note.
    #[must_use]
    #[inline]
    pub const fn tempo_10ns_pqn(&self) -> u32 {
        self.tempo_10ns_pqn
    }

    /// Returns the active tempo in beats per minute (BPM).
    #[must_use]
    pub fn bpm(&self) -> f32 {
        if self.tempo_10ns_pqn == 0 {
            return 120.0;
        }
        // 1 minute = 60 seconds = 60,000,000 microseconds = 6,000,000,000 10-nanoseconds.
        // BPM = 6,000,000,000 / tempo_10ns_pqn
        (6_000_000_000.0 / (self.tempo_10ns_pqn as f64)) as f32
    }

    /// Sets the playback tempo directly in 10-nanosecond units per primary quarter note.
    pub fn set_tempo_10ns_pqn(&mut self, tempo_10ns: u32) {
        if tempo_10ns == 0 {
            return;
        }
        // Anchor the current time before switching tempo
        self.tempo_anchor_us = self.current_us;
        self.tempo_anchor_tick = self.current_tick;
        self.tempo_10ns_pqn = tempo_10ns;
    }

    /// Converts a tick count delta since anchor tick into elapsed microseconds.
    #[must_use]
    pub fn delta_ticks_to_us(&self, delta_ticks: u64) -> u64 {
        match self.time_format {
            ClipTimeFormat::TicksPerQuarterNote(ppqn) => {
                if ppqn == 0 {
                    return 0;
                }
                // MPQN = tempo_10ns_pqn / 100
                // delta_us = (delta_ticks * MPQN) / ppqn
                // To maintain high precision: (delta_ticks * tempo_10ns_pqn) / (ppqn * 100)
                let num = (delta_ticks as u128) * (self.tempo_10ns_pqn as u128);
                let den = (ppqn as u128) * 100;
                (num / den) as u64
            }
            ClipTimeFormat::Smpte { fps, subframes } => {
                let total_fps_sub = (fps as u128) * (subframes as u128);
                if total_fps_sub == 0 {
                    return 0;
                }
                // delta_us = (delta_ticks * 1,000,000) / (fps * subframes)
                let num = (delta_ticks as u128) * 1_000_000;
                (num / total_fps_sub) as u64
            }
        }
    }

    /// Computes the absolute microsecond timestamp for an absolute tick value.
    #[must_use]
    pub fn tick_to_us(&self, tick: u64) -> u64 {
        if tick <= self.tempo_anchor_tick {
            self.tempo_anchor_us
        } else {
            let delta = tick - self.tempo_anchor_tick;
            self.tempo_anchor_us + self.delta_ticks_to_us(delta)
        }
    }

    /// Returns the current playback position in ticks.
    #[must_use]
    #[inline]
    pub const fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Returns the current playback position in microseconds.
    #[must_use]
    #[inline]
    pub const fn current_us(&self) -> u64 {
        self.current_us
    }

    /// Returns `true` if all registered tracks have exhausted their event streams.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        for i in 0..self.track_count {
            if let Some(trk) = &self.tracks[i] {
                if trk.active && trk.next_event.is_some() {
                    return false;
                }
            }
        }
        true
    }

    /// Resets all track parsers to the beginning and restores the initial transport position.
    pub fn reset(&mut self) {
        self.current_tick = 0;
        self.current_us = 0;
        self.tempo_10ns_pqn = Self::DEFAULT_TEMPO_10NS_PQN;
        self.tempo_anchor_tick = 0;
        self.tempo_anchor_us = 0;

        for i in 0..self.track_count {
            if let Some(payload) = self.initial_track_payloads[i] {
                let mut parser = MidiClipTrackParser::new(payload);
                let first_ev = parser.next();
                let active = first_ev.is_some();
                let initial_tick = first_ev.as_ref().map_or(0, |ev| ev.delta_time as u64);

                self.tracks[i] = Some(TrackState {
                    parser,
                    active,
                    current_tick: initial_tick,
                    next_event: first_ev,
                });
            }
        }
    }

    /// Fast-forwards / seeks the playback engine to the target tick position,
    /// evaluating tempo changes along the way without emitting external events.
    pub fn seek_to_tick(&mut self, target_tick: u64) {
        if target_tick < self.current_tick {
            self.reset();
        }

        while let Some(ev) = self.peek_next_event_info() {
            if ev.0 > target_tick {
                break;
            }
            let _ = self.next();
        }

        if self.current_tick < target_tick {
            let delta = target_tick - self.current_tick;
            self.current_us += self.delta_ticks_to_us(delta);
            self.current_tick = target_tick;
        }
    }

    /// Peeks at the next chronological event's `(tick_timestamp, track_idx)` without advancing.
    fn peek_next_event_info(&self) -> Option<(u64, usize)> {
        let mut earliest_tick = u64::MAX;
        let mut best_track = None;

        for i in 0..self.track_count {
            if let Some(trk) = &self.tracks[i] {
                if trk.active && trk.next_event.is_some() && trk.current_tick < earliest_tick {
                    earliest_tick = trk.current_tick;
                    best_track = Some(i);
                }
            }
        }

        best_track.map(|idx| (earliest_tick, idx))
    }

    /// Advances real-time playback by `elapsed_us` microseconds and writes any due UMP messages
    /// into `out_buffer`. Returns the number of UMP messages written.
    pub fn tick(&mut self, elapsed_us: u64, out_buffer: &mut [Ump]) -> usize {
        let target_us = self.current_us + elapsed_us;
        let mut written = 0;

        while written < out_buffer.len() {
            if let Some((next_tick, _)) = self.peek_next_event_info() {
                let next_us = self.tick_to_us(next_tick);
                if next_us <= target_us {
                    if let Some(ev) = self.next() {
                        out_buffer[written] = ev.ump;
                        written += 1;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        self.current_us = target_us;
        written
    }
}

impl<'a, const MAX_TRACKS: usize> Iterator for MultiTrackSequencer<'a, MAX_TRACKS> {
    type Item = ScheduledClipEvent;

    fn next(&mut self) -> Option<Self::Item> {
        // Find track with the lowest current_tick
        let mut earliest_tick = u64::MAX;
        let mut best_track_idx = None;

        for i in 0..self.track_count {
            if let Some(trk) = &self.tracks[i] {
                if trk.active && trk.next_event.is_some() && trk.current_tick < earliest_tick {
                    earliest_tick = trk.current_tick;
                    best_track_idx = Some(i);
                }
            }
        }

        let track_idx = best_track_idx?;
        let trk = self.tracks[track_idx].as_mut()?;
        let event = trk.next_event.take()?;
        let event_tick = trk.current_tick;

        // Advance track parser to fetch following event
        if let Some(next_ev) = trk.parser.next() {
            trk.current_tick += next_ev.delta_time as u64;
            trk.next_event = Some(next_ev);
        } else {
            trk.active = false;
        }

        let event_us = self.tick_to_us(event_tick);

        // Advance transport clocks
        self.current_tick = event_tick;
        self.current_us = event_us;

        // Check for FlexData Tempo update
        // Format: MT=0xD, Bank=0x00 (FLEXDATA_COMMON), Status=0x00 (FLEXDATA_COMMON_TEMPO)
        let ump = event.ump;
        if ump.message_type() == MessageType::FlexData {
            let status_bank = ((ump.data[0] >> 8) & 0xFF) as u8;
            let status = (ump.data[0] & 0xFF) as u8;
            if status_bank == FLEXDATA_COMMON && status == FLEXDATA_COMMON_TEMPO {
                let new_tempo = ump.data[1];
                if new_tempo > 0 {
                    self.tempo_anchor_tick = event_tick;
                    self.tempo_anchor_us = event_us;
                    self.tempo_10ns_pqn = new_tempo;
                }
            }
        }

        Some(ScheduledClipEvent {
            tick_timestamp: event_tick,
            us_timestamp: event_us,
            track_idx,
            ump,
        })
    }
}
