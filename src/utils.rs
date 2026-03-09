// Constants from include/utils.h

/// Note Off status byte (0x80).
pub const NOTE_OFF: u8 = 0x80;
/// Note On status byte (0x90).
pub const NOTE_ON: u8 = 0x90;
/// Polyphonic Key Pressure status byte (0xA0).
pub const KEY_PRESSURE: u8 = 0xA0;
/// Control Change status byte (0xB0).
pub const CC: u8 = 0xB0;
/// Registered Parameter Number (RPN) controller (0x20).
pub const RPN: u8 = 0x20;
/// Non-Registered Parameter Number (NRPN) controller (0x30).
pub const NRPN: u8 = 0x30;
/// Relative RPN controller (0x40).
pub const RPN_RELATIVE: u8 = 0x40;
/// Relative NRPN controller (0x50).
pub const NRPN_RELATIVE: u8 = 0x50;
/// Program Change status byte (0xC0).
pub const PROGRAM_CHANGE: u8 = 0xC0;
/// Channel Pressure (Aftertouch) status byte (0xD0).
pub const CHANNEL_PRESSURE: u8 = 0xD0;
/// Pitch Bend status byte (0xE0).
pub const PITCH_BEND: u8 = 0xE0;
/// Per-Note Pitch Bend controller (0x60).
pub const PITCH_BEND_PERNOTE: u8 = 0x60;
/// Per-Note NRPN controller (0x10).
pub const NRPN_PERNOTE: u8 = 0x10;
/// Per-Note RPN controller (0x00).
pub const RPN_PERNOTE: u8 = 0x00;
/// Per-Note Management controller (0xF0).
pub const PERNOTE_MANAGE: u8 = 0xF0;

/// System Exclusive Start status byte (0xF0).
pub const SYSEX_START: u8 = 0xF0;
/// MIDI Time Code Quarter Frame status byte (0xF1).
pub const TIMING_CODE: u8 = 0xF1;
/// Song Position Pointer status byte (0xF2).
pub const SPP: u8 = 0xF2;
/// Song Select status byte (0xF3).
pub const SONG_SELECT: u8 = 0xF3;
/// Tune Request status byte (0xF6).
pub const TUNEREQUEST: u8 = 0xF6;
/// System Exclusive End status byte (0xF7).
pub const SYSEX_STOP: u8 = 0xF7;
/// Timing Clock status byte (0xF8).
pub const TIMINGCLOCK: u8 = 0xF8;
/// Start Sequence status byte (0xFA).
pub const SEQSTART: u8 = 0xFA;
/// Continue Sequence status byte (0xFB).
pub const SEQCONT: u8 = 0xFB;
/// Stop Sequence status byte (0xFC).
pub const SEQSTOP: u8 = 0xFC;
/// Active Sensing status byte (0xFE).
pub const ACTIVESENSE: u8 = 0xFE;
/// System Reset status byte (0xFF).
pub const SYSTEMRESET: u8 = 0xFF;

/// Utility Message: No Operation.
pub const UTILITY_NOOP: u8 = 0x0;
/// Utility Message: Jitter Reduction Clock.
pub const UTILITY_JRCLOCK: u8 = 0x1;
/// Utility Message: Jitter Reduction Timestamp.
pub const UTILITY_JRTS: u8 = 0x2;
/// Utility Message: Delta Clock Tick.
pub const UTILITY_DELTACLOCKTICK: u8 = 0x3;
/// Utility Message: Delta Clock Since Last.
pub const UTILITY_DELTACLOCKSINCE: u8 = 0x4;

/// Message Type: Utility (0x0).
pub const UMP_UTILITY: u8 = 0x0;
/// Message Type: System (0x1).
pub const UMP_SYSTEM: u8 = 0x1;
/// Message Type: MIDI 1.0 Channel Voice (0x2).
pub const UMP_M1CVM: u8 = 0x2;
/// Message Type: SysEx7 (0x3).
pub const UMP_SYSEX7: u8 = 0x3;
/// Message Type: MIDI 2.0 Channel Voice (0x4).
pub const UMP_M2CVM: u8 = 0x4;
/// Message Type: Data (0x5).
pub const UMP_DATA: u8 = 0x5;
/// Message Type: Flex Data (0xD).
pub const UMP_FLEX_DATA: u8 = 0xD;
/// Message Type: MIDI Endpoint (0xF).
pub const UMP_MIDI_ENDPOINT: u8 = 0xF;

// Helper functions

/// Scales a value up from a smaller bit depth to a larger bit depth.
///
/// This function performs proper bit replication to fill the wider range.
/// For example, scaling a 7-bit value 0x7F (max) to 32-bit will result in 0xFFFFFFFF (max).
///
/// # Arguments
///
/// * `src_val` - The source value to scale.
/// * `src_bits` - The number of bits in the source value.
/// * `dst_bits` - The number of bits in the destination value.
///
/// # Returns
///
/// The scaled 32-bit value.
pub fn scale_up(src_val: u32, src_bits: u8, dst_bits: u8) -> u32 {
    // Prevent panic on invalid input
    if src_bits == 0 || src_bits > 32 || dst_bits > 32 {
        return 0;
    }

    // Handle value of 0 - skip processing
    if src_val == 0 {
        return 0;
    }

    // handle 1-bit (bool) scaling
    if src_bits == 1 {
        if dst_bits == 32 {
            return u32::MAX;
        }
        return (1 << dst_bits) - 1;
    }

    // Specialized optimizations for common MIDI conversions
    if dst_bits == 32 {
        if src_bits == 7 {
            // Fast path for 7-bit to 32-bit (e.g., Velocity, CC)
            let shifted = src_val << 25;
            if src_val <= 64 {
                return shifted;
            }
            let v = src_val & 0x3F;
            return shifted | (v << 19) | (v << 13) | (v << 7) | (v << 1) | (v >> 5);
        } else if src_bits == 8 {
            // Fast path for 8-bit to 32-bit (e.g., some SysEx data mappings)
            // ⚡ Bolt Optimization: Replace loop-based generic scaling with unrolled bitmath
            let shifted = src_val << 24;
            if src_val <= 128 {
                return shifted;
            }
            let v = src_val & 0x7F;
            return shifted | (v << 17) | (v << 10) | (v << 3) | (v >> 4);
        } else if src_bits == 14 {
            // Fast path for 14-bit to 32-bit (e.g., Pitch Bend, High Res Velocity)
            let shifted = src_val << 18;
            if src_val <= 8192 {
                return shifted;
            }
            let v = src_val & 0x1FFF;
            return shifted | (v << 5) | (v >> 8);
        }
    }

    // simple bit shift
    let scale_bits = dst_bits.saturating_sub(src_bits);
    let bit_shifted_value = src_val << scale_bits;
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

    let mut current_bit_shifted = bit_shifted_value;
    while repeat_value != 0 {
        current_bit_shifted |= repeat_value;
        repeat_value >>= repeat_bits;
    }

    current_bit_shifted
}

/// Scales a value down from a larger bit depth to a smaller bit depth.
///
/// This is a simple right shift operation.
///
/// # Arguments
///
/// * `src_val` - The source value to scale.
/// * `src_bits` - The number of bits in the source value.
/// * `dst_bits` - The number of bits in the destination value.
///
/// # Returns
///
/// The scaled down value.
pub fn scale_down(src_val: u32, src_bits: u8, dst_bits: u8) -> u32 {
    let scale_bits = src_bits.saturating_sub(dst_bits);
    // ⚡ Bolt Optimization: Consolidate bounds checking into a single branch
    // after calculating `scale_bits`. This prevents duplicate branching
    // overhead in the hot path while maintaining the same safety guarantees.
    if scale_bits >= 32 || src_bits > 32 || dst_bits > 32 {
        return 0;
    }
    src_val >> scale_bits
}
