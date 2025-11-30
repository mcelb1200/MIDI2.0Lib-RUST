use crate::utils::*;
use crate::ump::{UmpMessage, MessageType};

pub struct UmpFactory;

impl UmpFactory {
    // Utility Messages (MT=0x0)
    pub fn noop() -> UmpMessage {
        UmpMessage::Ump32(0)
    }

    pub fn jr_clock(clock_time: u16) -> UmpMessage {
        let val = ((UTILITY_JRCLOCK as u32) << 20) + (clock_time as u32);
        UmpMessage::Ump32(val)
    }

    pub fn jr_timestamp(timestamp: u16) -> UmpMessage {
        let val = ((UTILITY_JRTS as u32) << 20) + (timestamp as u32);
        UmpMessage::Ump32(val)
    }

    // System Common / Realtime (MT=0x1)
    fn mt1_create(group: u8, status: u8, val1: u8, val2: u8) -> UmpMessage {
        let w = (((UMP_SYSTEM as u32) << 28) + (((group & 0xF) as u32) << 24))
        + (((status as u32) & 0xFF) << 16)
        + (((val1 as u32) & 0x7F) << 8) + ((val2 as u32) & 0x7F);
        UmpMessage::Ump32(w)
    }

    pub fn timing_clock(group: u8) -> UmpMessage {
        Self::mt1_create(group, TIMINGCLOCK, 0, 0)
    }

    pub fn start(group: u8) -> UmpMessage {
        Self::mt1_create(group, SEQSTART, 0, 0)
    }

    pub fn continue_seq(group: u8) -> UmpMessage {
        Self::mt1_create(group, SEQCONT, 0, 0)
    }

    pub fn stop(group: u8) -> UmpMessage {
        Self::mt1_create(group, SEQSTOP, 0, 0)
    }

    pub fn active_sensing(group: u8) -> UmpMessage {
        Self::mt1_create(group, ACTIVESENSE, 0, 0)
    }

    pub fn system_reset(group: u8) -> UmpMessage {
        Self::mt1_create(group, SYSTEMRESET, 0, 0)
    }

    pub fn mtc_quarter_frame(group: u8, data: u8) -> UmpMessage {
        Self::mt1_create(group, TIMING_CODE, data, 0)
    }

    pub fn song_position_pointer(group: u8, position: u16) -> UmpMessage {
        Self::mt1_create(group, SPP, (position & 0x7F) as u8, ((position >> 7) & 0x7F) as u8)
    }

    pub fn song_select(group: u8, song: u8) -> UmpMessage {
        Self::mt1_create(group, SONG_SELECT, song, 0)
    }

    pub fn tune_request(group: u8) -> UmpMessage {
        Self::mt1_create(group, TUNEREQUEST, 0, 0)
    }


    // MIDI 1.0 Channel Voice (MT=0x2)
    fn mt2_create(group: u8, status: u8, channel: u8, val1: u8, val2: u8) -> UmpMessage {
        let mut message = ((UMP_M1CVM as u32) << 28) + (((group & 0xF) as u32) << 24);
        message += (((status & 0xF0) | (channel & 0xF)) as u32) << 16;
        message += ((val1 & 0x7F) as u32) << 8;
        message += (val2 & 0x7F) as u32;
        UmpMessage::Ump32(message)
    }

    pub fn midi1_note_off(group: u8, channel: u8, note: u8, velocity: u8) -> UmpMessage {
        Self::mt2_create(group, NOTE_OFF, channel, note, velocity)
    }

    pub fn midi1_note_on(group: u8, channel: u8, note: u8, velocity: u8) -> UmpMessage {
        Self::mt2_create(group, NOTE_ON, channel, note, velocity)
    }

    pub fn midi1_poly_pressure(group: u8, channel: u8, note: u8, pressure: u8) -> UmpMessage {
        Self::mt2_create(group, KEY_PRESSURE, channel, note, pressure)
    }

    pub fn midi1_control_change(group: u8, channel: u8, index: u8, value: u8) -> UmpMessage {
        Self::mt2_create(group, CC, channel, index, value)
    }

    pub fn midi1_program_change(group: u8, channel: u8, program: u8) -> UmpMessage {
        Self::mt2_create(group, PROGRAM_CHANGE, channel, program, 0)
    }

    pub fn midi1_channel_pressure(group: u8, channel: u8, pressure: u8) -> UmpMessage {
        Self::mt2_create(group, CHANNEL_PRESSURE, channel, pressure, 0)
    }

    pub fn midi1_pitch_bend(group: u8, channel: u8, value: u16) -> UmpMessage {
        Self::mt2_create(group, PITCH_BEND, channel, (value & 0x7F) as u8, ((value >> 7) & 0x7F) as u8)
    }

    // MIDI 2.0 Channel Voice (MT=0x4)
    fn mt4_create_first_word(group: u8, status: u8, channel: u8, val1: u8, val2: u8) -> u32 {
        let mut message = ((UMP_M2CVM as u32) << 28) + (((group & 0xF) as u32) << 24);
        message += (((status & 0xF0) | (channel & 0xF)) as u32) << 16;
        message += (val1 as u32) << 8;
        message += val2 as u32;
        message
    }

    pub fn midi2_note_off(group: u8, channel: u8, note: u8, attribute_type: u8, velocity: u16, attribute_data: u16) -> UmpMessage {
        let word1 = Self::mt4_create_first_word(group, NOTE_OFF, channel, note, attribute_type);
        let word2 = ((velocity as u32) << 16) | (attribute_data as u32);
        UmpMessage::Ump64(word1, word2)
    }

    pub fn midi2_note_on(group: u8, channel: u8, note: u8, attribute_type: u8, velocity: u16, attribute_data: u16) -> UmpMessage {
        let word1 = Self::mt4_create_first_word(group, NOTE_ON, channel, note, attribute_type);
        let word2 = ((velocity as u32) << 16) | (attribute_data as u32);
        UmpMessage::Ump64(word1, word2)
    }

    pub fn midi2_poly_pressure(group: u8, channel: u8, note: u8, pressure: u32) -> UmpMessage {
        let word1 = Self::mt4_create_first_word(group, KEY_PRESSURE, channel, note, 0);
        UmpMessage::Ump64(word1, pressure)
    }

    pub fn midi2_control_change(group: u8, channel: u8, index: u8, value: u32) -> UmpMessage {
        let word1 = Self::mt4_create_first_word(group, CC, channel, index, 0);
        UmpMessage::Ump64(word1, value)
    }

    pub fn midi2_rpn(group: u8, channel: u8, bank: u8, index: u8, value: u32) -> UmpMessage {
        let word1 = Self::mt4_create_first_word(group, RPN, channel, bank, index);
        UmpMessage::Ump64(word1, value)
    }

    pub fn midi2_nrpn(group: u8, channel: u8, bank: u8, index: u8, value: u32) -> UmpMessage {
        let word1 = Self::mt4_create_first_word(group, NRPN, channel, bank, index);
        UmpMessage::Ump64(word1, value)
    }

    pub fn midi2_relative_rpn(group: u8, channel: u8, bank: u8, index: u8, value: i32) -> UmpMessage {
        let word1 = Self::mt4_create_first_word(group, RPN_RELATIVE, channel, bank, index);
        UmpMessage::Ump64(word1, value as u32)
    }

    pub fn midi2_relative_nrpn(group: u8, channel: u8, bank: u8, index: u8, value: i32) -> UmpMessage {
        let word1 = Self::mt4_create_first_word(group, NRPN_RELATIVE, channel, bank, index);
        UmpMessage::Ump64(word1, value as u32)
    }

    pub fn midi2_program_change(group: u8, channel: u8, program: u8, bank_valid: bool, bank: u8, index: u8) -> UmpMessage {
         let word1 = Self::mt4_create_first_word(group, PROGRAM_CHANGE, channel, 0, if bank_valid { 1 } else { 0 });
         let word2 = ((program as u32) << 24) + if bank_valid { ((bank as u32) << 8) + (index as u32) } else { 0 };
         UmpMessage::Ump64(word1, word2)
    }

    pub fn midi2_channel_pressure(group: u8, channel: u8, pressure: u32) -> UmpMessage {
        let word1 = Self::mt4_create_first_word(group, CHANNEL_PRESSURE, channel, 0, 0);
        UmpMessage::Ump64(word1, pressure)
    }

    pub fn midi2_pitch_bend(group: u8, channel: u8, value: u32) -> UmpMessage {
        let word1 = Self::mt4_create_first_word(group, PITCH_BEND, channel, 0, 0);
        UmpMessage::Ump64(word1, value)
    }

    pub fn midi2_per_note_pitch_bend(group: u8, channel: u8, note: u8, value: u32) -> UmpMessage {
         let word1 = Self::mt4_create_first_word(group, PITCH_BEND_PERNOTE, channel, note, 0);
         UmpMessage::Ump64(word1, value)
    }
}
