//! MIDI 1.0 byte stream to MIDI 2.0 UMP Translator.
//!
//! Features:
//! - Aggregates 14-bit CCs (CC 0-31 + CC 32-63) into high-resolution 32-bit MT=0x4 CC messages.
//! - Aggregates RPN (CC 101/100/6/38) and NRPN (CC 99/98/6/38) sequences into 32-bit MT=0x4 RPN (0x4020_0000)
//!   and NRPN (0x4030_0000) messages per the MIDI 2.0 (M2-104-UM) specification.
//! - Bounded SysEx accumulation chunking into 64-bit (MT=0x3) SysEx7 UMP packets.
//! - Converts standard Channel Voice (Note On/Off, Poly Pressure, CC, Program Change, Channel Pressure, Pitch Bend)
//!   into MIDI 2.0 (MT=0x4) messages using `scale_up`.
//! - Passes through System Common & Real-Time messages into MT=0x1 UMPs.
//! - Correctly handles running status and real-time interleaving.

use crate::state::{ChannelStateTracker, ParameterType, SYSEX_MAX_CAPACITY};
use el_core::builder::{SystemBuilder, VoiceBuilder};
use el_core::ump::Ump;
use el_core::utils::scale_up;
use heapless::Vec;

/// Capacity for output packet queue inside `ToUmpTranslator`.
const TRANSLATOR_OUTPUT_CAPACITY: usize = 16;

/// Bidirectional translator translating MIDI 1.0 serial bytes into MIDI 2.0 UMPs.
#[derive(Clone, Debug)]
pub struct ToUmpTranslator {
    pub group: u8,
    pub aggregate_14bit_cc: bool,
    pub aggregate_nrpn: bool,
    state_tracker: ChannelStateTracker,
    running_status: u8,
    d0: u8,
    d1: Option<u8>,
    out_queue: Vec<Ump, TRANSLATOR_OUTPUT_CAPACITY>,
}

impl Default for ToUmpTranslator {
    fn default() -> Self {
        Self::new(0)
    }
}

impl ToUmpTranslator {
    /// Creates a new `ToUmpTranslator` with the specified default UMP group (0-15).
    #[must_use]
    pub const fn new(group: u8) -> Self {
        Self {
            group: group & 0x0F,
            aggregate_14bit_cc: true,
            aggregate_nrpn: true,
            state_tracker: ChannelStateTracker::new(),
            running_status: 0,
            d0: 0,
            d1: None,
            out_queue: Vec::new(),
        }
    }

    /// Resets translator state and flushes queues.
    pub fn reset(&mut self) {
        self.state_tracker.reset();
        self.running_status = 0;
        self.d0 = 0;
        self.d1 = None;
        self.out_queue.clear();
    }

    /// Pops the next translated UMP packet from the internal queue.
    #[must_use]
    pub fn pop_ump(&mut self) -> Option<Ump> {
        if self.out_queue.is_empty() {
            None
        } else {
            Some(self.out_queue.remove(0))
        }
    }

    /// Returns `true` if translated UMPs are ready to pop.
    #[must_use]
    pub fn has_ump(&self) -> bool {
        !self.out_queue.is_empty()
    }

    fn push_out(&mut self, ump: Ump) {
        let _ = self.out_queue.push(ump);
    }

    /// Feeds a single MIDI 1.0 byte into the translator.
    pub fn process_byte(&mut self, byte: u8) {
        // Real-Time messages (0xF8..=0xFF) can be interleaved anywhere
        if byte >= 0xF8 {
            self.emit_system_realtime(byte);
            return;
        }

        // Status Byte (bit 7 set)
        if (byte & 0x80) != 0 {
            if byte == 0xF0 {
                // SysEx Start
                self.state_tracker.sysex.start();
                self.d0 = 0;
                self.d1 = None;
                return;
            } else if byte == 0xF7 {
                // SysEx End
                if self.state_tracker.sysex.active {
                    self.flush_sysex7();
                    self.state_tracker.sysex.reset();
                }
                self.d0 = 0;
                self.d1 = None;
                return;
            }

            // Other status byte aborts ongoing SysEx
            if self.state_tracker.sysex.active {
                self.flush_sysex7();
                self.state_tracker.sysex.reset();
            }

            if byte < 0xF0 {
                // Channel Voice status
                self.running_status = byte;
                self.d0 = byte;
                self.d1 = None;
            } else {
                // System Common status (clears running status)
                self.running_status = 0;
                self.d0 = byte;
                self.d1 = None;
            }
            return;
        }

        // Data Byte (bit 7 clear)
        if self.state_tracker.sysex.active {
            if self.state_tracker.sysex.push(byte & 0x7F).is_err() {
                // Bounded buffer reached: chunk out early
                self.flush_sysex7();
                self.state_tracker.sysex.reset();
                let _ = self.state_tracker.sysex.push(byte & 0x7F);
                self.state_tracker.sysex.active = true;
            }
            return;
        }

        // If d0 is 0 but running_status is active, restore d0
        if self.d0 == 0 && self.running_status != 0 {
            self.d0 = self.running_status;
            self.d1 = None;
        }

        if self.d0 != 0 {
            let status_nibble = self.d0 & 0xF0;
            if status_nibble == 0xC0 || status_nibble == 0xD0 || self.d0 == 0xF1 || self.d0 == 0xF3
            {
                // Single-data-byte messages
                self.emit_channel_or_system(self.d0, byte & 0x7F, 0);
                if self.d0 >= 0xF0 {
                    self.d0 = 0;
                } else {
                    self.d0 = self.running_status;
                }
                self.d1 = None;
            } else {
                // Two-data-byte messages
                if let Some(first) = self.d1 {
                    self.emit_channel_or_system(self.d0, first, byte & 0x7F);
                    if self.d0 >= 0xF0 {
                        self.d0 = 0;
                    } else {
                        self.d0 = self.running_status;
                    }
                    self.d1 = None;
                } else {
                    self.d1 = Some(byte & 0x7F);
                }
            }
        }
    }

    fn emit_system_realtime(&mut self, byte: u8) {
        let ump = match byte {
            0xF8 => SystemBuilder::timing_clock(self.group),
            0xFA => SystemBuilder::start(self.group),
            0xFB => SystemBuilder::continue_seq(self.group),
            0xFC => SystemBuilder::stop(self.group),
            0xFE => SystemBuilder::active_sensing(self.group),
            0xFF => SystemBuilder::system_reset(self.group),
            _ => Ump::new(
                0x1000_0000 | ((self.group as u32 & 0xF) << 24) | ((byte as u32) << 16),
                0,
                0,
                0,
            ),
        };
        self.push_out(ump);
    }

    fn emit_channel_or_system(&mut self, status_byte: u8, d1: u8, d2: u8) {
        if status_byte >= 0xF0 {
            // System Common
            match status_byte {
                0xF1 => {
                    let ump = Ump::new(
                        0x1000_0000
                            | ((self.group as u32 & 0xF) << 24)
                            | 0x00F1_0000
                            | ((d1 as u32) << 8),
                        0,
                        0,
                        0,
                    );
                    self.push_out(ump);
                }
                0xF2 => {
                    let pos = ((d2 as u16) << 7) | (d1 as u16);
                    self.push_out(SystemBuilder::song_position_pointer(self.group, pos));
                }
                0xF3 => {
                    self.push_out(SystemBuilder::song_select(self.group, d1));
                }
                0xF6 => {
                    self.push_out(SystemBuilder::tune_request(self.group));
                }
                _ => {}
            }
            return;
        }

        let channel = status_byte & 0x0F;
        let ch_idx = channel as usize;
        let status_nibble = status_byte & 0xF0;

        match status_nibble {
            0x80 => {
                // Note Off
                let vel16 = scale_up(d2 as u32, 7, 16) as u16;
                self.push_out(VoiceBuilder::midi2_note_off(
                    self.group, channel, d1, 0, vel16, 0,
                ));
            }
            0x90 => {
                // Note On (velocity 0 is Note Off)
                if d2 == 0 {
                    self.push_out(VoiceBuilder::midi2_note_off(
                        self.group, channel, d1, 0, 0, 0,
                    ));
                } else {
                    let vel16 = scale_up(d2 as u32, 7, 16) as u16;
                    self.push_out(VoiceBuilder::midi2_note_on(
                        self.group, channel, d1, 0, vel16, 0,
                    ));
                }
            }
            0xA0 => {
                // Polyphonic Key Pressure: MT=0x4, Status=0xA
                let press32 = scale_up(d2 as u32, 7, 32);
                let w1 = 0x40A0_0000
                    | ((self.group as u32 & 0xF) << 24)
                    | ((channel as u32 & 0xF) << 16)
                    | ((d1 as u32 & 0x7F) << 8);
                self.push_out(Ump::new(w1, press32, 0, 0));
            }
            0xB0 => {
                // Control Change
                self.handle_cc(channel, ch_idx, d1, d2);
            }
            0xC0 => {
                // Program Change: MT=0x4, Status=0xC
                let w1 = 0x40C0_0000
                    | ((self.group as u32 & 0xF) << 24)
                    | ((channel as u32 & 0xF) << 16);
                let w2 = (d1 as u32 & 0x7F) << 24;
                self.push_out(Ump::new(w1, w2, 0, 0));
            }
            0xD0 => {
                // Channel Pressure: MT=0x4, Status=0xD
                let press32 = scale_up(d1 as u32, 7, 32);
                let w1 = 0x40D0_0000
                    | ((self.group as u32 & 0xF) << 24)
                    | ((channel as u32 & 0xF) << 16);
                self.push_out(Ump::new(w1, press32, 0, 0));
            }
            0xE0 => {
                // Pitch Bend (14-bit unsigned)
                let pb14 = ((d2 as u32) << 7) | (d1 as u32);
                let pb32 = scale_up(pb14, 14, 32);
                self.push_out(VoiceBuilder::midi2_pitch_bend(self.group, channel, pb32));
            }
            _ => {}
        }
    }

    fn handle_cc(&mut self, channel: u8, ch_idx: usize, cc_num: u8, val: u8) {
        // Check for RPN/NRPN aggregation
        if self.aggregate_nrpn {
            match cc_num {
                101 => {
                    // RPN MSB
                    self.state_tracker.param[ch_idx].active_param_type = Some(ParameterType::Rpn);
                    self.state_tracker.param[ch_idx].param_msb = Some(val);
                    return;
                }
                100 => {
                    // RPN LSB
                    self.state_tracker.param[ch_idx].active_param_type = Some(ParameterType::Rpn);
                    self.state_tracker.param[ch_idx].param_lsb = Some(val);
                    return;
                }
                99 => {
                    // NRPN MSB
                    self.state_tracker.param[ch_idx].active_param_type = Some(ParameterType::Nrpn);
                    self.state_tracker.param[ch_idx].param_msb = Some(val);
                    return;
                }
                98 => {
                    // NRPN LSB
                    self.state_tracker.param[ch_idx].active_param_type = Some(ParameterType::Nrpn);
                    self.state_tracker.param[ch_idx].param_lsb = Some(val);
                    return;
                }
                6 => {
                    // Data Entry MSB
                    self.state_tracker.param[ch_idx].data_msb = Some(val);
                    if self.state_tracker.param[ch_idx].is_param_selected() {
                        let param_type =
                            self.state_tracker.param[ch_idx].active_param_type.unwrap();
                        let p_msb = self.state_tracker.param[ch_idx].param_msb.unwrap();
                        let p_lsb = self.state_tracker.param[ch_idx].param_lsb.unwrap();
                        let d_lsb = self.state_tracker.param[ch_idx].data_lsb.unwrap_or(0);
                        let val14 = ((val as u32) << 7) | (d_lsb as u32);
                        let val32 = scale_up(val14, 14, 32);

                        let ump = match param_type {
                            ParameterType::Rpn => {
                                let combined = ((self.group as u32 & 0xF) << 24)
                                    | ((channel as u32 & 0xF) << 16)
                                    | ((p_msb as u32 & 0x7F) << 8)
                                    | (p_lsb as u32 & 0x7F);
                                Ump::new(0x4020_0000 | combined, val32, 0, 0)
                            }
                            ParameterType::Nrpn => {
                                let combined = ((self.group as u32 & 0xF) << 24)
                                    | ((channel as u32 & 0xF) << 16)
                                    | ((p_msb as u32 & 0x7F) << 8)
                                    | (p_lsb as u32 & 0x7F);
                                Ump::new(0x4030_0000 | combined, val32, 0, 0)
                            }
                        };
                        self.push_out(ump);
                    }
                    return;
                }
                38 => {
                    // Data Entry LSB
                    self.state_tracker.param[ch_idx].data_lsb = Some(val);
                    if self.state_tracker.param[ch_idx].is_param_selected()
                        && self.state_tracker.param[ch_idx].data_msb.is_some()
                    {
                        let param_type =
                            self.state_tracker.param[ch_idx].active_param_type.unwrap();
                        let p_msb = self.state_tracker.param[ch_idx].param_msb.unwrap();
                        let p_lsb = self.state_tracker.param[ch_idx].param_lsb.unwrap();
                        let d_msb = self.state_tracker.param[ch_idx].data_msb.unwrap();
                        let val14 = ((d_msb as u32) << 7) | (val as u32);
                        let val32 = scale_up(val14, 14, 32);

                        let ump = match param_type {
                            ParameterType::Rpn => {
                                let combined = ((self.group as u32 & 0xF) << 24)
                                    | ((channel as u32 & 0xF) << 16)
                                    | ((p_msb as u32 & 0x7F) << 8)
                                    | (p_lsb as u32 & 0x7F);
                                Ump::new(0x4020_0000 | combined, val32, 0, 0)
                            }
                            ParameterType::Nrpn => {
                                let combined = ((self.group as u32 & 0xF) << 24)
                                    | ((channel as u32 & 0xF) << 16)
                                    | ((p_msb as u32 & 0x7F) << 8)
                                    | (p_lsb as u32 & 0x7F);
                                Ump::new(0x4030_0000 | combined, val32, 0, 0)
                            }
                        };
                        self.push_out(ump);
                    }
                    return;
                }
                _ => {}
            }
        }

        // Check for 14-bit CC aggregation (CC 0..31 MSB, CC 32..63 LSB)
        if self.aggregate_14bit_cc {
            if cc_num < 32 {
                let msb_idx = cc_num as usize;
                self.state_tracker.cc14[ch_idx].msb[msb_idx] = Some(val);
                let lsb = self.state_tracker.cc14[ch_idx].lsb[msb_idx].unwrap_or(0);
                let val14 = ((val as u32) << 7) | (lsb as u32);
                let val32 = scale_up(val14, 14, 32);
                self.push_out(VoiceBuilder::midi2_cc(self.group, channel, cc_num, val32));
                return;
            } else if (32..64).contains(&cc_num) {
                let msb_idx = (cc_num - 32) as usize;
                self.state_tracker.cc14[ch_idx].lsb[msb_idx] = Some(val);
                if let Some(msb) = self.state_tracker.cc14[ch_idx].msb[msb_idx] {
                    let val14 = ((msb as u32) << 7) | (val as u32);
                    let val32 = scale_up(val14, 14, 32);
                    self.push_out(VoiceBuilder::midi2_cc(
                        self.group,
                        channel,
                        msb_idx as u8,
                        val32,
                    ));
                    return;
                }
            }
        }

        // Standard 7-bit CC scaled up to 32-bit
        let val32 = scale_up(val as u32, 7, 32);
        self.push_out(VoiceBuilder::midi2_cc(self.group, channel, cc_num, val32));
    }

    fn flush_sysex7(&mut self) {
        let total_len = self.state_tracker.sysex.buffer.len();
        if total_len == 0 {
            let w1 = 0x3000_0000 | ((self.group as u32 & 0xF) << 24);
            self.push_out(Ump::new(w1, 0, 0, 0));
            return;
        }

        // Copy buffer contents to local stack to release borrow on self
        let mut local_buf = [0_u8; SYSEX_MAX_CAPACITY];
        local_buf[..total_len].copy_from_slice(self.state_tracker.sysex.buffer.as_slice());

        let mut offset = 0;
        let mut first = true;

        while offset < total_len {
            let chunk_len = (total_len - offset).min(6);
            let is_last = offset + chunk_len >= total_len;
            let format: u8 = if first && is_last {
                0 // Complete
            } else if first {
                1 // Start
            } else if is_last {
                3 // End
            } else {
                2 // Continue
            };
            first = false;

            let mut chunk = [0_u8; 6];
            chunk[..chunk_len].copy_from_slice(&local_buf[offset..offset + chunk_len]);
            offset += chunk_len;

            let w1 = 0x3000_0000
                | ((self.group as u32 & 0xF) << 24)
                | ((format as u32 & 0xF) << 20)
                | ((chunk_len as u32 & 0xF) << 16)
                | ((chunk[0] as u32) << 8)
                | (chunk[1] as u32);
            let w2 = ((chunk[2] as u32) << 24)
                | ((chunk[3] as u32) << 16)
                | ((chunk[4] as u32) << 8)
                | (chunk[5] as u32);

            self.push_out(Ump::new(w1, w2, 0, 0));
        }
    }
}
