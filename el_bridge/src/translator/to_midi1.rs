//! MIDI 2.0 UMP to MIDI 1.0 byte stream Translator.
//!
//! Features:
//! - Converts MIDI 2.0 (MT=0x4) Channel Voice messages into compliant MIDI 1.0 serial bytes.
//! - Splits 14-bit CC / 32-bit CC into CC MSB (CC 0-31) and LSB (CC 32-63) pairs.
//! - Emits RPN / NRPN parameter selection and data entry byte sequences (CC 101/100/99/98 + CC 6/38).
//! - Reassembles MT=0x3 (SysEx7) UMP packets into continuous 7-bit SysEx byte sequences with 0xF0 and 0xF7 framing.
//! - Performs safe high-resolution down-scaling for velocity (16-bit to 7-bit), pitch bend (32-bit to 14-bit),
//!   and controllers (32-bit to 7-bit/14-bit).
//! - Optional running status generation to optimize legacy MIDI 1.0 wire bandwidth.

use el_core::ump::Ump;
use el_core::utils::scale_down;
use heapless::Vec;

/// Capacity for output byte queue in `ToMidi1Translator`.
const TRANSLATOR_BYTE_CAPACITY: usize = 64;

/// Translator converting MIDI 2.0 UMP packets into standard MIDI 1.0 byte streams.
#[derive(Clone, Debug)]
pub struct ToMidi1Translator {
    pub filter_group: Option<u8>,
    pub enable_running_status: bool,
    pub split_14bit_cc: bool,
    last_running_status: u8,
    last_rpn_msb: [u8; 16],
    last_rpn_lsb: [u8; 16],
    last_nrpn_msb: [u8; 16],
    last_nrpn_lsb: [u8; 16],
    out_queue: Vec<u8, TRANSLATOR_BYTE_CAPACITY>,
}

impl Default for ToMidi1Translator {
    fn default() -> Self {
        Self::new()
    }
}

impl ToMidi1Translator {
    /// Creates a new `ToMidi1Translator`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            filter_group: None,
            enable_running_status: false,
            split_14bit_cc: true,
            last_running_status: 0xFF,
            last_rpn_msb: [0xFF; 16],
            last_rpn_lsb: [0xFF; 16],
            last_nrpn_msb: [0xFF; 16],
            last_nrpn_lsb: [0xFF; 16],
            out_queue: Vec::new(),
        }
    }

    /// Resets translator state and flushes queue.
    pub fn reset(&mut self) {
        self.last_running_status = 0xFF;
        self.last_rpn_msb = [0xFF; 16];
        self.last_rpn_lsb = [0xFF; 16];
        self.last_nrpn_msb = [0xFF; 16];
        self.last_nrpn_lsb = [0xFF; 16];
        self.out_queue.clear();
    }

    /// Pops the next byte from the internal queue.
    #[must_use]
    pub fn pop_byte(&mut self) -> Option<u8> {
        if self.out_queue.is_empty() {
            None
        } else {
            Some(self.out_queue.remove(0))
        }
    }

    /// Returns `true` if bytes are ready to pop.
    #[must_use]
    pub fn has_byte(&self) -> bool {
        !self.out_queue.is_empty()
    }

    fn push_byte(&mut self, b: u8) {
        let _ = self.out_queue.push(b);
    }

    fn write_status_and_data(&mut self, status_ch: u8, d1: u8, d2: Option<u8>) {
        if !self.enable_running_status || self.last_running_status != status_ch {
            self.push_byte(status_ch);
            self.last_running_status = status_ch;
        }
        self.push_byte(d1 & 0x7F);
        if let Some(val2) = d2 {
            self.push_byte(val2 & 0x7F);
        }
    }

    /// Processes a UMP packet and enqueues corresponding MIDI 1.0 bytes.
    pub fn process_ump(&mut self, ump: &Ump) {
        let w0 = ump.data[0];
        let mt = (w0 >> 28) & 0xF;
        let group = ((w0 >> 24) & 0xF) as u8;

        if let Some(filter) = self.filter_group {
            if group != filter {
                return;
            }
        }

        match mt {
            // MT=0x1: System Real-Time and System Common
            0x1 => {
                let sys_byte = ((w0 >> 16) & 0xFF) as u8;
                if sys_byte == 0xF1 || sys_byte == 0xF3 {
                    self.push_byte(sys_byte);
                    self.push_byte(((w0 >> 8) & 0x7F) as u8);
                } else if sys_byte == 0xF2 {
                    self.push_byte(sys_byte);
                    self.push_byte(((w0 >> 8) & 0x7F) as u8);
                    self.push_byte((w0 & 0x7F) as u8);
                } else if sys_byte >= 0xF8 || sys_byte == 0xF6 {
                    self.push_byte(sys_byte);
                }
            }

            // MT=0x2: MIDI 1.0 Channel Voice
            0x2 => {
                let status_ch = ((w0 >> 16) & 0xFF) as u8;
                let status_nibble = status_ch & 0xF0;
                let d1 = ((w0 >> 8) & 0x7F) as u8;
                let d2 = (w0 & 0x7F) as u8;

                if status_nibble == 0xC0 || status_nibble == 0xD0 {
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

                if format == 0 || format == 1 {
                    self.push_byte(0xF0);
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

                if format == 0 || format == 3 {
                    self.push_byte(0xF7);
                }
            }

            // MT=0x4: MIDI 2.0 Channel Voice
            0x4 => {
                let status_nibble = ((w0 >> 16) & 0xF0) as u8;
                let channel = ((w0 >> 16) & 0x0F) as u8;
                let ch_idx = channel as usize;
                let val1 = ((w0 >> 8) & 0x7F) as u8;
                let val2 = (w0 & 0x7F) as u8;
                let w1 = ump.data[1];
                let status_ch = status_nibble | channel;

                match status_nibble {
                    0x80 => {
                        let vel = (scale_down(w1 >> 16, 16, 7) & 0x7F) as u8;
                        self.write_status_and_data(status_ch, val1, Some(vel));
                    }
                    0x90 => {
                        let mut vel = (scale_down(w1 >> 16, 16, 7) & 0x7F) as u8;
                        if vel == 0 {
                            vel = 1; // Prevent velocity 0 from acting as NoteOff in MIDI 1.0
                        }
                        self.write_status_and_data(status_ch, val1, Some(vel));
                    }
                    0xA0 => {
                        let press = (scale_down(w1, 32, 7) & 0x7F) as u8;
                        self.write_status_and_data(status_ch, val1, Some(press));
                    }
                    0xB0 => {
                        let cc_index = val1;
                        if self.split_14bit_cc && cc_index < 32 {
                            // Split 32-bit CC into 14-bit CC MSB + LSB pair
                            let val14 = scale_down(w1, 32, 14) as u16;
                            let msb = ((val14 >> 7) & 0x7F) as u8;
                            let lsb = (val14 & 0x7F) as u8;
                            self.write_status_and_data(status_ch, cc_index, Some(msb));
                            self.write_status_and_data(status_ch, cc_index + 32, Some(lsb));
                        } else {
                            let val7 = (scale_down(w1, 32, 7) & 0x7F) as u8;
                            self.write_status_and_data(status_ch, cc_index, Some(val7));
                        }
                    }
                    0xC0 => {
                        let bank_valid = (w0 & 0x1) != 0;
                        if bank_valid {
                            let bank_msb = ((w1 >> 8) & 0x7F) as u8;
                            let bank_lsb = (w1 & 0x7F) as u8;
                            let cc_ch = 0xB0 | channel;
                            self.write_status_and_data(cc_ch, 0, Some(bank_msb));
                            self.write_status_and_data(cc_ch, 32, Some(bank_lsb));
                        }
                        let prog = ((w1 >> 24) & 0x7F) as u8;
                        self.write_status_and_data(0xC0 | channel, prog, None);
                    }
                    0xD0 => {
                        let val7 = (scale_down(w1, 32, 7) & 0x7F) as u8;
                        self.write_status_and_data(status_ch, val7, None);
                    }
                    0xE0 => {
                        let pb14 = scale_down(w1, 32, 14) as u16;
                        let lsb = (pb14 & 0x7F) as u8;
                        let msb = ((pb14 >> 7) & 0x7F) as u8;
                        self.write_status_and_data(status_ch, lsb, Some(msb));
                    }
                    0x20 => {
                        // MIDI 2.0 RPN
                        let cc_ch = 0xB0 | channel;
                        if self.last_rpn_msb[ch_idx] != val1 || self.last_rpn_lsb[ch_idx] != val2 {
                            self.write_status_and_data(cc_ch, 101, Some(val1));
                            self.write_status_and_data(cc_ch, 100, Some(val2));
                            self.last_rpn_msb[ch_idx] = val1;
                            self.last_rpn_lsb[ch_idx] = val2;
                            self.last_nrpn_msb[ch_idx] = 0xFF;
                            self.last_nrpn_lsb[ch_idx] = 0xFF;
                        }
                        let val14 = scale_down(w1, 32, 14) as u16;
                        self.write_status_and_data(cc_ch, 6, Some(((val14 >> 7) & 0x7F) as u8));
                        self.write_status_and_data(cc_ch, 38, Some((val14 & 0x7F) as u8));
                    }
                    0x30 => {
                        // MIDI 2.0 NRPN
                        let cc_ch = 0xB0 | channel;
                        if self.last_nrpn_msb[ch_idx] != val1 || self.last_nrpn_lsb[ch_idx] != val2
                        {
                            self.write_status_and_data(cc_ch, 99, Some(val1));
                            self.write_status_and_data(cc_ch, 98, Some(val2));
                            self.last_nrpn_msb[ch_idx] = val1;
                            self.last_nrpn_lsb[ch_idx] = val2;
                            self.last_rpn_msb[ch_idx] = 0xFF;
                            self.last_rpn_lsb[ch_idx] = 0xFF;
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
