use crate::ump::Ump;
use crate::utils::split_14bit;

pub struct VoiceBuilder;

impl VoiceBuilder {
    #[must_use]
    pub fn midi1_note_on(group: u8, channel: u8, note: u8, velocity: u8) -> Ump {
        let w1 = 0x20900000
            | ((group as u32) << 24)
            | ((channel as u32) << 16)
            | ((note as u32) << 8)
            | (velocity as u32);
        Ump::new(w1, 0, 0, 0)
    }

    #[must_use]
    pub fn midi1_note_off(group: u8, channel: u8, note: u8, velocity: u8) -> Ump {
        let w1 = 0x20800000
            | ((group as u32) << 24)
            | ((channel as u32) << 16)
            | ((note as u32) << 8)
            | (velocity as u32);
        Ump::new(w1, 0, 0, 0)
    }

    #[must_use]
    pub fn midi1_cc(group: u8, channel: u8, index: u8, value: u8) -> Ump {
        let w1 = 0x20B00000
            | ((group as u32) << 24)
            | ((channel as u32) << 16)
            | ((index as u32) << 8)
            | (value as u32);
        Ump::new(w1, 0, 0, 0)
    }

    #[must_use]
    pub fn midi1_pitch_bend(group: u8, channel: u8, value: u16) -> Ump {
        let (msb, lsb) = split_14bit(value);
        let w1 = 0x20E00000
            | ((group as u32) << 24)
            | ((channel as u32) << 16)
            | ((lsb as u32) << 8)
            | (msb as u32);
        Ump::new(w1, 0, 0, 0)
    }

    #[must_use]
    pub fn midi2_note_on(
        group: u8,
        channel: u8,
        note: u8,
        attr_type: u8,
        velocity: u16,
        attr_data: u16,
    ) -> Ump {
        let w1 = 0x40900000
            | ((group as u32) << 24)
            | ((channel as u32) << 16)
            | ((note as u32) << 8)
            | (attr_type as u32);
        let w2 = ((velocity as u32) << 16) | (attr_data as u32);
        Ump::new(w1, w2, 0, 0)
    }

    #[must_use]
    pub fn midi2_note_off(
        group: u8,
        channel: u8,
        note: u8,
        attr_type: u8,
        velocity: u16,
        attr_data: u16,
    ) -> Ump {
        let w1 = 0x40800000
            | ((group as u32) << 24)
            | ((channel as u32) << 16)
            | ((note as u32) << 8)
            | (attr_type as u32);
        let w2 = ((velocity as u32) << 16) | (attr_data as u32);
        Ump::new(w1, w2, 0, 0)
    }

    #[must_use]
    pub fn midi2_cc(group: u8, channel: u8, index: u8, value: u32) -> Ump {
        let w1 =
            0x40B00000 | ((group as u32) << 24) | ((channel as u32) << 16) | ((index as u32) << 8);
        Ump::new(w1, value, 0, 0)
    }

    #[must_use]
    pub fn midi2_pitch_bend(group: u8, channel: u8, value: u32) -> Ump {
        let w1 = 0x40E00000 | ((group as u32) << 24) | ((channel as u32) << 16);
        Ump::new(w1, value, 0, 0)
    }

    #[must_use]
    pub fn midi2_nrpn(group: u8, channel: u8, bank: u8, index: u8, value: u32) -> Ump {
        // NRPN Status = 0x02, Bank = Data Byte 1, Index = Data Byte 2
        let w1 = 0x40020000
            | ((group as u32) << 24)
            | ((channel as u32) << 16)
            | ((bank as u32) << 8)
            | (index as u32);
        Ump::new(w1, value, 0, 0)
    }
}

pub struct UtilityBuilder;

impl UtilityBuilder {
    #[must_use]
    pub fn noop() -> Ump {
        Ump::new(0, 0, 0, 0)
    }

    #[must_use]
    pub fn jitter_reduction_clock(group: u8, timestamp: u16) -> Ump {
        let w1 = 0x00100000 | ((group as u32) << 24) | (timestamp as u32);
        Ump::new(w1, 0, 0, 0)
    }

    #[must_use]
    pub fn jitter_reduction_timestamp(group: u8, timestamp: u16) -> Ump {
        let w1 = 0x00200000 | ((group as u32) << 24) | (timestamp as u32);
        Ump::new(w1, 0, 0, 0)
    }
}
