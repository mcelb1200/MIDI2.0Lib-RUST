use crate::ump::Ump;
use crate::utils::{
    split_14bit, FLEXDATA_COMMON, FLEXDATA_COMMON_CHORD, FLEXDATA_COMMON_KEYSIG,
    FLEXDATA_COMMON_METRONOME, FLEXDATA_COMMON_TEMPO, FLEXDATA_COMMON_TIMESIG, FLEXDATA_LYRIC,
    FLEXDATA_PERFORMANCE,
};

pub struct VoiceBuilder;

impl VoiceBuilder {
    #[must_use]
    #[inline]
    pub fn midi1_note_on(group: u8, channel: u8, note: u8, velocity: u8) -> Ump {
        let combined = ((group as u32) << 24)
            | ((channel as u32) << 16)
            | ((note as u32) << 8)
            | (velocity as u32);
        let w1 = 0x20900000 | (combined & 0x0F0F7F7F);
        Ump::new(w1, 0, 0, 0)
    }

    #[must_use]
    #[inline]
    pub fn midi1_note_off(group: u8, channel: u8, note: u8, velocity: u8) -> Ump {
        let combined = ((group as u32) << 24)
            | ((channel as u32) << 16)
            | ((note as u32) << 8)
            | (velocity as u32);
        let w1 = 0x20800000 | (combined & 0x0F0F7F7F);
        Ump::new(w1, 0, 0, 0)
    }

    #[must_use]
    #[inline]
    pub fn midi1_cc(group: u8, channel: u8, index: u8, value: u8) -> Ump {
        let combined = ((group as u32) << 24)
            | ((channel as u32) << 16)
            | ((index as u32) << 8)
            | (value as u32);
        let w1 = 0x20B00000 | (combined & 0x0F0F7F7F);
        Ump::new(w1, 0, 0, 0)
    }

    #[must_use]
    #[inline]
    pub fn midi1_pitch_bend(group: u8, channel: u8, value: u16) -> Ump {
        let (msb, lsb) = split_14bit(value);
        let combined =
            ((group as u32) << 24) | ((channel as u32) << 16) | ((lsb as u32) << 8) | (msb as u32);
        let w1 = 0x20E00000 | (combined & 0x0F0F7F7F);
        Ump::new(w1, 0, 0, 0)
    }

    #[must_use]
    #[inline]
    pub fn midi2_note_on(
        group: u8,
        channel: u8,
        note: u8,
        attr_type: u8,
        velocity: u16,
        attr_data: u16,
    ) -> Ump {
        let combined = ((group as u32) << 24)
            | ((channel as u32) << 16)
            | ((note as u32) << 8)
            | (attr_type as u32);
        let w1 = 0x40900000 | (combined & 0x0F0F7FFF);
        let w2 = ((velocity as u32) << 16) | (attr_data as u32);
        Ump::new(w1, w2, 0, 0)
    }

    #[must_use]
    #[inline]
    pub fn midi2_note_off(
        group: u8,
        channel: u8,
        note: u8,
        attr_type: u8,
        velocity: u16,
        attr_data: u16,
    ) -> Ump {
        let combined = ((group as u32) << 24)
            | ((channel as u32) << 16)
            | ((note as u32) << 8)
            | (attr_type as u32);
        let w1 = 0x40800000 | (combined & 0x0F0F7FFF);
        let w2 = ((velocity as u32) << 16) | (attr_data as u32);
        Ump::new(w1, w2, 0, 0)
    }

    #[must_use]
    #[inline]
    pub fn midi2_cc(group: u8, channel: u8, index: u8, value: u32) -> Ump {
        let combined = ((group as u32) << 24) | ((channel as u32) << 16) | ((index as u32) << 8);
        let w1 = 0x40B00000 | (combined & 0x0F0F7F00);
        Ump::new(w1, value, 0, 0)
    }

    #[must_use]
    #[inline]
    pub fn midi2_pitch_bend(group: u8, channel: u8, value: u32) -> Ump {
        let combined = ((group as u32) << 24) | ((channel as u32) << 16);
        let w1 = 0x40E00000 | (combined & 0x0F0F0000);
        Ump::new(w1, value, 0, 0)
    }

    #[must_use]
    #[inline]
    pub fn midi2_nrpn(group: u8, channel: u8, bank: u8, index: u8, value: u32) -> Ump {
        // NRPN Status = 0x02, Bank = Data Byte 1, Index = Data Byte 2
        let combined = ((group as u32) << 24)
            | ((channel as u32) << 16)
            | ((bank as u32) << 8)
            | (index as u32);
        let w1 = 0x40020000 | (combined & 0x0F0F7F7F);
        Ump::new(w1, value, 0, 0)
    }

    #[must_use]
    #[inline]
    pub fn midi2_per_note_pitch_bend(group: u8, channel: u8, note: u8, value: u32) -> Ump {
        let combined = ((group as u32) << 24) | ((channel as u32) << 16) | ((note as u32) << 8);
        let w1 = 0x40600000 | (combined & 0x0F0F7F00);
        Ump::new(w1, value, 0, 0)
    }

    #[must_use]
    #[inline]
    pub fn midi2_per_note_cc(group: u8, channel: u8, note: u8, index: u8, value: u32) -> Ump {
        let combined = ((group as u32) << 24)
            | ((channel as u32) << 16)
            | ((note as u32) << 8)
            | (index as u32);
        let w1 = 0x40100000 | (combined & 0x0F0F7FFF);
        Ump::new(w1, value, 0, 0)
    }

    #[must_use]
    #[inline]
    pub fn midi2_per_note_rpn(group: u8, channel: u8, note: u8, index: u8, value: u32) -> Ump {
        let combined = ((group as u32) << 24)
            | ((channel as u32) << 16)
            | ((note as u32) << 8)
            | (index as u32);
        let w1 = 0x40000000 | (combined & 0x0F0F7FFF);
        Ump::new(w1, value, 0, 0)
    }

    #[must_use]
    #[inline]
    pub fn midi2_per_note_manage(group: u8, channel: u8, note: u8, option_flags: u8) -> Ump {
        let combined = ((group as u32) << 24)
            | ((channel as u32) << 16)
            | ((note as u32) << 8)
            | (option_flags as u32);
        let w1 = 0x40F00000 | (combined & 0x0F0F7FFF);
        Ump::new(w1, 0, 0, 0)
    }
}

pub struct UtilityBuilder;

impl UtilityBuilder {
    #[must_use]
    #[inline]
    pub fn noop() -> Ump {
        Ump::new(0, 0, 0, 0)
    }

    #[must_use]
    #[inline]
    pub fn jitter_reduction_clock(group: u8, timestamp: u16) -> Ump {
        let combined = ((group as u32) << 24) | (timestamp as u32);
        let w1 = 0x00100000 | (combined & 0x0F00FFFF);
        Ump::new(w1, 0, 0, 0)
    }

    #[must_use]
    #[inline]
    pub fn jitter_reduction_timestamp(group: u8, timestamp: u16) -> Ump {
        let combined = ((group as u32) << 24) | (timestamp as u32);
        let w1 = 0x00200000 | (combined & 0x0F00FFFF);
        Ump::new(w1, 0, 0, 0)
    }
}

pub struct FlexBuilder;

impl FlexBuilder {
    /// Builds word 0 header for Flex Data message (MT=0xD).
    #[must_use]
    #[inline]
    pub fn header(group: u8, form: u8, addrs: u8, channel: u8, status_bank: u8, status: u8) -> u32 {
        let combined = ((group as u32) << 24)
            | (((form as u32) & 0x3) << 22)
            | (((addrs as u32) & 0x3) << 20)
            | (((channel as u32) & 0xF) << 16)
            | ((status_bank as u32) << 8)
            | (status as u32);
        0xD000_0000 | (combined & 0x0FFF_FFFF)
    }

    /// Generic Flex Data UMP builder.
    #[must_use]
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn flex_data(
        group: u8,
        form: u8,
        addrs: u8,
        channel: u8,
        status_bank: u8,
        status: u8,
        data1: u32,
        data2: u32,
        data3: u32,
    ) -> Ump {
        let w0 = Self::header(group, form, addrs, channel, status_bank, status);
        Ump::new(w0, data1, data2, data3)
    }

    /// Creates a Flex Data Set Tempo message.
    #[must_use]
    #[inline]
    pub fn tempo(group: u8, channel: u8, form: u8, addrs: u8, num_10ns_pqn: u32) -> Ump {
        Self::flex_data(
            group,
            form,
            addrs,
            channel,
            FLEXDATA_COMMON,
            FLEXDATA_COMMON_TEMPO,
            num_10ns_pqn,
            0,
            0,
        )
    }

    /// Creates a Flex Data Set Time Signature message.
    #[must_use]
    #[inline]
    pub fn time_sig(
        group: u8,
        channel: u8,
        form: u8,
        addrs: u8,
        numerator: u8,
        denominator: u8,
        num_32_notes: u8,
    ) -> Ump {
        let word1 = ((numerator as u32) << 24)
            | ((denominator as u32) << 16)
            | ((num_32_notes as u32) << 8);
        Self::flex_data(
            group,
            form,
            addrs,
            channel,
            FLEXDATA_COMMON,
            FLEXDATA_COMMON_TIMESIG,
            word1,
            0,
            0,
        )
    }

    /// Creates a Flex Data Set Metronome message.
    #[must_use]
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn metronome(
        group: u8,
        channel: u8,
        form: u8,
        addrs: u8,
        num_clk_pri_cli: u8,
        b_acc_p1: u8,
        b_acc_p2: u8,
        b_acc_p3: u8,
        num_sub_div_cli1: u8,
        num_sub_div_cli2: u8,
    ) -> Ump {
        let word1 = ((num_clk_pri_cli as u32) << 24)
            | ((b_acc_p1 as u32) << 16)
            | ((b_acc_p2 as u32) << 8)
            | (b_acc_p3 as u32);
        let word2 = ((num_sub_div_cli1 as u32) << 24) | ((num_sub_div_cli2 as u32) << 16);
        Self::flex_data(
            group,
            form,
            addrs,
            channel,
            FLEXDATA_COMMON,
            FLEXDATA_COMMON_METRONOME,
            word1,
            word2,
            0,
        )
    }

    /// Creates a Flex Data Set Key Signature message.
    #[must_use]
    #[inline]
    pub fn key_sig(
        group: u8,
        channel: u8,
        form: u8,
        addrs: u8,
        sharps_flats: u8,
        tonic: u8,
    ) -> Ump {
        let word1 = ((sharps_flats as u32) << 24) | ((tonic as u32) << 16);
        Self::flex_data(
            group,
            form,
            addrs,
            channel,
            FLEXDATA_COMMON,
            FLEXDATA_COMMON_KEYSIG,
            word1,
            0,
            0,
        )
    }

    /// Creates a Flex Data Set Chord message.
    #[must_use]
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn chord(
        group: u8,
        channel: u8,
        form: u8,
        addrs: u8,
        ch_shrp_flt: u8,
        ch_tonic: u8,
        ch_type: u8,
        ch_alt1_type: u8,
        ch_alt1_deg: u8,
        ch_alt2_type: u8,
        ch_alt2_deg: u8,
        ch_alt3_type: u8,
        ch_alt3_deg: u8,
        ch_alt4_type: u8,
        ch_alt4_deg: u8,
        ba_shrp_flt: u8,
        ba_tonic: u8,
        ba_type: u8,
        ba_alt1_type: u8,
        ba_alt1_deg: u8,
        ba_alt2_type: u8,
        ba_alt2_deg: u8,
    ) -> Ump {
        let word1 = (((ch_shrp_flt & 0xF) as u32) << 28)
            | (((ch_tonic & 0xF) as u32) << 24)
            | ((ch_type as u32) << 16)
            | (((ch_alt1_type & 0xF) as u32) << 12)
            | (((ch_alt1_deg & 0xF) as u32) << 8)
            | (((ch_alt2_type & 0xF) as u32) << 4)
            | ((ch_alt2_deg & 0xF) as u32);

        let word2 = (((ch_alt3_type & 0xF) as u32) << 28)
            | (((ch_alt3_deg & 0xF) as u32) << 24)
            | (((ch_alt4_type & 0xF) as u32) << 20)
            | (((ch_alt4_deg & 0xF) as u32) << 16);

        let word3 = (((ba_shrp_flt & 0xF) as u32) << 28)
            | (((ba_tonic & 0xF) as u32) << 24)
            | ((ba_type as u32) << 16)
            | (((ba_alt1_type & 0xF) as u32) << 12)
            | (((ba_alt1_deg & 0xF) as u32) << 8)
            | (((ba_alt2_type & 0xF) as u32) << 4)
            | ((ba_alt2_deg & 0xF) as u32);

        Self::flex_data(
            group,
            form,
            addrs,
            channel,
            FLEXDATA_COMMON,
            FLEXDATA_COMMON_CHORD,
            word1,
            word2,
            word3,
        )
    }

    /// Creates a Flex Data Text message carrying up to 12 text bytes across words 1..3.
    #[must_use]
    #[inline]
    pub fn text(
        group: u8,
        channel: u8,
        form: u8,
        addrs: u8,
        status_bank: u8,
        status: u8,
        text: &[u8],
    ) -> Ump {
        let mut words = [0_u32; 3];
        let mut offset = 0;
        let text_len = text.len();

        for word in &mut words {
            for b in (0..4).rev() {
                if offset < text_len {
                    *word |= (text[offset] as u32) << (b * 8);
                    offset += 1;
                }
            }
        }

        Self::flex_data(
            group,
            form,
            addrs,
            channel,
            status_bank,
            status,
            words[0],
            words[1],
            words[2],
        )
    }

    /// Creates a Flex Data Performance Text message (Status Bank = 0x01).
    #[must_use]
    #[inline]
    pub fn performance(
        group: u8,
        channel: u8,
        form: u8,
        addrs: u8,
        status: u8,
        text: &[u8],
    ) -> Ump {
        Self::text(
            group,
            channel,
            form,
            addrs,
            FLEXDATA_PERFORMANCE,
            status,
            text,
        )
    }

    /// Creates a Flex Data Lyric Text message (Status Bank = 0x02).
    #[must_use]
    #[inline]
    pub fn lyric(group: u8, channel: u8, form: u8, addrs: u8, status: u8, text: &[u8]) -> Ump {
        Self::text(group, channel, form, addrs, FLEXDATA_LYRIC, status, text)
    }
}
