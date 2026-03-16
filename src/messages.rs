use crate::ump::Ump;
use crate::utils::*;

/// Factory struct for creating Universal MIDI Packets (UMP).
///
/// This struct provides static methods to generate UMPs for various MIDI messages,
/// including Utility, System, MIDI 1.0 Channel Voice, and MIDI 2.0 Channel Voice messages.
pub struct UmpFactory;

impl UmpFactory {
    // Utility Messages (MT=0x0)

    /// Creates a No Operation (NOOP) Utility UMP.
    ///
    /// # Returns
    ///
    /// A `Ump` representing a NOOP message.
    #[must_use]
    pub fn noop() -> Ump {
        Ump::new()
    }

    /// Creates a Jitter Reduction Clock Utility UMP.
    ///
    /// # Arguments
    ///
    /// * `clock_time` - The 16-bit clock time value.
    ///
    /// # Returns
    ///
    /// A `Ump` containing the JR Clock message.
    #[must_use]
    pub fn jr_clock(clock_time: u16) -> Ump {
        let val = ((u32::from(UTILITY_JRCLOCK)) << 20) + (u32::from(clock_time));
        Ump {
            data: [val, 0, 0, 0],
        }
    }

    /// Creates a Jitter Reduction Timestamp Utility UMP.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The 16-bit timestamp value.
    ///
    /// # Returns
    ///
    /// A `Ump` containing the JR Timestamp message.
    #[must_use]
    pub fn jr_timestamp(timestamp: u16) -> Ump {
        let val = ((u32::from(UTILITY_JRTS)) << 20) + (u32::from(timestamp));
        Ump {
            data: [val, 0, 0, 0],
        }
    }

    // System Common / Realtime (MT=0x1)

    /// Helper to create a System Common or Realtime UMP (MT=0x1).
    #[must_use]
    fn mt1_create(group: u8, status: u8, val1: u8, val2: u8) -> Ump {
        let w = (((u32::from(UMP_SYSTEM)) << 28) + ((u32::from(group & 0xF)) << 24))
            + (((u32::from(status)) & 0xFF) << 16)
            + (((u32::from(val1)) & 0x7F) << 8)
            + ((u32::from(val2)) & 0x7F);
        Ump { data: [w, 0, 0, 0] }
    }

    /// Creates a Timing Clock UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Timing Clock message.
    #[must_use]
    pub fn timing_clock(group: u8) -> Ump {
        Self::mt1_create(group, TIMINGCLOCK, 0, 0)
    }

    /// Creates a Start Sequence UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Start message.
    #[must_use]
    pub fn start(group: u8) -> Ump {
        Self::mt1_create(group, SEQSTART, 0, 0)
    }

    /// Creates a Continue Sequence UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Continue message.
    #[must_use]
    pub fn continue_seq(group: u8) -> Ump {
        Self::mt1_create(group, SEQCONT, 0, 0)
    }

    /// Creates a Stop Sequence UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Stop message.
    #[must_use]
    pub fn stop(group: u8) -> Ump {
        Self::mt1_create(group, SEQSTOP, 0, 0)
    }

    /// Creates an Active Sensing UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    ///
    /// # Returns
    ///
    /// A `Ump` representing an Active Sensing message.
    #[must_use]
    pub fn active_sensing(group: u8) -> Ump {
        Self::mt1_create(group, ACTIVESENSE, 0, 0)
    }

    /// Creates a System Reset UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    ///
    /// # Returns
    ///
    /// A `Ump` representing a System Reset message.
    #[must_use]
    pub fn system_reset(group: u8) -> Ump {
        Self::mt1_create(group, SYSTEMRESET, 0, 0)
    }

    /// Creates a MIDI Time Code Quarter Frame UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `data` - The data byte for the quarter frame message.
    ///
    /// # Returns
    ///
    /// A `Ump` representing an MTC Quarter Frame message.
    #[must_use]
    pub fn mtc_quarter_frame(group: u8, data: u8) -> Ump {
        Self::mt1_create(group, TIMING_CODE, data, 0)
    }

    /// Creates a Song Position Pointer UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `position` - The 14-bit song position value.
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Song Position Pointer message.
    #[must_use]
    pub fn song_position_pointer(group: u8, position: u16) -> Ump {
        Self::mt1_create(
            group,
            SPP,
            (position & 0x7F) as u8,
            ((position >> 7) & 0x7F) as u8,
        )
    }

    /// Creates a Song Select UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `song` - The song number (0-127).
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Song Select message.
    #[must_use]
    pub fn song_select(group: u8, song: u8) -> Ump {
        Self::mt1_create(group, SONG_SELECT, song, 0)
    }

    /// Creates a Tune Request UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Tune Request message.
    #[must_use]
    pub fn tune_request(group: u8) -> Ump {
        Self::mt1_create(group, TUNEREQUEST, 0, 0)
    }

    // MIDI 1.0 Channel Voice (MT=0x2)

    /// Helper to create a MIDI 1.0 Channel Voice UMP (MT=0x2).
    #[must_use]
    fn mt2_create(group: u8, status: u8, channel: u8, val1: u8, val2: u8) -> Ump {
        let mut message = ((u32::from(UMP_M1CVM)) << 28) + ((u32::from(group & 0xF)) << 24);
        message += (u32::from((status & 0xF0) | (channel & 0xF))) << 16;
        message += (u32::from(val1 & 0x7F)) << 8;
        message += u32::from(val2 & 0x7F);
        Ump {
            data: [message, 0, 0, 0],
        }
    }

    /// Creates a MIDI 1.0 Note Off UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `note` - The note number (0-127).
    /// * `velocity` - The velocity (0-127).
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Note Off message.
    #[must_use]
    pub fn midi1_note_off(group: u8, channel: u8, note: u8, velocity: u8) -> Ump {
        Self::mt2_create(group, NOTE_OFF, channel, note, velocity)
    }

    /// Creates a MIDI 1.0 Note On UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `note` - The note number (0-127).
    /// * `velocity` - The velocity (0-127).
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Note On message.
    #[must_use]
    pub fn midi1_note_on(group: u8, channel: u8, note: u8, velocity: u8) -> Ump {
        Self::mt2_create(group, NOTE_ON, channel, note, velocity)
    }

    /// Creates a MIDI 1.0 Polyphonic Key Pressure (Aftertouch) UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `note` - The note number (0-127).
    /// * `pressure` - The pressure value (0-127).
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Polyphonic Key Pressure message.
    #[must_use]
    pub fn midi1_poly_pressure(group: u8, channel: u8, note: u8, pressure: u8) -> Ump {
        Self::mt2_create(group, KEY_PRESSURE, channel, note, pressure)
    }

    /// Creates a MIDI 1.0 Control Change UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `index` - The controller number (0-127).
    /// * `value` - The control value (0-127).
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Control Change message.
    #[must_use]
    pub fn midi1_control_change(group: u8, channel: u8, index: u8, value: u8) -> Ump {
        Self::mt2_create(group, CC, channel, index, value)
    }

    /// Creates a MIDI 1.0 Program Change UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `program` - The program number (0-127).
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Program Change message.
    #[must_use]
    pub fn midi1_program_change(group: u8, channel: u8, program: u8) -> Ump {
        Self::mt2_create(group, PROGRAM_CHANGE, channel, program, 0)
    }

    /// Creates a MIDI 1.0 Channel Pressure (Aftertouch) UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `pressure` - The pressure value (0-127).
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Channel Pressure message.
    #[must_use]
    pub fn midi1_channel_pressure(group: u8, channel: u8, pressure: u8) -> Ump {
        Self::mt2_create(group, CHANNEL_PRESSURE, channel, pressure, 0)
    }

    /// Creates a MIDI 1.0 Pitch Bend UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `value` - The 14-bit pitch bend value.
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Pitch Bend message.
    #[must_use]
    pub fn midi1_pitch_bend(group: u8, channel: u8, value: u16) -> Ump {
        Self::mt2_create(
            group,
            PITCH_BEND,
            channel,
            (value & 0x7F) as u8,
            ((value >> 7) & 0x7F) as u8,
        )
    }

    // MIDI 2.0 Channel Voice (MT=0x4)

    /// Helper to create the first word of a MIDI 2.0 Channel Voice UMP (MT=0x4).
    #[must_use]
    fn mt4_create_first_word(group: u8, status: u8, channel: u8, val1: u8, val2: u8) -> u32 {
        let mut message = ((u32::from(UMP_M2CVM)) << 28) + ((u32::from(group & 0xF)) << 24);
        message += (u32::from((status & 0xF0) | (channel & 0xF))) << 16;
        message += (u32::from(val1)) << 8;
        message += u32::from(val2);
        message
    }

    /// Creates a MIDI 2.0 Note Off UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `note` - The note number (0-127).
    /// * `attribute_type` - The attribute type.
    /// * `velocity` - The 16-bit velocity.
    /// * `attribute_data` - The 16-bit attribute data.
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Note Off message.
    #[must_use]
    pub fn midi2_note_off(
        group: u8,
        channel: u8,
        note: u8,
        attribute_type: u8,
        velocity: u16,
        attribute_data: u16,
    ) -> Ump {
        let word1 =
            Self::mt4_create_first_word(group, NOTE_OFF, channel, note & 0x7F, attribute_type);
        let word2 = ((velocity as u32) << 16) | (attribute_data as u32);
        Ump {
            data: [word1, word2, 0, 0],
        }
    }

    /// Creates a MIDI 2.0 Note On UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `note` - The note number (0-127).
    /// * `attribute_type` - The attribute type.
    /// * `velocity` - The 16-bit velocity.
    /// * `attribute_data` - The 16-bit attribute data.
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Note On message.
    #[must_use]
    pub fn midi2_note_on(
        group: u8,
        channel: u8,
        note: u8,
        attribute_type: u8,
        velocity: u16,
        attribute_data: u16,
    ) -> Ump {
        let word1 =
            Self::mt4_create_first_word(group, NOTE_ON, channel, note & 0x7F, attribute_type);
        let word2 = ((velocity as u32) << 16) | (attribute_data as u32);
        Ump {
            data: [word1, word2, 0, 0],
        }
    }

    /// Creates a MIDI 2.0 Polyphonic Key Pressure (Aftertouch) UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `note` - The note number (0-127).
    /// * `pressure` - The 32-bit pressure value.
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Polyphonic Key Pressure message.
    #[must_use]
    pub fn midi2_poly_pressure(group: u8, channel: u8, note: u8, pressure: u32) -> Ump {
        let word1 = Self::mt4_create_first_word(group, KEY_PRESSURE, channel, note & 0x7F, 0);
        Ump {
            data: [word1, pressure, 0, 0],
        }
    }

    /// Creates a MIDI 2.0 Control Change UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `index` - The controller number (0-127).
    /// * `value` - The 32-bit control value.
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Control Change message.
    #[must_use]
    pub fn midi2_control_change(group: u8, channel: u8, index: u8, value: u32) -> Ump {
        let word1 = Self::mt4_create_first_word(group, CC, channel, index & 0x7F, 0);
        Ump {
            data: [word1, value, 0, 0],
        }
    }

    /// Creates a MIDI 2.0 Registered Parameter Number (RPN) UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `bank` - The parameter bank.
    /// * `index` - The parameter index.
    /// * `value` - The 32-bit value.
    ///
    /// # Returns
    ///
    /// A `Ump` representing an RPN message.
    #[must_use]
    pub fn midi2_rpn(group: u8, channel: u8, bank: u8, index: u8, value: u32) -> Ump {
        let word1 = Self::mt4_create_first_word(group, RPN, channel, bank & 0x7F, index & 0x7F);
        Ump {
            data: [word1, value, 0, 0],
        }
    }

    /// Creates a MIDI 2.0 Non-Registered Parameter Number (NRPN) UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `bank` - The parameter bank.
    /// * `index` - The parameter index.
    /// * `value` - The 32-bit value.
    ///
    /// # Returns
    ///
    /// A `Ump` representing an NRPN message.
    #[must_use]
    pub fn midi2_nrpn(group: u8, channel: u8, bank: u8, index: u8, value: u32) -> Ump {
        let word1 = Self::mt4_create_first_word(group, NRPN, channel, bank & 0x7F, index & 0x7F);
        Ump {
            data: [word1, value, 0, 0],
        }
    }

    /// Creates a MIDI 2.0 Relative RPN UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `bank` - The parameter bank.
    /// * `index` - The parameter index.
    /// * `value` - The 32-bit relative value (signed).
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Relative RPN message.
    #[must_use]
    pub fn midi2_relative_rpn(group: u8, channel: u8, bank: u8, index: u8, value: i32) -> Ump {
        let word1 =
            Self::mt4_create_first_word(group, RPN_RELATIVE, channel, bank & 0x7F, index & 0x7F);
        Ump {
            data: [word1, value as u32, 0, 0],
        }
    }

    /// Creates a MIDI 2.0 Relative NRPN UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `bank` - The parameter bank.
    /// * `index` - The parameter index.
    /// * `value` - The 32-bit relative value (signed).
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Relative NRPN message.
    #[must_use]
    pub fn midi2_relative_nrpn(group: u8, channel: u8, bank: u8, index: u8, value: i32) -> Ump {
        let word1 =
            Self::mt4_create_first_word(group, NRPN_RELATIVE, channel, bank & 0x7F, index & 0x7F);
        Ump {
            data: [word1, value as u32, 0, 0],
        }
    }

    /// Creates a MIDI 2.0 Program Change UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `program` - The program number.
    /// * `bank_valid` - Whether the bank select is valid.
    /// * `bank` - The bank MSB.
    /// * `index` - The bank LSB.
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Program Change message.
    #[must_use]
    pub fn midi2_program_change(
        group: u8,
        channel: u8,
        program: u8,
        bank_valid: bool,
        bank: u8,
        index: u8,
    ) -> Ump {
        let word1 = Self::mt4_create_first_word(
            group,
            PROGRAM_CHANGE,
            channel,
            0,
            if bank_valid { 1 } else { 0 },
        );
        let word2 = (((program & 0x7F) as u32) << 24)
            + if bank_valid {
                (((bank & 0x7F) as u32) << 8) + ((index & 0x7F) as u32)
            } else {
                0
            };
        Ump {
            data: [word1, word2, 0, 0],
        }
    }

    /// Creates a MIDI 2.0 Channel Pressure (Aftertouch) UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `pressure` - The 32-bit pressure value.
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Channel Pressure message.
    #[must_use]
    pub fn midi2_channel_pressure(group: u8, channel: u8, pressure: u32) -> Ump {
        let word1 = Self::mt4_create_first_word(group, CHANNEL_PRESSURE, channel, 0, 0);
        Ump {
            data: [word1, pressure, 0, 0],
        }
    }

    /// Creates a MIDI 2.0 Pitch Bend UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `value` - The 32-bit pitch bend value.
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Pitch Bend message.
    #[must_use]
    pub fn midi2_pitch_bend(group: u8, channel: u8, value: u32) -> Ump {
        let word1 = Self::mt4_create_first_word(group, PITCH_BEND, channel, 0, 0);
        Ump {
            data: [word1, value, 0, 0],
        }
    }

    /// Creates a MIDI 2.0 Per-Note Pitch Bend UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `note` - The note number (0-127).
    /// * `value` - The 32-bit pitch bend value.
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Per-Note Pitch Bend message.
    #[must_use]
    pub fn midi2_per_note_pitch_bend(group: u8, channel: u8, note: u8, value: u32) -> Ump {
        let word1 = Self::mt4_create_first_word(group, PITCH_BEND_PERNOTE, channel, note & 0x7F, 0);
        Ump {
            data: [word1, value, 0, 0],
        }
    }
}
