/// Combines 14-bit CC fragments into a single u16 value
#[must_use]
pub fn join_14bit(msb: u8, lsb: u8) -> u16 {
    (((msb & 0x7F) as u16) << 7) | ((lsb & 0x7F) as u16)
}

/// Splits a 14-bit u16 value into MSB and LSB
#[must_use]
pub fn split_14bit(value: u16) -> (u8, u8) {
    let msb = ((value >> 7) & 0x7F) as u8;
    let lsb = (value & 0x7F) as u8;
    (msb, lsb)
}

/// Scales a value up to 32 bits using the MIDI 2.0 Bit Duplication algorithm
#[must_use]
pub fn scale_up(value: u32, src_bits: u8, dst_bits: u8) -> u32 {
    if src_bits == dst_bits || src_bits == 0 || dst_bits == 0 {
        return value;
    }

    // Safety constraint: explicit 1-bit path to prevent shift overflow
    if src_bits == 1 {
        return if value == 0 { 0 } else { u32::MAX };
    }

    // Bound the value to its original bit width max. Use wrapping_shl to prevent overflow on `1 << 32`.
    let src_max = if src_bits == 32 { u32::MAX } else { (1_u32 << src_bits) - 1 };
    let val = value & src_max;

    // If it's the exact center or below (for 8-bit, 128 is center, but scaling logic dictates shifting)
    if val == 0 {
        return 0;
    }
    if val == src_max {
        return if dst_bits == 32 { u32::MAX } else { (1_u32 << dst_bits) - 1 };
    }

    // Explicit optimized fast-paths for hot operations (no-loop)
    if dst_bits == 32 {
        if src_bits == 7 {
            let left = val << 25;
            return left | (left >> 7) | (left >> 14) | (left >> 21) | (left >> 28);
        } else if src_bits == 8 {
            let left = val << 24;
            return left | (left >> 8) | (left >> 16) | (left >> 24);
        } else if src_bits == 14 {
            let left = val << 18;
            return left | (left >> 14) | (left >> 28);
        } else if src_bits == 16 {
            let left = val << 16;
            return left | (left >> 16);
        }
    }

    // Generic fallback for other bit depths
    let mut out = 0_u32;
    let mut bits_left = dst_bits as i32;
    let left_aligned = val << (32 - src_bits);

    while bits_left > 0 {
        let shift = 32 - bits_left;
        if shift > 0 {
            out |= left_aligned >> shift;
        } else {
            out |= left_aligned.wrapping_shl((-shift) as u32);
        }
        bits_left -= src_bits as i32;
    }

    out
}

/// Scales a value down from a higher bit depth
#[must_use]
pub fn scale_down(value: u32, src_bits: u8, dst_bits: u8) -> u32 {
    if src_bits <= dst_bits || dst_bits == 0 {
        return value;
    }
    let shift = src_bits - dst_bits;
    value >> shift
}
