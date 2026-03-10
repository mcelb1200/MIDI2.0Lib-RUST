use core::fmt;

/// Represents the MIDI 2.0 Message Type (MT).
/// The Message Type is the high 4 bits of the first word of a Universal MIDI Packet (UMP).
/// It determines the format and length of the message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    /// Utility Messages (32 bits). Used for NOOP, timestamps, and clock signals.
    Utility = 0x0,
    /// System Real Time and System Common Messages (32 bits).
    System = 0x1,
    /// MIDI 1.0 Channel Voice Messages (32 bits).
    Midi1ChannelVoice = 0x2,
    /// 64-bit Data Messages, including SysEx7.
    SysEx7 = 0x3,
    /// MIDI 2.0 Channel Voice Messages (64 bits).
    Midi2ChannelVoice = 0x4,
    /// 128-bit Data Messages.
    Data = 0x5,
    /// Reserved for future use.
    Reserved6 = 0x6,
    /// Reserved for future use.
    Reserved7 = 0x7,
    /// Reserved for future use (64-bit).
    Reserved8 = 0x8,
    /// Reserved for future use (64-bit).
    Reserved9 = 0x9,
    /// Reserved for future use (64-bit).
    ReservedA = 0xA,
    /// Reserved for future use (96-bit).
    ReservedB = 0xB,
    /// Reserved for future use (96-bit).
    ReservedC = 0xC,
    /// Flex Data Messages (128 bits).
    FlexData = 0xD,
    /// Reserved for future use (128-bit).
    ReservedE = 0xE,
    /// UMP Stream Messages (128 bits). Used for endpoint discovery and protocol negotiation.
    Stream = 0xF,
}

impl MessageType {
    /// Converts a raw 4-bit integer into a `MessageType` enum.
    ///
    /// # Arguments
    ///
    /// * `val` - The 4-bit integer value (0-15).
    ///
    /// # Returns
    ///
    /// The corresponding `MessageType`. Defaults to `Utility` if an invalid value is provided (though all 4-bit values are covered).
    pub fn from_u8(val: u8) -> Self {
        const MESSAGE_TYPES: [MessageType; 16] = [
            MessageType::Utility,
            MessageType::System,
            MessageType::Midi1ChannelVoice,
            MessageType::SysEx7,
            MessageType::Midi2ChannelVoice,
            MessageType::Data,
            MessageType::Reserved6,
            MessageType::Reserved7,
            MessageType::Reserved8,
            MessageType::Reserved9,
            MessageType::ReservedA,
            MessageType::ReservedB,
            MessageType::ReservedC,
            MessageType::FlexData,
            MessageType::ReservedE,
            MessageType::Stream,
        ];

        // ⚡ Bolt Optimization: Replace explicit bounds check with a bitmask
        // to prevent branch mispredictions in hot parsing loops.
        MESSAGE_TYPES[(val & 0xF) as usize]
    }

    /// Returns the number of 32-bit words required for this message type.
    ///
    /// # Returns
    ///
    /// The number of words (1, 2, 3, or 4).
    pub fn word_count(&self) -> usize {
        const WORD_COUNTS: [u8; 16] = [
            1, // Utility
            1, // System
            1, // Midi1ChannelVoice
            2, // SysEx7
            2, // Midi2ChannelVoice
            4, // Data
            1, // Reserved6
            1, // Reserved7
            2, // Reserved8
            2, // Reserved9
            2, // ReservedA
            3, // ReservedB
            3, // ReservedC
            4, // FlexData
            4, // ReservedE
            4, // Stream
        ];
        WORD_COUNTS[*self as usize] as usize
    }
}

impl From<u8> for MessageType {
    /// Converts a raw u8 value to a `MessageType`.
    fn from(val: u8) -> Self {
        Self::from_u8(val)
    }
}

/// A Universal MIDI Packet (UMP).
///
/// This struct holds up to 128 bits (4 x 32-bit words) of data, which is the maximum size of a UMP.
/// It provides helper methods to access and modify common fields like Message Type, Group, Status, and Channel.
/// Note that not all messages use all 4 words; check `message_type().word_count()` to see how many are valid.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Ump {
    /// The raw data of the UMP, stored as four 32-bit integers.
    pub data: [u32; 4],
}

impl Ump {
    /// Creates a new, empty UMP (initialized to zero).
    ///
    /// # Returns
    ///
    /// A new `Ump` instance with all data set to 0.
    pub fn new() -> Self {
        Ump { data: [0; 4] }
    }

    /// Gets the Message Type of the UMP.
    ///
    /// # Returns
    ///
    /// The `MessageType` derived from the high 4 bits of the first word.
    pub fn message_type(&self) -> MessageType {
        let mt = (self.data[0] >> 28) as u8;
        MessageType::from(mt)
    }

    /// Sets the Message Type of the UMP.
    ///
    /// This modifies the high 4 bits of the first word.
    ///
    /// # Arguments
    ///
    /// * `mt` - The `MessageType` to set.
    pub fn set_message_type(&mut self, mt: MessageType) {
        self.data[0] &= 0x0FFFFFFF;
        self.data[0] |= (mt as u8 as u32) << 28;
    }

    /// Gets the Group number (0-15) of the UMP.
    ///
    /// The Group field is located in bits [24:27] of the first word.
    ///
    /// # Returns
    ///
    /// The group number.
    pub fn group(&self) -> u8 {
        ((self.data[0] >> 24) & 0xF) as u8
    }

    /// Sets the Group number of the UMP.
    ///
    /// # Arguments
    ///
    /// * `group` - The group number to set (0-15). The value is masked to 4 bits.
    pub fn set_group(&mut self, group: u8) {
        self.data[0] &= 0xF0FFFFFF;
        self.data[0] |= ((group as u32) & 0xF) << 24;
    }

    /// Gets the Status byte of the UMP.
    ///
    /// For many message types (like Channel Voice), the status nibble is at bits [20:23] of the first word.
    /// However, this method extracts bits [16:23] masked with 0xF0, effectively getting the high nibble of the byte at that position.
    /// This is commonly used to identify the specific command (e.g., Note On, Note Off) within a message type.
    ///
    /// # Returns
    ///
    /// The status byte (with lower nibble zeroed out).
    pub fn status(&self) -> u8 {
        ((self.data[0] >> 16) & 0xF0) as u8
    }

    /// Gets the Channel number (0-15) of the UMP.
    ///
    /// For Channel Voice messages, the channel is stored in bits [16:19] of the first word.
    ///
    /// # Returns
    ///
    /// The channel number.
    pub fn channel(&self) -> u8 {
        ((self.data[0] >> 16) & 0x0F) as u8
    }
}

impl Default for Ump {
    /// Creates a default UMP (all zeros).
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Ump {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Ump({:08X}, {:08X}, {:08X}, {:08X})",
            self.data[0], self.data[1], self.data[2], self.data[3]
        )
    }
}

impl fmt::Display for Ump {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
