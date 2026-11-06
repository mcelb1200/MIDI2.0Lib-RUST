//! Serial UART / DIN / TRS Byte-Stream Transport & Bi-Directional Adapter
//!
//! Implements bidirectional state machines for converting between legacy MIDI 1.0
//! byte streams (UART/DIN/TRS) and Universal MIDI Packets (UMP).
//!
//! Features:
//! - Running status reconstruction and handling
//! - Real-time byte interleaving without corrupting active multi-byte messages or SysEx
//! - SysEx7 chunking into MT=0x3 packets and reassembly back to byte streams
//! - Down-conversion of MIDI 2.0 (MT=0x4) channel voice messages to MIDI 1.0 byte streams
//!   including 16-bit to 7-bit velocity scaling, 32-bit CC/pitch-bend scaling, and RPN/NRPN generation.

use crate::ump::Ump;
use crate::utils::*;

/// Bounded capacity for the `ByteStreamToUmp` output ringbuffer.
const BSTOUMP_BUFFER_SIZE: usize = 8;
/// Bounded capacity for the `UmpToByteStream` output ringbuffer.
const UMPTOBS_BUFFER_SIZE: usize = 32;

/// State machine converting incoming raw MIDI 1.0 serial bytes into Universal MIDI Packets (UMP).
#[derive(Debug)]
pub struct ByteStreamToUmp {
    /// Default UMP group (0..15) applied to generated UMPs.
    pub default_group: u8,
    d0: u8,
    d1: u8,
    sysex_state: u8, // 0 = idle, 1 = start, 2 = continue
    sysex_pos: u8,
    sysex_buf: [u8; 6],
    buffer: [Ump; BSTOUMP_BUFFER_SIZE],
    read_idx: usize,
    write_idx: usize,
    count: usize,
}

impl Default for ByteStreamToUmp {
    fn default() -> Self {
        Self::new(0)
    }
}

impl ByteStreamToUmp {
    /// Creates a new `ByteStreamToUmp` parser with the specified default group.
    #[must_use]
    pub const fn new(default_group: u8) -> Self {
        Self {
            default_group: default_group & 0x0F,
            d0: 0,
            d1: 255,
            sysex_state: 0,
            sysex_pos: 0,
            sysex_buf: [0; 6],
            buffer: [Ump { data: [0; 4] }; BSTOUMP_BUFFER_SIZE],
            read_idx: 0,
            write_idx: 0,
            count: 0,
        }
    }

    /// Resets the internal state and empties the packet buffer.
    pub fn reset(&mut self) {
        self.d0 = 0;
        self.d1 = 255;
        self.sysex_state = 0;
        self.sysex_pos = 0;
        self.sysex_buf = [0; 6];
        self.read_idx = 0;
        self.write_idx = 0;
        self.count = 0;
    }

    /// Returns `true` if a UMP packet is ready to be read.
    #[must_use]
    #[inline]
    pub fn available(&self) -> bool {
        self.count > 0
    }

    /// Returns the number of UMP packets available in the output buffer.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns `true` if the output buffer is empty.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Reads an available UMP packet from the internal buffer.
    #[must_use]
    pub fn pop_ump(&mut self) -> Option<Ump> {
        if self.count == 0 {
            return None;
        }
        let ump = self.buffer[self.read_idx];
        self.read_idx = (self.read_idx + 1) % BSTOUMP_BUFFER_SIZE;
        self.count -= 1;
        Some(ump)
    }

    #[inline]
    fn push_ump(&mut self, ump: Ump) {
        if self.count < BSTOUMP_BUFFER_SIZE {
            self.buffer[self.write_idx] = ump;
            self.write_idx = (self.write_idx + 1) % BSTOUMP_BUFFER_SIZE;
            self.count += 1;
        }
    }

    #[inline]
    fn emit_voice_or_system(&mut self, b0: u8, b1: u8, b2: u8) {
        let status = b0 & 0xF0;
        let mut ump = Ump::new();
        if b0 >= TIMING_CODE {
            // System Common / Real-Time (MT=0x1)
            ump.data[0] = ((1_u32) << 28)
                | ((self.default_group as u32 & 0xF) << 24)
                | ((b0 as u32) << 16)
                | ((b1 as u32) << 8)
                | (b2 as u32);
            self.push_ump(ump);
        } else if (NOTE_OFF..=PITCH_BEND).contains(&status) {
            // MIDI 1.0 Channel Voice (MT=0x2)
            ump.data[0] = ((2_u32) << 28)
                | ((self.default_group as u32 & 0xF) << 24)
                | ((b0 as u32) << 16)
                | ((b1 as u32) << 8)
                | (b2 as u32);
            self.push_ump(ump);
        }
    }

    #[inline]
    fn dump_sysex7(&mut self, format: u8) {
        if self.sysex_pos == 0 && format != 0 {
            return;
        }
        let len = if self.sysex_pos > 6 {
            6
        } else {
            self.sysex_pos
        };
        let mut ump = Ump::new();
        ump.data[0] = ((3_u32) << 28)
            | ((self.default_group as u32 & 0xF) << 24)
            | ((format as u32 & 0xF) << 20)
            | ((len as u32 & 0xF) << 16)
            | ((self.sysex_buf[0] as u32) << 8)
            | (self.sysex_buf[1] as u32);
        ump.data[1] = ((self.sysex_buf[2] as u32) << 24)
            | ((self.sysex_buf[3] as u32) << 16)
            | ((self.sysex_buf[4] as u32) << 8)
            | (self.sysex_buf[5] as u32);

        self.push_ump(ump);
        self.sysex_buf = [0; 6];
        self.sysex_pos = 0;
    }

    /// Feeds a single byte from a serial stream into the parser.
    pub fn process_byte(&mut self, byte: u8) {
        // Real-Time messages (0xF8..=0xFF) can be interleaved anywhere
        if byte >= TIMINGCLOCK {
            self.emit_voice_or_system(byte, 0, 0);
            return;
        }

        // Status Byte received (bit 7 set)
        if (byte & 0x80) != 0 {
            if byte == SYSEX_START {
                // If a SysEx was already in progress, abort it
                self.sysex_state = 1; // Start
                self.sysex_pos = 0;
                self.sysex_buf = [0; 6];
                self.d0 = 0;
                self.d1 = 255;
            } else if byte == SYSEX_STOP {
                if self.sysex_state > 0 {
                    let fmt = if self.sysex_state == 1 { 0 } else { 3 }; // Complete (0) or End (3)
                    self.dump_sysex7(fmt);
                }
                self.sysex_state = 0;
                self.sysex_pos = 0;
                self.d0 = 0;
                self.d1 = 255;
            } else {
                // Normal status byte
                if self.sysex_state > 0 {
                    // Non-realtime status byte cancels active SysEx
                    self.sysex_state = 0;
                    self.sysex_pos = 0;
                }
                self.d0 = byte;
                self.d1 = 255;
            }
            return;
        }

        // Data Byte received (bit 7 clear)
        if self.sysex_state >= 1 {
            // Accumulate into SysEx buffer
            if self.sysex_pos >= 6 {
                let fmt = self.sysex_state; // 1 = Start, 2 = Continue
                self.dump_sysex7(fmt);
                self.sysex_state = 2; // Next chunk is Continue
            }
            if (self.sysex_pos as usize) < self.sysex_buf.len() {
                self.sysex_buf[self.sysex_pos as usize] = byte & 0x7F;
                self.sysex_pos += 1;
            }
        } else if self.d0 != 0 {
            let status_nibble = self.d0 & 0xF0;
            if status_nibble == PROGRAM_CHANGE
                || status_nibble == CHANNEL_PRESSURE
                || self.d0 == TIMING_CODE
                || self.d0 == SONG_SELECT
            {
                // Single data byte messages
                self.emit_voice_or_system(self.d0, byte & 0x7F, 0);
                // Keep running status for channel voice messages, reset for system common
                if self.d0 >= TIMING_CODE {
                    self.d0 = 0;
                }
            } else if self.d0 < SYSEX_START || self.d0 == SPP {
                // 2-data byte messages
                if self.d1 == 255 {
                    self.d1 = byte & 0x7F;
                } else {
                    self.emit_voice_or_system(self.d0, self.d1, byte & 0x7F);
                    self.d1 = 255;
                    // Reset d0 if System Common (SPP)
                    if self.d0 >= TIMING_CODE {
                        self.d0 = 0;
                    }
                }
            }
        }
    }
}

/// State machine converting Universal MIDI Packets (UMP) into raw MIDI 1.0 serial byte streams.
#[derive(Debug)]
pub struct UmpToByteStream {
    /// If Some(group), only UMPs matching this group are converted. None processes all groups.
    pub filter_group: Option<u8>,
    /// Whether to generate running status bytes (reducing MIDI 1.0 wire bandwidth).
    pub enable_running_status: bool,
    last_running_status: u8,
    last_rpn_msb: [u8; 16],
    last_rpn_lsb: [u8; 16],
    last_nrpn_msb: [u8; 16],
    last_nrpn_lsb: [u8; 16],
    buffer: [u8; UMPTOBS_BUFFER_SIZE],
    read_idx: usize,
    write_idx: usize,
    count: usize,
}

impl Default for UmpToByteStream {
    fn default() -> Self {
        Self::new()
    }
}

impl UmpToByteStream {
    /// Creates a new `UmpToByteStream` converter.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            filter_group: None,
            enable_running_status: false,
            last_running_status: 255,
            last_rpn_msb: [255; 16],
            last_rpn_lsb: [255; 16],
            last_nrpn_msb: [255; 16],
            last_nrpn_lsb: [255; 16],
            buffer: [0; UMPTOBS_BUFFER_SIZE],
            read_idx: 0,
            write_idx: 0,
            count: 0,
        }
    }

    /// Resets the converter state and flushes the output byte queue.
    pub fn reset(&mut self) {
        self.last_running_status = 255;
        self.last_rpn_msb = [255; 16];
        self.last_rpn_lsb = [255; 16];
        self.last_nrpn_msb = [255; 16];
        self.last_nrpn_lsb = [255; 16];
        self.read_idx = 0;
        self.write_idx = 0;
        self.count = 0;
    }

    /// Returns `true` if serial bytes are available to be read.
    #[must_use]
    #[inline]
    pub fn available(&self) -> bool {
        self.count > 0
    }

    /// Returns the number of bytes queued for transmission.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns `true` if no bytes are queued.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Reads an available byte from the internal FIFO.
    #[must_use]
    pub fn pop_byte(&mut self) -> Option<u8> {
        if self.count == 0 {
            return None;
        }
        let b = self.buffer[self.read_idx];
        self.read_idx = (self.read_idx + 1) % UMPTOBS_BUFFER_SIZE;
        self.count -= 1;
        Some(b)
    }

    #[inline]
    fn push_byte(&mut self, b: u8) {
        if self.count < UMPTOBS_BUFFER_SIZE {
            self.buffer[self.write_idx] = b;
            self.write_idx = (self.write_idx + 1) % UMPTOBS_BUFFER_SIZE;
            self.count += 1;
        }
    }

    #[inline]
    fn write_status_and_data(&mut self, status_ch: u8, data1: u8, data2: Option<u8>) {
        if !self.enable_running_status || self.last_running_status != status_ch {
            self.push_byte(status_ch);
            self.last_running_status = status_ch;
        }
        self.push_byte(data1 & 0x7F);
        if let Some(d2) = data2 {
            self.push_byte(d2 & 0x7F);
        }
    }

    /// Processes a single Universal MIDI Packet (UMP) and produces serialized MIDI 1.0 bytes.
    pub fn process_ump(&mut self, ump: &Ump) {
        let w0 = ump.data[0];
        let mt = (w0 >> 28) & 0xF;
        let group = ((w0 >> 24) & 0xF) as u8;

        if let Some(filt) = self.filter_group {
            if group != filt {
                return;
            }
        }

        match mt {
            // MT=0x1: System Real-Time and System Common
            0x1 => {
                let sys_byte = ((w0 >> 16) & 0xFF) as u8;
                if sys_byte == TIMING_CODE || sys_byte == SONG_SELECT {
                    self.push_byte(sys_byte);
                    self.push_byte(((w0 >> 8) & 0x7F) as u8);
                } else if sys_byte == SPP {
                    self.push_byte(sys_byte);
                    self.push_byte(((w0 >> 8) & 0x7F) as u8);
                    self.push_byte((w0 & 0x7F) as u8);
                } else if sys_byte >= TIMINGCLOCK || sys_byte == TUNEREQUEST {
                    self.push_byte(sys_byte);
                }
            }

            // MT=0x2: MIDI 1.0 Channel Voice
            0x2 => {
                let status_ch = ((w0 >> 16) & 0xFF) as u8;
                let status_nibble = status_ch & 0xF0;
                let d1 = ((w0 >> 8) & 0x7F) as u8;
                let d2 = (w0 & 0x7F) as u8;

                if status_nibble == PROGRAM_CHANGE || status_nibble == CHANNEL_PRESSURE {
                    self.write_status_and_data(status_ch, d1, None);
                } else {
                    self.write_status_and_data(status_ch, d1, Some(d2));
                }
            }

            // MT=0x3: SysEx7 Data
            0x3 => {
                let format = ((w0 >> 20) & 0xF) as u8;
                let num_bytes = ((w0 >> 16) & 0xF) as usize;
                let w1 = ump.data[1];

                // Emit F0 on Complete (0) or Start (1)
                if format == 0 || format == 1 {
                    self.push_byte(SYSEX_START);
                }

                let payload = [
                    ((w0 >> 8) & 0x7F) as u8,
                    (w0 & 0x7F) as u8,
                    ((w1 >> 24) & 0x7F) as u8,
                    ((w1 >> 16) & 0x7F) as u8,
                    ((w1 >> 8) & 0x7F) as u8,
                    (w1 & 0x7F) as u8,
                ];

                for b in payload.into_iter().take(num_bytes.min(6)) {
                    self.push_byte(b);
                }

                // Emit F7 on Complete (0) or End (3)
                if format == 0 || format == 3 {
                    self.push_byte(SYSEX_STOP);
                }
            }

            // MT=0x4: MIDI 2.0 Channel Voice -> Down-convert to MIDI 1.0
            0x4 => {
                let status_nibble = ((w0 >> 16) & 0xF0) as u8;
                let channel = ((w0 >> 16) & 0x0F) as u8;
                let ch_idx = (channel & 0x0F) as usize;
                let val1 = ((w0 >> 8) & 0x7F) as u8;
                let val2 = (w0 & 0x7F) as u8;
                let w1 = ump.data[1];
                let status_ch = status_nibble | channel;

                match status_nibble {
                    NOTE_OFF => {
                        let vel = (scale_down(w1 >> 16, 16, 7) & 0x7F) as u8;
                        self.write_status_and_data(status_ch, val1, Some(vel));
                    }
                    NOTE_ON => {
                        let mut vel = (scale_down(w1 >> 16, 16, 7) & 0x7F) as u8;
                        if vel == 0 {
                            vel = 1; // Prevent velocity 0 from acting as NoteOff
                        }
                        self.write_status_and_data(status_ch, val1, Some(vel));
                    }
                    KEY_PRESSURE => {
                        let press = (scale_down(w1, 32, 7) & 0x7F) as u8;
                        self.write_status_and_data(status_ch, val1, Some(press));
                    }
                    CC => {
                        let val = (scale_down(w1, 32, 7) & 0x7F) as u8;
                        self.write_status_and_data(status_ch, val1, Some(val));
                    }
                    CHANNEL_PRESSURE => {
                        let val = (scale_down(w1, 32, 7) & 0x7F) as u8;
                        self.write_status_and_data(status_ch, val, None);
                    }
                    PITCH_BEND => {
                        // MIDI 1.0 pitch bend: 14-bit unsigned (0..16383), center 8192
                        let pb14 = scale_down(w1, 32, 14) as u16;
                        let lsb = (pb14 & 0x7F) as u8;
                        let msb = ((pb14 >> 7) & 0x7F) as u8;
                        self.write_status_and_data(status_ch, lsb, Some(msb));
                    }
                    PROGRAM_CHANGE => {
                        let bank_valid = (w0 & 0x1) != 0;
                        if bank_valid {
                            let bank_msb = ((w1 >> 8) & 0x7F) as u8;
                            let bank_lsb = (w1 & 0x7F) as u8;
                            let cc_ch = CC | channel;
                            self.write_status_and_data(cc_ch, 0, Some(bank_msb));
                            self.write_status_and_data(cc_ch, 32, Some(bank_lsb));
                        }
                        let prog = ((w1 >> 24) & 0x7F) as u8;
                        self.write_status_and_data(PROGRAM_CHANGE | channel, prog, None);
                    }
                    RPN => {
                        let cc_ch = CC | channel;
                        if self.last_rpn_msb[ch_idx] != val1 || self.last_rpn_lsb[ch_idx] != val2 {
                            self.write_status_and_data(cc_ch, 101, Some(val1));
                            self.write_status_and_data(cc_ch, 100, Some(val2));
                            self.last_rpn_msb[ch_idx] = val1;
                            self.last_rpn_lsb[ch_idx] = val2;
                            self.last_nrpn_msb[ch_idx] = 255;
                            self.last_nrpn_lsb[ch_idx] = 255;
                        }
                        let val14 = scale_down(w1, 32, 14) as u16;
                        self.write_status_and_data(cc_ch, 6, Some(((val14 >> 7) & 0x7F) as u8));
                        self.write_status_and_data(cc_ch, 38, Some((val14 & 0x7F) as u8));
                    }
                    NRPN => {
                        let cc_ch = CC | channel;
                        if self.last_nrpn_msb[ch_idx] != val1 || self.last_nrpn_lsb[ch_idx] != val2
                        {
                            self.write_status_and_data(cc_ch, 99, Some(val1));
                            self.write_status_and_data(cc_ch, 98, Some(val2));
                            self.last_nrpn_msb[ch_idx] = val1;
                            self.last_nrpn_lsb[ch_idx] = val2;
                            self.last_rpn_msb[ch_idx] = 255;
                            self.last_rpn_lsb[ch_idx] = 255;
                        }
                        let val14 = scale_down(w1, 32, 14) as u16;
                        self.write_status_and_data(cc_ch, 6, Some(((val14 >> 7) & 0x7F) as u8));
                        self.write_status_and_data(cc_ch, 38, Some((val14 & 0x7F) as u8));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
