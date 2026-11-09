use crate::ump::Ump;
use crate::utils::{
    split_14bit, FLEXDATA_CLIP_NAME, FLEXDATA_COMMON, FLEXDATA_COMMON_BAR_MARKER,
    FLEXDATA_COMMON_BEAT_MARKER, FLEXDATA_COMMON_CHORD, FLEXDATA_COMMON_KEYSIG,
    FLEXDATA_COMMON_METRONOME, FLEXDATA_COMMON_TEMPO, FLEXDATA_COMMON_TIMESIG,
    FLEXDATA_COMPOSITION_NAME, FLEXDATA_COPYRIGHT_NOTICE, FLEXDATA_LYRIC,
    FLEXDATA_METADATA_PROJECT, FLEXDATA_METADATA_TRACK, FLEXDATA_PERFORMANCE,
    FLEXDATA_PROJECT_NAME, FLEXDATA_TRACK_NAME,
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

pub struct SystemBuilder;

impl SystemBuilder {
    #[must_use]
    #[inline]
    fn mt1_create(group: u8, status: u8, val1: u8, val2: u8) -> Ump {
        let combined =
            ((group as u32) << 24) | ((status as u32) << 16) | ((val1 as u32) << 8) | (val2 as u32);
        let w0 = 0x1000_0000 | (combined & 0x0FFF_7F7F);
        Ump::new(w0, 0, 0, 0)
    }

    /// Creates a Timing Clock message (0xF8).
    #[must_use]
    #[inline]
    pub fn timing_clock(group: u8) -> Ump {
        Self::mt1_create(group, 0xF8, 0, 0)
    }

    /// Creates a Start Sequence message (0xFA).
    #[must_use]
    #[inline]
    pub fn start(group: u8) -> Ump {
        Self::mt1_create(group, 0xFA, 0, 0)
    }

    /// Creates a Continue Sequence message (0xFB).
    #[must_use]
    #[inline]
    pub fn continue_seq(group: u8) -> Ump {
        Self::mt1_create(group, 0xFB, 0, 0)
    }

    /// Creates a Stop Sequence message (0xFC).
    #[must_use]
    #[inline]
    pub fn stop(group: u8) -> Ump {
        Self::mt1_create(group, 0xFC, 0, 0)
    }

    /// Creates an Active Sensing message (0xFE).
    #[must_use]
    #[inline]
    pub fn active_sensing(group: u8) -> Ump {
        Self::mt1_create(group, 0xFE, 0, 0)
    }

    /// Creates a System Reset message (0xFF).
    #[must_use]
    #[inline]
    pub fn system_reset(group: u8) -> Ump {
        Self::mt1_create(group, 0xFF, 0, 0)
    }

    /// Creates a MIDI Time Code Quarter Frame message (0xF1).
    #[must_use]
    #[inline]
    pub fn mtc_quarter_frame(group: u8, data: u8) -> Ump {
        Self::mt1_create(group, 0xF1, data, 0)
    }

    /// Creates a Song Position Pointer message (0xF2).
    #[must_use]
    #[inline]
    pub fn song_position_pointer(group: u8, position: u16) -> Ump {
        Self::mt1_create(
            group,
            0xF2,
            (position & 0x7F) as u8,
            ((position >> 7) & 0x7F) as u8,
        )
    }

    /// Creates a Song Select message (0xF3).
    #[must_use]
    #[inline]
    pub fn song_select(group: u8, song: u8) -> Ump {
        Self::mt1_create(group, 0xF3, song, 0)
    }

    /// Creates a Tune Request message (0xF6).
    #[must_use]
    #[inline]
    pub fn tune_request(group: u8) -> Ump {
        Self::mt1_create(group, 0xF6, 0, 0)
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

    /// Creates a Delta Clockstamp Ticks message (ticks per quarter note, Status=0x3).
    #[must_use]
    #[inline]
    pub fn delta_clockstamp_ticks(ticks_per_qtr_note: u16) -> Ump {
        let w1 = 0x00300000 | (ticks_per_qtr_note as u32);
        Ump::new(w1, 0, 0, 0)
    }

    /// Creates a Delta Clockstamp Since Last message (Status=0x4).
    #[must_use]
    #[inline]
    pub fn delta_clockstamp_since(ticks_since_last: u16) -> Ump {
        let w1 = 0x00400000 | (ticks_since_last as u32);
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

    /// Creates a Flex Data Bar Marker message (Status Bank 0x00, Status 0x07).
    #[must_use]
    #[inline]
    pub fn bar_marker(group: u8, channel: u8, form: u8, addrs: u8, bar_number: u32) -> Ump {
        Self::flex_data(
            group,
            form,
            addrs,
            channel,
            FLEXDATA_COMMON,
            FLEXDATA_COMMON_BAR_MARKER,
            bar_number,
            0,
            0,
        )
    }

    /// Creates a Flex Data Beat Marker message (Status Bank 0x00, Status 0x08).
    #[must_use]
    #[inline]
    pub fn beat_marker(group: u8, channel: u8, form: u8, addrs: u8, beat_number: u32) -> Ump {
        Self::flex_data(
            group,
            form,
            addrs,
            channel,
            FLEXDATA_COMMON,
            FLEXDATA_COMMON_BEAT_MARKER,
            beat_number,
            0,
            0,
        )
    }

    /// Creates a Flex Data Project Metadata text message (Status Bank 0x03).
    #[must_use]
    #[inline]
    pub fn project_metadata(
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
            FLEXDATA_METADATA_PROJECT,
            status,
            text,
        )
    }

    /// Creates a Flex Data Project Name message (Status Bank 0x03, Status 0x00).
    #[must_use]
    #[inline]
    pub fn project_name(group: u8, channel: u8, form: u8, addrs: u8, name: &[u8]) -> Ump {
        Self::project_metadata(group, channel, form, addrs, FLEXDATA_PROJECT_NAME, name)
    }

    /// Creates a Flex Data Composition Name message (Status Bank 0x03, Status 0x01).
    #[must_use]
    #[inline]
    pub fn flex_composition_name(group: u8, channel: u8, form: u8, addrs: u8, name: &[u8]) -> Ump {
        Self::project_metadata(group, channel, form, addrs, FLEXDATA_COMPOSITION_NAME, name)
    }

    /// Creates a Flex Data Copyright Notice message (Status Bank 0x03, Status 0x04).
    #[must_use]
    #[inline]
    pub fn copyright_notice(group: u8, channel: u8, form: u8, addrs: u8, notice: &[u8]) -> Ump {
        Self::project_metadata(
            group,
            channel,
            form,
            addrs,
            FLEXDATA_COPYRIGHT_NOTICE,
            notice,
        )
    }

    /// Creates a Flex Data Track Metadata text message (Status Bank 0x04).
    #[must_use]
    #[inline]
    pub fn track_metadata(
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
            FLEXDATA_METADATA_TRACK,
            status,
            text,
        )
    }

    /// Creates a Flex Data Track Name message (Status Bank 0x04, Status 0x00).
    #[must_use]
    #[inline]
    pub fn track_name(group: u8, channel: u8, form: u8, addrs: u8, name: &[u8]) -> Ump {
        Self::track_metadata(group, channel, form, addrs, FLEXDATA_TRACK_NAME, name)
    }

    /// Creates a Flex Data Clip Name message (Status Bank 0x04, Status 0x02).
    #[must_use]
    #[inline]
    pub fn clip_name(group: u8, channel: u8, form: u8, addrs: u8, name: &[u8]) -> Ump {
        Self::track_metadata(group, channel, form, addrs, FLEXDATA_CLIP_NAME, name)
    }
}

pub struct StreamBuilder;

impl StreamBuilder {
    /// Creates an Endpoint Discovery UMP message.
    #[must_use]
    #[inline]
    pub fn endpoint_discovery(ump_version_major: u8, ump_version_minor: u8, filter: u8) -> Ump {
        let w0 = (0xF_u32 << 28) | ((ump_version_major as u32) << 8) | (ump_version_minor as u32);
        let w1 = (filter as u32) << 24;
        Ump::new(w0, w1, 0, 0)
    }

    /// Creates a Function Block Discovery UMP message (Status=0x010).
    #[must_use]
    #[inline]
    pub fn function_block_discovery(fb_idx: u8, filter: u8) -> Ump {
        let w0 = (0xF_u32 << 28) | (0x010_u32 << 16) | ((fb_idx as u32) << 8) | (filter as u32);
        Ump::new(w0, 0, 0, 0)
    }

    /// Creates a Function Block Info Notification UMP message (Status=0x011).
    #[must_use]
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn function_block_info_notify(
        fb_idx: u8,
        active: bool,
        direction: u8,
        sender: bool,
        receiver: bool,
        first_group: u8,
        group_length: u8,
        midi_ci_version: u8,
        is_midi1: u8,
        max_s8_streams: u8,
    ) -> Ump {
        let w0 = (0xF_u32 << 28)
            | (0x011_u32 << 16)
            | ((active as u32) << 15)
            | (((fb_idx & 0x7F) as u32) << 8)
            | ((sender as u32) << 5)
            | ((receiver as u32) << 4)
            | (((is_midi1 & 0x3) as u32) << 2)
            | ((direction & 0x3) as u32);
        let w1 = (((first_group & 0x1F) as u32) << 24)
            | (((group_length & 0x1F) as u32) << 16)
            | (((midi_ci_version & 0x7F) as u32) << 8)
            | (max_s8_streams as u32);
        Ump::new(w0, w1, 0, 0)
    }
}
