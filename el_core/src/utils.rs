/// Combines 14-bit CC fragments into a single u16 value
#[must_use]
pub fn join_14bit(msb: u8, lsb: u8) -> u16 {
    (u16::from(msb & 0x7F) << 7) | u16::from(lsb & 0x7F)
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
    let src_max = if src_bits >= 32 {
        u32::MAX
    } else {
        (1_u32 << src_bits) - 1
    };
    let val = value & src_max;

    // If it's the exact center or below (for 8-bit, 128 is center, but scaling logic dictates shifting)
    if val == 0 {
        return 0;
    }
    if val == src_max {
        return if dst_bits >= 32 {
            u32::MAX
        } else {
            (1_u32 << dst_bits) - 1
        };
    }

    // Explicit optimized fast-paths for hot operations (no-loop)
    if dst_bits == 32 {
        if src_bits == 7 {
            let shifted = val << 25;
            if val <= 64 {
                return shifted;
            }
            let v = val & 0x3F;
            return shifted | (v << 19) | (v << 13) | (v << 7) | (v << 1) | (v >> 5);
        } else if src_bits == 8 {
            let shifted = val << 24;
            if val <= 128 {
                return shifted;
            }
            let v = val & 0x7F;
            return shifted | (v << 17) | (v << 10) | (v << 3) | (v >> 4);
        } else if src_bits == 14 {
            let shifted = val << 18;
            if val <= 8192 {
                return shifted;
            }
            let v = val & 0x1FFF;
            return shifted | (v << 5) | (v >> 8);
        } else if src_bits == 16 {
            let shifted = val << 16;
            if val <= 32768 {
                return shifted;
            }
            let v = val & 0x7FFF;
            return shifted | (v << 1) | (v >> 14);
        }
    }

    // Generic fallback for other bit depths
    let mut out = 0_u32;
    let mut bits_left = i32::from(if dst_bits > 32 { 32 } else { dst_bits });

    // Prevent underflow panic if src_bits > 32
    let shift_amount = 32_i32 - i32::from(src_bits);
    let left_aligned = if shift_amount <= -32 {
        0
    } else if shift_amount < 0 {
        val >> (-shift_amount)
    } else {
        val << shift_amount
    };

    while bits_left > 0 {
        let shift = 32 - bits_left;
        if shift > 0 {
            out |= left_aligned >> shift;
        } else {
            out |= left_aligned.wrapping_shl((-shift) as u32);
        }
        bits_left -= i32::from(src_bits);
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
    if shift >= 32 {
        0
    } else {
        value >> shift
    }
}
