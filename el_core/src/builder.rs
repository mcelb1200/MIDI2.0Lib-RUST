use crate::ump::Ump;
use crate::utils::split_14bit;

pub struct VoiceBuilder;

impl VoiceBuilder {
    #[must_use]
    pub fn midi1_note_on(group: u8, channel: u8, note: u8, velocity: u8) -> Ump {
        let w1 = 0x20900000
            | (u32::from(group) << 24)
            | (u32::from(channel) << 16)
            | (u32::from(note) << 8)
            | u32::from(velocity);
        Ump::new(w1, 0, 0, 0)
    }

    #[must_use]
    pub fn midi1_note_off(group: u8, channel: u8, note: u8, velocity: u8) -> Ump {
        let w1 = 0x20800000
            | (u32::from(group) << 24)
            | (u32::from(channel) << 16)
            | (u32::from(note) << 8)
            | u32::from(velocity);
        Ump::new(w1, 0, 0, 0)
    }

    #[must_use]
    pub fn midi1_cc(group: u8, channel: u8, index: u8, value: u8) -> Ump {
        let w1 = 0x20B00000
            | (u32::from(group) << 24)
            | (u32::from(channel) << 16)
            | (u32::from(index) << 8)
            | u32::from(value);
        Ump::new(w1, 0, 0, 0)
    }

    #[must_use]
    pub fn midi1_pitch_bend(group: u8, channel: u8, value: u16) -> Ump {
        let (msb, lsb) = split_14bit(value);
        let w1 = 0x20E00000
            | (u32::from(group) << 24)
            | (u32::from(channel) << 16)
            | (u32::from(lsb) << 8)
            | u32::from(msb);
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
            | (u32::from(group) << 24)
            | (u32::from(channel) << 16)
            | (u32::from(note) << 8)
            | u32::from(attr_type);
        let w2 = (u32::from(velocity) << 16) | u32::from(attr_data);
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
            | (u32::from(group) << 24)
            | (u32::from(channel) << 16)
            | (u32::from(note) << 8)
            | u32::from(attr_type);
        let w2 = (u32::from(velocity) << 16) | u32::from(attr_data);
        Ump::new(w1, w2, 0, 0)
    }

    #[must_use]
    pub fn midi2_cc(group: u8, channel: u8, index: u8, value: u32) -> Ump {
        let w1 = 0x40B00000
            | (u32::from(group) << 24)
            | (u32::from(channel) << 16)
            | (u32::from(index) << 8);
        Ump::new(w1, value, 0, 0)
    }

    #[must_use]
    pub fn midi2_pitch_bend(group: u8, channel: u8, value: u32) -> Ump {
        let w1 = 0x40E00000 | (u32::from(group) << 24) | (u32::from(channel) << 16);
        Ump::new(w1, value, 0, 0)
    }

    #[must_use]
    pub fn midi2_nrpn(group: u8, channel: u8, bank: u8, index: u8, value: u32) -> Ump {
        // NRPN Status = 0x02, Bank = Data Byte 1, Index = Data Byte 2
        let w1 = 0x40020000
            | (u32::from(group) << 24)
            | (u32::from(channel) << 16)
            | (u32::from(bank) << 8)
            | u32::from(index);
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
        let w1 = 0x00100000 | (u32::from(group) << 24) | u32::from(timestamp);
        Ump::new(w1, 0, 0, 0)
    }

    #[must_use]
    pub fn jitter_reduction_timestamp(group: u8, timestamp: u16) -> Ump {
        let w1 = 0x00200000 | (u32::from(group) << 24) | u32::from(timestamp);
        Ump::new(w1, 0, 0, 0)
    }
}
