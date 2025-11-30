
// Constants from include/utils.h

pub const NOTE_OFF: u8 = 0x80;
pub const NOTE_ON: u8 = 0x90;
pub const KEY_PRESSURE: u8 = 0xA0;
pub const CC: u8 = 0xB0;
pub const RPN: u8 = 0x20;
pub const NRPN: u8 = 0x30;
pub const RPN_RELATIVE: u8 = 0x40;
pub const NRPN_RELATIVE: u8 = 0x50;
pub const PROGRAM_CHANGE: u8 = 0xC0;
pub const CHANNEL_PRESSURE: u8 = 0xD0;
pub const PITCH_BEND: u8 = 0xE0;
pub const PITCH_BEND_PERNOTE: u8 = 0x60;
pub const NRPN_PERNOTE: u8 = 0x10;
pub const RPN_PERNOTE: u8 = 0x00;
pub const PERNOTE_MANAGE: u8 = 0xF0;

pub const SYSEX_START: u8 = 0xF0;
pub const TIMING_CODE: u8 = 0xF1;
pub const SPP: u8 = 0xF2;
pub const SONG_SELECT: u8 = 0xF3;
pub const TUNEREQUEST: u8 = 0xF6;
pub const SYSEX_STOP: u8 = 0xF7;
pub const TIMINGCLOCK: u8 = 0xF8;
pub const SEQSTART: u8 = 0xFA;
pub const SEQCONT: u8 = 0xFB;
pub const SEQSTOP: u8 = 0xFC;
pub const ACTIVESENSE: u8 = 0xFE;
pub const SYSTEMRESET: u8 = 0xFF;

pub const UTILITY_NOOP: u8 = 0x0;
pub const UTILITY_JRCLOCK: u8 = 0x1;
pub const UTILITY_JRTS: u8 = 0x2;
pub const UTILITY_DELTACLOCKTICK: u8 = 0x3;
pub const UTILITY_DELTACLOCKSINCE: u8 = 0x4;

pub const UMP_UTILITY: u8 = 0x0;
pub const UMP_SYSTEM: u8 = 0x1;
pub const UMP_M1CVM: u8 = 0x2;
pub const UMP_SYSEX7: u8 = 0x3;
pub const UMP_M2CVM: u8 = 0x4;
pub const UMP_DATA: u8 = 0x5;
pub const UMP_FLEX_DATA: u8 = 0xD;
pub const UMP_MIDI_ENDPOINT: u8 = 0xF;

// Helper functions

pub fn scale_up(src_val: u32, src_bits: u8, dst_bits: u8) -> u32 {
    // Handle value of 0 - skip processing
    if src_val == 0 {
        return 0;
    }

    // handle 1-bit (bool) scaling
    if src_bits == 1 {
        return (1 << dst_bits) - 1;
    }

    // simple bit shift
    let scale_bits = dst_bits.saturating_sub(src_bits);
    let mut bit_shifted_value = src_val << scale_bits;
    let src_center = 1 << (src_bits - 1);

    if src_val <= src_center {
        return bit_shifted_value;
    }

    // expanded bit repeat scheme
    let repeat_bits = src_bits - 1;
    let repeat_mask = (1 << repeat_bits) - 1;
    let mut repeat_value = src_val & repeat_mask;

    if scale_bits > repeat_bits {
        repeat_value <<= scale_bits - repeat_bits;
    } else {
        repeat_value >>= repeat_bits - scale_bits;
    }

    while repeat_value != 0 {
        bit_shifted_value |= repeat_value;
        repeat_value >>= repeat_bits;
    }

    bit_shifted_value
}

pub fn scale_down(src_val: u32, src_bits: u8, dst_bits: u8) -> u32 {
    let scale_bits = src_bits.saturating_sub(dst_bits);
    src_val >> scale_bits
}
