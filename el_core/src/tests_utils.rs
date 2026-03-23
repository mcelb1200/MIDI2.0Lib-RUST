#[cfg(test)]
mod tests {
    use crate::utils::{join_14bit, scale_down, scale_up, split_14bit};

    #[test]
    fn test_14bit_join_split() {
        let msb = 0x20;
        let lsb = 0x40;
        let joined = join_14bit(msb, lsb);
        assert_eq!(joined, 0x1040);

        let (split_msb, split_lsb) = split_14bit(joined);
        assert_eq!(split_msb, msb);
        assert_eq!(split_lsb, lsb);
    }

    #[test]
    fn test_scale_up() {
        // 7-bit to 32-bit (MIDI 1.0 to MIDI 2.0 conversion)
        assert_eq!(scale_up(0, 7, 32), 0);
        // Correct MIDI 2.0 scaling logic for 64 (0x40):
        // 64 is 1000000 in binary.
        // With scale_up logic returning 2147483648 (0x80000000).
        assert_eq!(scale_up(64, 7, 32), 2147483648);
        assert_eq!(scale_up(127, 7, 32), u32::MAX); // Max

        // 1-bit to 32-bit fallback explicit check
        assert_eq!(scale_up(1, 1, 32), u32::MAX);
        assert_eq!(scale_up(0, 1, 32), 0);
    }

    #[test]
    fn test_scale_down() {
        // 32-bit to 7-bit (MIDI 2.0 to MIDI 1.0 conversion)
        assert_eq!(scale_down(u32::MAX, 32, 7), 127);
        assert_eq!(scale_down(0x80808080, 32, 7), 64);
        assert_eq!(scale_down(0, 32, 7), 0);
    }

    #[test]
    fn test_scale_down_out_of_bounds() {
        // Value has bits outside the source bit width
        // E.g., src_bits = 14, but value has bit 15 set.
        let val: u32 = 0x8000; // Bit 15 set, but max for 14 bits is 0x3FFF
        let masked_val = val & 0x3FFF; // which is 0
                                       // Expected scale_down to mask out bit 15, resulting in scaling down 0
        assert_eq!(scale_down(val, 14, 7), scale_down(masked_val, 14, 7));

        // Let's also test a case where valid bits are set along with out-of-bounds bits
        let val2: u32 = 0x8000 | 0x2000; // Bit 15 set, Bit 13 set (8192)
        assert_eq!(scale_down(val2, 14, 7), scale_down(0x2000, 14, 7)); // 8192 scaled down from 14 to 7
    }
}
