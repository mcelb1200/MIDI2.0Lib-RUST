//! USB-MIDI 2.0 Class Specification & Group Terminal Blocks (GTB)
//!
//! Provides descriptors and packet framing for USB-MIDI 2.0 endpoints,
//! including Group Terminal Block descriptors and zero-copy packet packers/unpackers.

use crate::ump::Ump;

/// USB Class / Subclass / Protocol Constants for MIDI 2.0
pub const USB_AUDIO_CLASS: u8 = 0x01;
pub const USB_MIDISTREAMING_SUBCLASS: u8 = 0x03;
pub const USB_MIDI_PROTOCOL_UMP: u8 = 0x02; // MIDI 2.0 UMP Protocol

/// Class-Specific AC/MS Descriptor Subtypes
pub const MS_DESCRIPTOR_UNDEFINED: u8 = 0x00;
pub const MS_HEADER: u8 = 0x01;
pub const MS_HEADER_2_0: u8 = 0x02;
pub const MS_GROUP_TERMINAL_BLOCK: u8 = 0x03;

/// Group Terminal Block Type (Directionality)
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GroupTerminalBlockType {
    /// Bidirectional terminal block (Input and Output)
    Bidirectional = 0x00,
    /// Input Only terminal block (device receives from host / input jack)
    InputOnly = 0x01,
    /// Output Only terminal block (device transmits to host / output jack)
    OutputOnly = 0x02,
}

impl GroupTerminalBlockType {
    #[inline]
    #[must_use]
    pub const fn from_u8(val: u8) -> Self {
        match val {
            0x01 => Self::InputOnly,
            0x02 => Self::OutputOnly,
            _ => Self::Bidirectional,
        }
    }
}

/// Group Terminal Block Protocol Capability
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GroupTerminalProtocol {
    /// Unknown or not configured
    Unknown = 0x00,
    /// Default protocol is MIDI 1.0 (UP / 64-bit packets with 31.25kbps limit)
    Midi1_0 = 0x01,
    /// Default protocol is MIDI 2.0 (UMP Native)
    Midi2_0 = 0x02,
}

impl GroupTerminalProtocol {
    #[inline]
    #[must_use]
    pub const fn from_u8(val: u8) -> Self {
        match val {
            0x01 => Self::Midi1_0,
            0x02 => Self::Midi2_0,
            _ => Self::Unknown,
        }
    }
}

/// USB-MIDI 2.0 Group Terminal Block (GTB) Descriptor
///
/// Describes the association of UMP Groups to physical/virtual jacks,
/// directionality, protocol capability, and SysEx8 stream limits.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct GroupTerminalBlock {
    /// Block ID (1-based index)
    pub block_id: u8,
    /// Directionality of the block
    pub block_type: GroupTerminalBlockType,
    /// First Group number (0..15)
    pub group_first: u8,
    /// Number of Groups spanned by this block (1..16)
    pub group_count: u8,
    /// MIDI protocol supported / default
    pub protocol: GroupTerminalProtocol,
    /// Max SysEx8 streams supported (0 = not supported)
    pub max_sysex8_streams: u8,
    /// String descriptor index describing this terminal block (0 = none)
    pub string_idx: u8,
}

impl GroupTerminalBlock {
    /// Length in bytes of a Class-Specific Group Terminal Block descriptor
    pub const DESCRIPTOR_LEN: u8 = 13;
    pub const DESCRIPTOR_TYPE_CS_INTERFACE: u8 = 0x24;

    /// Creates a new `GroupTerminalBlock`.
    #[must_use]
    pub const fn new(
        block_id: u8,
        block_type: GroupTerminalBlockType,
        group_first: u8,
        group_count: u8,
    ) -> Self {
        Self {
            block_id,
            block_type,
            group_first,
            group_count,
            protocol: GroupTerminalProtocol::Midi2_0,
            max_sysex8_streams: 0,
            string_idx: 0,
        }
    }

    /// Sets the MIDI protocol capability.
    #[must_use]
    #[inline]
    pub const fn protocol(mut self, protocol: GroupTerminalProtocol) -> Self {
        self.protocol = protocol;
        self
    }

    /// Sets the maximum SysEx8 streams count.
    #[must_use]
    #[inline]
    pub const fn max_sysex8_streams(mut self, count: u8) -> Self {
        self.max_sysex8_streams = count;
        self
    }

    /// Sets the string descriptor index.
    #[must_use]
    #[inline]
    pub const fn string_index(mut self, idx: u8) -> Self {
        self.string_idx = idx;
        self
    }

    /// Serializes this Group Terminal Block descriptor into a standard 13-byte USB descriptor buffer.
    ///
    /// # Format:
    /// - `bLength` (13)
    /// - `bDescriptorType` (0x24 = CS_INTERFACE)
    /// - `bDescriptorSubtype` (0x03 = MS_GROUP_TERMINAL_BLOCK)
    /// - `bGrpTrmBlkID` (1 byte)
    /// - `bGrpTrmBlkType` (1 byte: 0=Bidi, 1=In, 2=Out)
    /// - `nGroupTrm` (1 byte: group_first)
    /// - `nNumGroupTrm` (1 byte: group_count)
    /// - `iBlockItem` (1 byte: string descriptor index)
    /// - `bMIDIProtocol` (1 byte: 1=MIDI 1.0, 2=MIDI 2.0)
    /// - `wMaxSysex8Streams` (2 bytes little-endian)
    /// - `wReserved` (2 bytes = 0x0000)
    #[inline]
    pub fn serialize(&self, buf: &mut [u8; 13]) {
        buf[0] = Self::DESCRIPTOR_LEN;
        buf[1] = Self::DESCRIPTOR_TYPE_CS_INTERFACE;
        buf[2] = MS_GROUP_TERMINAL_BLOCK;
        buf[3] = self.block_id;
        buf[4] = self.block_type as u8;
        buf[5] = self.group_first & 0x0F;
        buf[6] = self.group_count;
        buf[7] = self.string_idx;
        buf[8] = self.protocol as u8;
        buf[9] = self.max_sysex8_streams;
        buf[10] = 0x00;
        buf[11] = 0x00;
        buf[12] = 0x00;
    }

    /// Parses a Class-Specific Group Terminal Block descriptor from raw bytes.
    #[must_use]
    pub fn parse(buf: &[u8]) -> Option<Self> {
        if buf.len() < 13 {
            return None;
        }
        if buf[0] < 13
            || buf[1] != Self::DESCRIPTOR_TYPE_CS_INTERFACE
            || buf[2] != MS_GROUP_TERMINAL_BLOCK
        {
            return None;
        }
        Some(Self {
            block_id: buf[3],
            block_type: GroupTerminalBlockType::from_u8(buf[4]),
            group_first: buf[5] & 0x0F,
            group_count: buf[6],
            string_idx: buf[7],
            protocol: GroupTerminalProtocol::from_u8(buf[8]),
            max_sysex8_streams: buf[9],
        })
    }
}

/// USB-MIDI 2.0 Header Descriptor
///
/// Class-Specific MS Interface Header Descriptor (v2.0).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct UsbMidi2HeaderDescriptor {
    /// BCD Release specification (0x0200 for USB-MIDI 2.0)
    pub bcd_release: u16,
    /// Total length of class-specific descriptors returned
    pub total_length: u16,
}

impl UsbMidi2HeaderDescriptor {
    pub const DESCRIPTOR_LEN: u8 = 7;
    pub const DESCRIPTOR_TYPE_CS_INTERFACE: u8 = 0x24;

    #[must_use]
    pub const fn new(total_length: u16) -> Self {
        Self {
            bcd_release: 0x0200,
            total_length,
        }
    }

    #[inline]
    pub fn serialize(&self, buf: &mut [u8; 7]) {
        buf[0] = Self::DESCRIPTOR_LEN;
        buf[1] = Self::DESCRIPTOR_TYPE_CS_INTERFACE;
        buf[2] = MS_HEADER;
        buf[3] = (self.bcd_release & 0xFF) as u8;
        buf[4] = ((self.bcd_release >> 8) & 0xFF) as u8;
        buf[5] = (self.total_length & 0xFF) as u8;
        buf[6] = ((self.total_length >> 8) & 0xFF) as u8;
    }

    #[must_use]
    pub fn parse(buf: &[u8]) -> Option<Self> {
        if buf.len() < 7 {
            return None;
        }
        if buf[0] < 7 || buf[1] != Self::DESCRIPTOR_TYPE_CS_INTERFACE || buf[2] != MS_HEADER {
            return None;
        }
        let bcd_release = u16::from(buf[3]) | (u16::from(buf[4]) << 8);
        let total_length = u16::from(buf[5]) | (u16::from(buf[6]) << 8);
        Some(Self {
            bcd_release,
            total_length,
        })
    }
}

/// Zero-copy USB-MIDI 2.0 Packet Framing and Stream Unpacker
///
/// In USB-MIDI 2.0, packets transferred over USB Bulk/Interrupt endpoints consist
/// directly of 32-bit UMP words in Little-Endian byte order.
pub struct UsbMidiPacketPacker;

impl UsbMidiPacketPacker {
    /// Packs a slice of UMPs into a raw USB byte buffer in Little-Endian format.
    ///
    /// Returns the number of bytes written to `dst` or None if the buffer is too small.
    #[inline]
    pub fn pack_umps(umps: &[Ump], dst: &mut [u8]) -> Option<usize> {
        let mut offset = 0;
        for ump in umps {
            let words = ump.word_count();
            let needed_bytes = words * 4;
            if offset + needed_bytes > dst.len() {
                return None;
            }
            for i in 0..words {
                let word_bytes = ump.data[i].to_le_bytes();
                dst[offset..offset + 4].copy_from_slice(&word_bytes);
                offset += 4;
            }
        }
        Some(offset)
    }

    /// Unpacks a single UMP from a raw USB byte slice in Little-Endian format.
    ///
    /// Returns `Some((ump, bytes_consumed))` on success, or `None` if `src` contains
    /// fewer bytes than required for the message type.
    #[inline]
    #[must_use]
    pub fn unpack_ump(src: &[u8]) -> Option<(Ump, usize)> {
        if src.len() < 4 {
            return None;
        }
        let w0 = u32::from_le_bytes([src[0], src[1], src[2], src[3]]);
        let words = Ump::calculate_word_count(w0);
        let needed_bytes = words * 4;
        if src.len() < needed_bytes {
            return None;
        }

        let mut data = [0_u32; 4];
        data[0] = w0;
        for (i, word) in data.iter_mut().enumerate().take(words).skip(1) {
            let off = i * 4;
            *word = u32::from_le_bytes([src[off], src[off + 1], src[off + 2], src[off + 3]]);
        }

        Some((Ump { data }, needed_bytes))
    }
}
