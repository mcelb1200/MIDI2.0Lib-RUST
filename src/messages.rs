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

    /// Creates a Delta Clockstamp Ticks Utility UMP (ticks per quarter note).
    ///
    /// # Arguments
    ///
    /// * `ticks_per_qtr_note` - The 16-bit ticks per quarter note.
    ///
    /// # Returns
    ///
    /// A `Ump` containing the Delta Clockstamp Ticks message.
    #[must_use]
    pub fn delta_clockstamp_ticks(ticks_per_qtr_note: u16) -> Ump {
        let val = ((u32::from(UTILITY_DELTACLOCKTICK)) << 20) + (u32::from(ticks_per_qtr_note));
        Ump {
            data: [val, 0, 0, 0],
        }
    }

    /// Creates a Delta Clockstamp Since Last Event Utility UMP.
    ///
    /// # Arguments
    ///
    /// * `ticks_since_last` - The 16-bit tick count since the last event.
    ///
    /// # Returns
    ///
    /// A `Ump` containing the Delta Clockstamp Since Last message.
    #[must_use]
    pub fn delta_clockstamp_since(ticks_since_last: u16) -> Ump {
        let val = ((u32::from(UTILITY_DELTACLOCKSINCE)) << 20) + (u32::from(ticks_since_last));
        Ump {
            data: [val, 0, 0, 0],
        }
    }

    // System Common / Realtime (MT=0x1)

    /// Helper to create a System Common or Realtime UMP (MT=0x1).
    #[must_use]
    #[inline]
    fn mt1_create(group: u8, status: u8, val1: u8, val2: u8) -> Ump {
        // ⚡ Bolt Optimization: Use a single bitwise combination and global mask.
        let combined =
            ((group as u32) << 24) | ((status as u32) << 16) | ((val1 as u32) << 8) | (val2 as u32);

        Ump {
            data: [
                ((u32::from(UMP_SYSTEM)) << 28) | (combined & 0x0FFF7F7F),
                0,
                0,
                0,
            ],
        }
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
        // ⚡ Bolt Optimization: Use a single bitwise combination and global mask.
        // Instead of masking individual arguments (e.g., `(group & 0xF) << 24`) and accumulating
        // with addition, combine all unmasked values via bitwise OR and apply one global mask
        // at the end. This reduces instruction count and improves message builder speed.
        // Note: status and channel share the same byte and must still be correctly interleaved.
        let combined = ((group as u32) << 24)
            | ((((status & 0xF0) | (channel & 0xF)) as u32) << 16)
            | ((val1 as u32) << 8)
            | (val2 as u32);

        Ump {
            data: [
                ((u32::from(UMP_M1CVM)) << 28) | (combined & 0x0FFF7F7F),
                0,
                0,
                0,
            ],
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
        // ⚡ Bolt Optimization: Use a single bitwise combination and global mask.
        let combined = ((group as u32) << 24)
            | ((((status & 0xF0) | (channel & 0xF)) as u32) << 16)
            | ((val1 as u32) << 8)
            | (val2 as u32);

        ((u32::from(UMP_M2CVM)) << 28) | (combined & 0x0FFFFFFF)
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
    #[inline]
    pub fn midi2_per_note_pitch_bend(group: u8, channel: u8, note: u8, value: u32) -> Ump {
        let word1 = Self::mt4_create_first_word(group, PITCH_BEND_PERNOTE, channel, note & 0x7F, 0);
        Ump {
            data: [word1, value, 0, 0],
        }
    }

    /// Creates a MIDI 2.0 Per-Note Controller (CC / Assignable Per-Note) UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `note` - The note number (0-127).
    /// * `index` - The controller index (0-255).
    /// * `value` - The 32-bit data value.
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Per-Note CC message.
    #[must_use]
    #[inline]
    pub fn midi2_per_note_cc(group: u8, channel: u8, note: u8, index: u8, value: u32) -> Ump {
        let word1 = Self::mt4_create_first_word(group, NRPN_PERNOTE, channel, note & 0x7F, index);
        Ump {
            data: [word1, value, 0, 0],
        }
    }

    /// Creates a MIDI 2.0 Per-Note Registered Parameter Number (RPN) UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `note` - The note number (0-127).
    /// * `index` - The RPN index (0-255).
    /// * `value` - The 32-bit data value.
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Per-Note RPN message.
    #[must_use]
    #[inline]
    pub fn midi2_per_note_rpn(group: u8, channel: u8, note: u8, index: u8, value: u32) -> Ump {
        let word1 = Self::mt4_create_first_word(group, RPN_PERNOTE, channel, note & 0x7F, index);
        Ump {
            data: [word1, value, 0, 0],
        }
    }

    /// Creates a MIDI 2.0 Per-Note Management UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The destination group (0-15).
    /// * `channel` - The destination channel (0-15).
    /// * `note` - The note number (0-127).
    /// * `option_flags` - The option flags (bit 0 = DETACH, bit 1 = RESET).
    ///
    /// # Returns
    ///
    /// A `Ump` representing a Per-Note Management message.
    #[must_use]
    #[inline]
    pub fn midi2_per_note_manage(group: u8, channel: u8, note: u8, option_flags: u8) -> Ump {
        let word1 =
            Self::mt4_create_first_word(group, PERNOTE_MANAGE, channel, note & 0x7F, option_flags);
        Ump {
            data: [word1, 0, 0, 0],
        }
    }

    // Flex Data Messages (MT=0xD)

    /// Helper to create the header (Word 0) of a Flex Data UMP (MT=0xD).
    ///
    /// Bits:
    /// - [31:28] Message Type (0xD)
    /// - [27:24] Group (4 bits)
    /// - [23:22] Form (2 bits)
    /// - [21:20] Address (2 bits)
    /// - [19:16] Channel (4 bits)
    /// - [15:8]  Status Bank (8 bits)
    /// - [7:0]   Status (8 bits)
    #[must_use]
    #[inline]
    pub fn flex_header(
        group: u8,
        form: u8,
        addrs: u8,
        channel: u8,
        status_bank: u8,
        status: u8,
    ) -> u32 {
        let combined = ((group as u32) << 24)
            | (((form as u32) & 0x3) << 22)
            | (((addrs as u32) & 0x3) << 20)
            | (((channel as u32) & 0xF) << 16)
            | ((status_bank as u32) << 8)
            | (status as u32);
        ((u32::from(UMP_FLEX_DATA)) << 28) | (combined & 0x0FFFFFFF)
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
        let word0 = Self::flex_header(group, form, addrs, channel, status_bank, status);
        Ump {
            data: [word0, data1, data2, data3],
        }
    }

    /// Creates a Flex Data Set Tempo UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - Destination group (0-15).
    /// * `channel` - Destination channel (0-15).
    /// * `form` - Multi-packet format (0-3).
    /// * `addrs` - Target address (0: Channel, 1: Group, 2: Complete Endpoint).
    /// * `num_10ns_pqn` - Number of 10-nanosecond units per primary quarter note.
    #[must_use]
    #[inline]
    pub fn flex_tempo(group: u8, channel: u8, form: u8, addrs: u8, num_10ns_pqn: u32) -> Ump {
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

    /// Creates a Flex Data Set Time Signature UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - Destination group (0-15).
    /// * `channel` - Destination channel (0-15).
    /// * `form` - Multi-packet format (0-3).
    /// * `addrs` - Target address (0: Channel, 1: Group, 2: Complete Endpoint).
    /// * `numerator` - Time signature numerator.
    /// * `denominator` - Time signature denominator (negative power of 2, e.g. 2 for quarter note).
    /// * `num_32_notes` - Number of 1/32 notes in a beat.
    #[must_use]
    #[inline]
    pub fn flex_time_sig(
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

    /// Creates a Flex Data Set Metronome UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - Destination group (0-15).
    /// * `channel` - Destination channel (0-15).
    /// * `form` - Multi-packet format (0-3).
    /// * `addrs` - Target address (0: Channel, 1: Group, 2: Complete Endpoint).
    /// * `num_clk_pri_cli` - Number of clocks per primary click.
    /// * `b_acc_p1` - Bar accent part 1.
    /// * `b_acc_p2` - Bar accent part 2.
    /// * `b_acc_p3` - Bar accent part 3.
    /// * `num_sub_div_cli1` - Number of subdivision clicks 1.
    /// * `num_sub_div_cli2` - Number of subdivision clicks 2.
    #[must_use]
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn flex_metronome(
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

    /// Creates a Flex Data Set Key Signature UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - Destination group (0-15).
    /// * `channel` - Destination channel (0-15).
    /// * `form` - Multi-packet format (0-3).
    /// * `addrs` - Target address (0: Channel, 1: Group, 2: Complete Endpoint).
    /// * `sharps_flats` - Number of sharps (>0) or flats (<0, signed 2s complement).
    /// * `tonic` - Tonic note.
    #[must_use]
    #[inline]
    pub fn flex_key_sig(
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

    /// Creates a Flex Data Set Chord UMP.
    #[must_use]
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn flex_chord(
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

    /// Creates a Flex Data Text UMP carrying up to 12 text bytes across words 1..3.
    ///
    /// # Arguments
    ///
    /// * `group` - Destination group (0-15).
    /// * `channel` - Destination channel (0-15).
    /// * `form` - Multi-packet format (0-3).
    /// * `addrs` - Target address (0: Channel, 1: Group, 2: Complete Endpoint).
    /// * `status_bank` - Status Bank.
    /// * `status` - Status byte.
    /// * `text` - Payload text bytes (up to 12 bytes packed into words 1..3).
    #[must_use]
    #[inline]
    pub fn flex_text(
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

    /// Creates a Flex Data Performance Text UMP (Status Bank = 0x01).
    #[must_use]
    #[inline]
    pub fn flex_performance(
        group: u8,
        channel: u8,
        form: u8,
        addrs: u8,
        status: u8,
        text: &[u8],
    ) -> Ump {
        Self::flex_text(
            group,
            channel,
            form,
            addrs,
            FLEXDATA_PERFORMANCE,
            status,
            text,
        )
    }

    /// Creates a Flex Data Lyric Text UMP (Status Bank = 0x02).
    #[must_use]
    #[inline]
    pub fn flex_lyric(group: u8, channel: u8, form: u8, addrs: u8, status: u8, text: &[u8]) -> Ump {
        Self::flex_text(group, channel, form, addrs, FLEXDATA_LYRIC, status, text)
    }

    /// Creates a Flex Data Bar Marker message (Status Bank 0x00, Status 0x07).
    #[must_use]
    #[inline]
    pub fn flex_bar_marker(group: u8, channel: u8, form: u8, addrs: u8, bar_number: u32) -> Ump {
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
    pub fn flex_beat_marker(group: u8, channel: u8, form: u8, addrs: u8, beat_number: u32) -> Ump {
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
    pub fn flex_project_metadata(
        group: u8,
        channel: u8,
        form: u8,
        addrs: u8,
        status: u8,
        text: &[u8],
    ) -> Ump {
        Self::flex_text(
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
    pub fn flex_project_name(group: u8, channel: u8, form: u8, addrs: u8, name: &[u8]) -> Ump {
        Self::flex_project_metadata(group, channel, form, addrs, FLEXDATA_PROJECT_NAME, name)
    }

    /// Creates a Flex Data Composition Name message (Status Bank 0x03, Status 0x01).
    #[must_use]
    #[inline]
    pub fn flex_composition_name(group: u8, channel: u8, form: u8, addrs: u8, name: &[u8]) -> Ump {
        Self::flex_project_metadata(group, channel, form, addrs, FLEXDATA_COMPOSITION_NAME, name)
    }

    /// Creates a Flex Data Copyright Notice message (Status Bank 0x03, Status 0x04).
    #[must_use]
    #[inline]
    pub fn flex_copyright_notice(
        group: u8,
        channel: u8,
        form: u8,
        addrs: u8,
        notice: &[u8],
    ) -> Ump {
        Self::flex_project_metadata(
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
    pub fn flex_track_metadata(
        group: u8,
        channel: u8,
        form: u8,
        addrs: u8,
        status: u8,
        text: &[u8],
    ) -> Ump {
        Self::flex_text(
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
    pub fn flex_track_name(group: u8, channel: u8, form: u8, addrs: u8, name: &[u8]) -> Ump {
        Self::flex_track_metadata(group, channel, form, addrs, FLEXDATA_TRACK_NAME, name)
    }

    /// Creates a Flex Data Clip Name message (Status Bank 0x04, Status 0x02).
    #[must_use]
    #[inline]
    pub fn flex_clip_name(group: u8, channel: u8, form: u8, addrs: u8, name: &[u8]) -> Ump {
        Self::flex_track_metadata(group, channel, form, addrs, FLEXDATA_CLIP_NAME, name)
    }
}

/// Status/Format of a 128-bit Flex Data message (bits 22-23 of word 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FlexDataFormat {
    /// Complete message in a single 128-bit UMP packet.
    Complete = 0x0,
    /// Start packet of a multi-packet message.
    Start = 0x1,
    /// Continue packet of a multi-packet message.
    Continue = 0x2,
    /// End packet of a multi-packet message.
    End = 0x3,
}

impl FlexDataFormat {
    #[must_use]
    #[inline]
    pub fn from_u8(val: u8) -> Option<Self> {
        match val & 0x3 {
            0x0 => Some(FlexDataFormat::Complete),
            0x1 => Some(FlexDataFormat::Start),
            0x2 => Some(FlexDataFormat::Continue),
            0x3 => Some(FlexDataFormat::End),
            _ => None,
        }
    }
}

/// An iterator that borrows a text payload and streams zero-allocation multi-packet Flex Data UMPs.
///
/// Multi-packet text messages carry up to 12 bytes per packet across words 1..3:
/// - Single packet <= 12 bytes -> Form 0 (Complete)
/// - Multi packet > 12 bytes -> Form 1 (Start), Form 2 (Continue), Form 3 (End)
#[derive(Debug, Clone)]
pub struct FlexDataChunker<'a> {
    group: u8,
    channel: u8,
    addrs: u8,
    status_bank: u8,
    status: u8,
    payload: &'a [u8],
    offset: usize,
    finished: bool,
}

impl<'a> FlexDataChunker<'a> {
    /// Creates a new `FlexDataChunker`.
    #[must_use]
    pub fn new(
        group: u8,
        channel: u8,
        addrs: u8,
        status_bank: u8,
        status: u8,
        payload: &'a [u8],
    ) -> Self {
        Self {
            group: group & 0xF,
            channel: channel & 0xF,
            addrs: addrs & 0x3,
            status_bank,
            status,
            payload,
            offset: 0,
            finished: false,
        }
    }
}

impl<'a> Iterator for FlexDataChunker<'a> {
    type Item = Ump;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let total_len = self.payload.len();
        if total_len == 0 {
            self.finished = true;
            return Some(UmpFactory::flex_text(
                self.group,
                self.channel,
                FlexDataFormat::Complete as u8,
                self.addrs,
                self.status_bank,
                self.status,
                &[],
            ));
        }

        let remaining = total_len - self.offset;
        let chunk_size = if remaining > 12 { 12 } else { remaining };
        let chunk = &self.payload[self.offset..self.offset + chunk_size];
        let is_first = self.offset == 0;
        let is_last = (self.offset + chunk_size) >= total_len;

        let format = match (is_first, is_last) {
            (true, true) => FlexDataFormat::Complete,
            (true, false) => FlexDataFormat::Start,
            (false, false) => FlexDataFormat::Continue,
            (false, true) => FlexDataFormat::End,
        };

        self.offset += chunk_size;
        if is_last {
            self.finished = true;
        }

        Some(UmpFactory::flex_text(
            self.group,
            self.channel,
            format as u8,
            self.addrs,
            self.status_bank,
            self.status,
            chunk,
        ))
    }
}

/// Error conditions that can occur during Flex Data reassembly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDataReceiverError {
    /// The incoming packet is not an MT=0xD Flex Data packet.
    NotFlexDataPacket,
    /// Group mismatch with the active multi-packet sequence.
    GroupMismatch,
    /// Channel or address mismatch.
    AddressMismatch,
    /// Status Bank or Status mismatch.
    StatusMismatch,
    /// Unexpected packet sequence format (e.g. Continue/End received when idle).
    UnexpectedFormat,
    /// The incoming packet data would exceed the fixed buffer capacity.
    BufferOverflow,
}

/// State of a `FlexDataReceiver`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDataReceiverState {
    /// Idle, waiting for Complete or Start packet.
    Idle,
    /// Currently accumulating multi-packet text.
    Receiving,
    /// Text message complete.
    Complete,
}

/// A zero-allocation, bounded-capacity receiver and reassembler for incoming Flex Data multi-packet text messages.
pub struct FlexDataReceiver<const CAP: usize> {
    buffer: [u8; CAP],
    len: usize,
    group: u8,
    channel: u8,
    addrs: u8,
    status_bank: u8,
    status: u8,
    state: FlexDataReceiverState,
}

impl<const CAP: usize> Default for FlexDataReceiver<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> FlexDataReceiver<CAP> {
    /// Creates a new `FlexDataReceiver` with capacity `CAP`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: [0; CAP],
            len: 0,
            group: 0,
            channel: 0,
            addrs: 0,
            status_bank: 0,
            status: 0,
            state: FlexDataReceiverState::Idle,
        }
    }

    /// Resets the receiver to idle state.
    pub fn reset(&mut self) {
        self.len = 0;
        self.state = FlexDataReceiverState::Idle;
    }

    /// Returns current state.
    #[must_use]
    #[inline]
    pub fn state(&self) -> FlexDataReceiverState {
        self.state
    }

    /// Returns the accumulated text bytes.
    #[must_use]
    #[inline]
    pub fn text(&self) -> &[u8] {
        &self.buffer[..self.len]
    }

    /// Returns the active Status Bank.
    #[must_use]
    #[inline]
    pub fn status_bank(&self) -> u8 {
        self.status_bank
    }

    /// Returns the active Status.
    #[must_use]
    #[inline]
    pub fn status(&self) -> u8 {
        self.status
    }

    /// Processes an incoming UMP packet.
    pub fn process_ump(
        &mut self,
        ump: &Ump,
    ) -> Result<FlexDataReceiverState, FlexDataReceiverError> {
        let mt = (ump.data[0] >> 28) as u8;
        if mt != UMP_FLEX_DATA {
            return Err(FlexDataReceiverError::NotFlexDataPacket);
        }

        let w0 = ump.data[0];
        let group = ((w0 >> 24) & 0xF) as u8;
        let form_nibble = ((w0 >> 22) & 0x3) as u8;
        let form =
            FlexDataFormat::from_u8(form_nibble).ok_or(FlexDataReceiverError::UnexpectedFormat)?;
        let addrs = ((w0 >> 20) & 0x3) as u8;
        let channel = ((w0 >> 16) & 0xF) as u8;
        let status_bank = ((w0 >> 8) & 0xFF) as u8;
        let status = (w0 & 0xFF) as u8;

        match form {
            FlexDataFormat::Complete | FlexDataFormat::Start => {
                self.len = 0;
                self.group = group;
                self.channel = channel;
                self.addrs = addrs;
                self.status_bank = status_bank;
                self.status = status;
            }
            FlexDataFormat::Continue | FlexDataFormat::End => {
                if self.state != FlexDataReceiverState::Receiving {
                    return Err(FlexDataReceiverError::UnexpectedFormat);
                }
                if group != self.group {
                    return Err(FlexDataReceiverError::GroupMismatch);
                }
                if channel != self.channel || addrs != self.addrs {
                    return Err(FlexDataReceiverError::AddressMismatch);
                }
                if status_bank != self.status_bank || status != self.status {
                    return Err(FlexDataReceiverError::StatusMismatch);
                }
            }
        }

        // Extract up to 12 text bytes from words 1..3
        let words = [ump.data[1], ump.data[2], ump.data[3]];
        let mut temp = [0_u8; 12];
        let mut extracted = 0;

        for word in &words {
            for b in (0..4).rev() {
                let byte = ((word >> (b * 8)) & 0xFF) as u8;
                if byte != 0 {
                    temp[extracted] = byte;
                    extracted += 1;
                }
            }
        }

        if self.len + extracted > CAP {
            self.reset();
            return Err(FlexDataReceiverError::BufferOverflow);
        }

        self.buffer[self.len..self.len + extracted].copy_from_slice(&temp[..extracted]);
        self.len += extracted;

        match form {
            FlexDataFormat::Complete | FlexDataFormat::End => {
                self.state = FlexDataReceiverState::Complete;
            }
            FlexDataFormat::Start | FlexDataFormat::Continue => {
                self.state = FlexDataReceiverState::Receiving;
            }
        }

        Ok(self.state)
    }
}

/// A scheduled UMP item paired with optional Jitter Reduction (JR) timestamp or Delta Clockstamp (DCT).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledUmp {
    /// The UMP packet payload.
    pub ump: Ump,
    /// Optional 16-bit Jitter Reduction timestamp.
    pub jr_timestamp: Option<u16>,
    /// Optional 16-bit Delta Clockstamp ticks since last event.
    pub dct_since_last: Option<u16>,
}

impl ScheduledUmp {
    /// Creates a new `ScheduledUmp`.
    #[must_use]
    #[inline]
    pub const fn new(ump: Ump) -> Self {
        Self {
            ump,
            jr_timestamp: None,
            dct_since_last: None,
        }
    }

    /// Attaches a JR Timestamp.
    #[must_use]
    #[inline]
    pub const fn with_jr_timestamp(mut self, ts: u16) -> Self {
        self.jr_timestamp = Some(ts);
        self
    }

    /// Attaches a Delta Clockstamp.
    #[must_use]
    #[inline]
    pub const fn with_dct(mut self, ticks: u16) -> Self {
        self.dct_since_last = Some(ticks);
        self
    }
}

/// A bounded, zero-allocation Jitter Reduction (JR) and Delta Clockstamp (DCT) Scheduler Queue.
///
/// Handles pairing incoming JR/DCT utility messages with adjacent Channel Voice and System messages,
/// and interleaving timestamps during outbound packet generation.
pub struct JitterReductionQueue<const CAP: usize> {
    items: [ScheduledUmp; CAP],
    len: usize,
    read_pos: usize,
    write_pos: usize,
    last_clock: u16,
    pending_jr_timestamp: Option<u16>,
    pending_dct_since: Option<u16>,
}

impl<const CAP: usize> Default for JitterReductionQueue<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> JitterReductionQueue<CAP> {
    /// Creates a new `JitterReductionQueue`.
    #[must_use]
    pub const fn new() -> Self {
        const { assert!(CAP > 0, "Queue capacity must be greater than zero") };
        Self {
            items: [ScheduledUmp {
                ump: Ump { data: [0; 4] },
                jr_timestamp: None,
                dct_since_last: None,
            }; CAP],
            len: 0,
            read_pos: 0,
            write_pos: 0,
            last_clock: 0,
            pending_jr_timestamp: None,
            pending_dct_since: None,
        }
    }

    /// Returns the number of items queued.
    #[must_use]
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Checks if queue is empty.
    #[must_use]
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Checks if queue is full.
    #[must_use]
    #[inline]
    pub const fn is_full(&self) -> bool {
        self.len == CAP
    }

    /// Updates the reference sender clock.
    #[inline]
    pub fn update_clock(&mut self, clock_time: u16) {
        self.last_clock = clock_time;
    }

    /// Ingests a raw UMP packet. If it is a Utility JR Clock/Timestamp or DCT message,
    /// it updates internal state and pairs it with the immediately subsequent message.
    ///
    /// # Returns
    /// - `Ok(true)` if a musical/data message was queued.
    /// - `Ok(false)` if a utility message was consumed and held for pairing.
    /// - `Err(&'static str)` if queue is full.
    pub fn ingest_ump(&mut self, ump: &Ump) -> Result<bool, &'static str> {
        let mt = (ump.data[0] >> 28) as u8;
        if mt == UTILITY_NOOP {
            let status = ((ump.data[0] >> 20) & 0xF) as u8;
            let val = (ump.data[0] & 0xFFFF) as u16;

            match status {
                UTILITY_JRCLOCK => {
                    self.update_clock(val);
                    return Ok(false);
                }
                UTILITY_JRTS => {
                    self.pending_jr_timestamp = Some(val);
                    return Ok(false);
                }
                UTILITY_DELTACLOCKSINCE => {
                    self.pending_dct_since = Some(val);
                    return Ok(false);
                }
                UTILITY_DELTACLOCKTICK => {
                    // Ticks per quarter note announcement; can be queued or tracked
                    return Ok(false);
                }
                _ => return Ok(false),
            }
        }

        if self.is_full() {
            return Err("JitterReductionQueue is full");
        }

        let scheduled = ScheduledUmp {
            ump: *ump,
            jr_timestamp: self.pending_jr_timestamp.take(),
            dct_since_last: self.pending_dct_since.take(),
        };

        self.items[self.write_pos] = scheduled;
        self.write_pos = (self.write_pos + 1) % CAP;
        self.len += 1;

        Ok(true)
    }

    /// Pops the next scheduled UMP from the queue.
    pub fn pop(&mut self) -> Option<ScheduledUmp> {
        if self.len == 0 {
            return None;
        }

        let item = self.items[self.read_pos];
        self.read_pos = (self.read_pos + 1) % CAP;
        self.len -= 1;
        Some(item)
    }

    /// Pushes an outbound message with an associated JR timestamp or DCT, queuing both
    /// the timestamp Utility UMP and the target message.
    pub fn push_outbound(
        &mut self,
        ump: &Ump,
        jr_timestamp: Option<u16>,
        dct_since: Option<u16>,
    ) -> Result<(), &'static str> {
        let count_needed =
            1 + usize::from(jr_timestamp.is_some()) + usize::from(dct_since.is_some());
        if self.len + count_needed > CAP {
            return Err("Insufficient capacity in JitterReductionQueue");
        }

        if let Some(ts) = jr_timestamp {
            let jr_ump = UmpFactory::jr_timestamp(ts);
            self.items[self.write_pos] = ScheduledUmp::new(jr_ump);
            self.write_pos = (self.write_pos + 1) % CAP;
            self.len += 1;
        }

        if let Some(dct) = dct_since {
            let dct_ump = UmpFactory::delta_clockstamp_since(dct);
            self.items[self.write_pos] = ScheduledUmp::new(dct_ump);
            self.write_pos = (self.write_pos + 1) % CAP;
            self.len += 1;
        }

        self.items[self.write_pos] = ScheduledUmp::new(*ump)
            .with_jr_timestamp(jr_timestamp.unwrap_or(0))
            .with_dct(dct_since.unwrap_or(0));
        self.write_pos = (self.write_pos + 1) % CAP;
        self.len += 1;

        Ok(())
    }
}
