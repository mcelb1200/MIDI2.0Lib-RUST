use crate::ump::{MessageType, Ump};

/// Status/Format of a 64-bit SysEx7 UMP (bits 20-23 of word 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SysEx7Format {
    /// Complete System Exclusive message in a single 64-bit UMP packet.
    Complete = 0x0,
    /// Start of a System Exclusive message spanning multiple packets.
    Start = 0x1,
    /// Continuation packet of a multi-packet System Exclusive message.
    Continue = 0x2,
    /// End packet of a multi-packet System Exclusive message.
    End = 0x3,
}

impl SysEx7Format {
    /// Converts a 4-bit nibble into a `SysEx7Format`, or `None` if invalid.
    #[must_use]
    #[inline]
    pub fn from_u8(val: u8) -> Option<Self> {
        match val & 0xF {
            0x0 => Some(SysEx7Format::Complete),
            0x1 => Some(SysEx7Format::Start),
            0x2 => Some(SysEx7Format::Continue),
            0x3 => Some(SysEx7Format::End),
            _ => None,
        }
    }
}

/// Helper function to construct a single 64-bit (MT=0x3) SysEx7 UMP.
///
/// Data bytes are masked to 7 bits (0x7F) in compliance with the MIDI 2.0 SysEx7 specification.
/// Any bytes beyond the given slice up to 6 are padded with zeros.
#[must_use]
pub fn build_sysex7_ump(group: u8, format: SysEx7Format, bytes: &[u8]) -> Ump {
    let num_bytes = if bytes.len() > 6 {
        6
    } else {
        bytes.len() as u8
    };
    let mut w0 = (0x3 << 28)
        | (((group & 0xF) as u32) << 24)
        | (((format as u8) as u32) << 20)
        | ((num_bytes as u32) << 16);
    let mut w1 = 0_u32;

    if num_bytes > 0 {
        w0 |= ((bytes[0] & 0x7F) as u32) << 8;
    }
    if num_bytes > 1 {
        w0 |= (bytes[1] & 0x7F) as u32;
    }
    if num_bytes > 2 {
        w1 |= ((bytes[2] & 0x7F) as u32) << 24;
    }
    if num_bytes > 3 {
        w1 |= ((bytes[3] & 0x7F) as u32) << 16;
    }
    if num_bytes > 4 {
        w1 |= ((bytes[4] & 0x7F) as u32) << 8;
    }
    if num_bytes > 5 {
        w1 |= (bytes[5] & 0x7F) as u32;
    }

    Ump {
        data: [w0, w1, 0, 0],
    }
}

/// An iterator that borrows a `&[u8]` SysEx payload and emits zero-allocation 64-bit (MT=0x3) UMPs.
///
/// This implements Session 3.1 of the MIDI 2.0 Roadmap for embedded / `no_std` environments
/// with zero dynamic memory allocation.
#[derive(Debug, Clone)]
pub struct SysEx7Chunker<'a> {
    group: u8,
    payload: &'a [u8],
    offset: usize,
    finished: bool,
}

impl<'a> SysEx7Chunker<'a> {
    /// Creates a new `SysEx7Chunker` for the given group and payload.
    #[must_use]
    #[inline]
    pub fn new(group: u8, payload: &'a [u8]) -> Self {
        Self {
            group: group & 0xF,
            payload,
            offset: 0,
            finished: false,
        }
    }
}

impl<'a> Iterator for SysEx7Chunker<'a> {
    type Item = Ump;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let total_len = self.payload.len();

        // Special case: empty payload fits into a single Complete packet with 0 bytes.
        if total_len == 0 {
            self.finished = true;
            return Some(build_sysex7_ump(self.group, SysEx7Format::Complete, &[]));
        }

        let remaining = total_len - self.offset;
        let chunk_size = if remaining > 6 { 6 } else { remaining };
        let chunk = &self.payload[self.offset..self.offset + chunk_size];
        let is_first = self.offset == 0;
        let is_last = (self.offset + chunk_size) >= total_len;

        let format = match (is_first, is_last) {
            (true, true) => SysEx7Format::Complete,
            (true, false) => SysEx7Format::Start,
            (false, false) => SysEx7Format::Continue,
            (false, true) => SysEx7Format::End,
        };

        self.offset += chunk_size;
        if is_last {
            self.finished = true;
        }

        Some(build_sysex7_ump(self.group, format, chunk))
    }
}

/// Error conditions that can occur during SysEx7 reassembly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysEx7ReceiverError {
    /// The incoming packet is not an MT=0x3 SysEx7 packet.
    NotSysEx7Packet,
    /// Group mismatch with the currently ongoing SysEx transmission.
    GroupMismatch,
    /// Unexpected packet sequence format (e.g. Continue/End received when not transmitting, or Start when already transmitting).
    UnexpectedFormat,
    /// The incoming packet data would exceed the fixed buffer capacity.
    BufferOverflow,
    /// The packet's reported byte count exceeds the valid maximum of 6.
    InvalidByteCount,
}

/// State of the `SysEx7Receiver` reassembler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysEx7ReceiverState {
    /// Idle, waiting for a `Complete` or `Start` packet.
    Idle,
    /// Currently receiving a multi-packet sequence.
    Receiving,
    /// A complete SysEx message has been collected in the buffer.
    Complete,
}

/// A zero-allocation, bounded-capacity receiver and reassembler for incoming SysEx7 UMP packets.
///
/// Designed specifically for embedded and `no_std` use cases where dynamic heap allocations
/// are forbidden and memory exhaustion attacks must be prevented.
pub struct SysEx7Receiver<const CAP: usize> {
    buffer: [u8; CAP],
    len: usize,
    active_group: u8,
    state: SysEx7ReceiverState,
}

impl<const CAP: usize> Default for SysEx7Receiver<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> SysEx7Receiver<CAP> {
    /// Creates a new, empty `SysEx7Receiver` with capacity `CAP`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: [0; CAP],
            len: 0,
            active_group: 0,
            state: SysEx7ReceiverState::Idle,
        }
    }

    /// Resets the receiver to idle state and clears accumulated data.
    pub fn reset(&mut self) {
        self.len = 0;
        self.state = SysEx7ReceiverState::Idle;
    }

    /// Returns the current state of the receiver.
    #[must_use]
    #[inline]
    pub fn state(&self) -> SysEx7ReceiverState {
        self.state
    }

    /// Returns the current active group (if receiving or complete).
    #[must_use]
    #[inline]
    pub fn group(&self) -> u8 {
        self.active_group
    }

    /// Returns a slice of the accumulated SysEx payload bytes.
    #[must_use]
    #[inline]
    pub fn payload(&self) -> &[u8] {
        &self.buffer[..self.len]
    }

    /// Processes an incoming UMP packet.
    ///
    /// # Returns
    /// - `Ok(SysEx7ReceiverState::Complete)` when a complete SysEx message has been assembled.
    /// - `Ok(SysEx7ReceiverState::Receiving)` when more packets are expected.
    /// - `Err(SysEx7ReceiverError)` if an invalid, out-of-order, or overflowing packet was received.
    pub fn process_ump(&mut self, ump: &Ump) -> Result<SysEx7ReceiverState, SysEx7ReceiverError> {
        if ump.message_type() != MessageType::SysEx7 {
            return Err(SysEx7ReceiverError::NotSysEx7Packet);
        }

        let w0 = ump.data[0];
        let w1 = ump.data[1];

        let group = ((w0 >> 24) & 0xF) as u8;
        let format_nibble = ((w0 >> 20) & 0xF) as u8;
        let format =
            SysEx7Format::from_u8(format_nibble).ok_or(SysEx7ReceiverError::UnexpectedFormat)?;
        let num_bytes = ((w0 >> 16) & 0xF) as usize;

        if num_bytes > 6 {
            return Err(SysEx7ReceiverError::InvalidByteCount);
        }

        // Validate state transitions
        match format {
            SysEx7Format::Complete => {
                self.len = 0;
                self.active_group = group;
            }
            SysEx7Format::Start => {
                self.len = 0;
                self.active_group = group;
            }
            SysEx7Format::Continue | SysEx7Format::End => {
                if self.state != SysEx7ReceiverState::Receiving {
                    return Err(SysEx7ReceiverError::UnexpectedFormat);
                }
                if group != self.active_group {
                    return Err(SysEx7ReceiverError::GroupMismatch);
                }
            }
        }

        // Check buffer capacity
        if self.len + num_bytes > CAP {
            self.reset();
            return Err(SysEx7ReceiverError::BufferOverflow);
        }

        // Extract bytes (sanitized to 7 bits)
        let mut temp = [0_u8; 6];
        if num_bytes > 0 {
            temp[0] = ((w0 >> 8) & 0x7F) as u8;
        }
        if num_bytes > 1 {
            temp[1] = (w0 & 0x7F) as u8;
        }
        if num_bytes > 2 {
            temp[2] = ((w1 >> 24) & 0x7F) as u8;
        }
        if num_bytes > 3 {
            temp[3] = ((w1 >> 16) & 0x7F) as u8;
        }
        if num_bytes > 4 {
            temp[4] = ((w1 >> 8) & 0x7F) as u8;
        }
        if num_bytes > 5 {
            temp[5] = (w1 & 0x7F) as u8;
        }

        self.buffer[self.len..self.len + num_bytes].copy_from_slice(&temp[..num_bytes]);
        self.len += num_bytes;

        match format {
            SysEx7Format::Complete | SysEx7Format::End => {
                self.state = SysEx7ReceiverState::Complete;
            }
            SysEx7Format::Start | SysEx7Format::Continue => {
                self.state = SysEx7ReceiverState::Receiving;
            }
        }

        Ok(self.state)
    }
}

/// Status/Format of a 128-bit SysEx8 UMP (bits 20-23 of word 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SysEx8Format {
    /// Complete System Exclusive 8 message in a single 128-bit UMP packet.
    Complete = 0x0,
    /// Start of a System Exclusive 8 message spanning multiple packets.
    Start = 0x1,
    /// Continuation packet of a multi-packet System Exclusive 8 message.
    Continue = 0x2,
    /// End packet of a multi-packet System Exclusive 8 message.
    End = 0x3,
}

impl SysEx8Format {
    /// Converts a 4-bit nibble into a `SysEx8Format`, or `None` if invalid.
    #[must_use]
    #[inline]
    pub fn from_u8(val: u8) -> Option<Self> {
        match val & 0xF {
            0x0 => Some(SysEx8Format::Complete),
            0x1 => Some(SysEx8Format::Start),
            0x2 => Some(SysEx8Format::Continue),
            0x3 => Some(SysEx8Format::End),
            _ => None,
        }
    }
}

/// Helper function to construct a single 128-bit (MT=0x5) SysEx8 UMP.
///
/// In MIDI 2.0 SysEx8:
/// - Word 0 bits [28:31] = 0x5 (MT)
/// - Word 0 bits [24:27] = Group (0-15)
/// - Word 0 bits [20:23] = Status / Format (0=Complete, 1=Start, 2=Continue, 3=End)
/// - Word 0 bits [16:19] = Total bytes in this packet including Stream ID (1 to 14, where num_payload_bytes + 1)
/// - Word 0 bits [8:15]  = Stream ID
/// - Word 0 bits [0:7]   = Payload byte 0 (if present)
/// - Word 1 = Payload bytes 1-4 (big-endian inside the word)
/// - Word 2 = Payload bytes 5-8
/// - Word 3 = Payload bytes 9-12
///
/// Payload bytes in SysEx8 are 8-bit clean data.
/// Any bytes beyond the given slice up to 13 are padded with zeros.
#[must_use]
pub fn build_sysex8_ump(group: u8, stream_id: u8, format: SysEx8Format, bytes: &[u8]) -> Ump {
    let payload_len = if bytes.len() > 13 {
        13
    } else {
        bytes.len() as u8
    };
    let num_bytes = payload_len + 1; // includes stream_id

    let mut w0 = (0x5 << 28)
        | (((group & 0xF) as u32) << 24)
        | (((format as u8) as u32) << 20)
        | (((num_bytes & 0xF) as u32) << 16)
        | ((stream_id as u32) << 8);

    let mut w1 = 0_u32;
    let mut w2 = 0_u32;
    let mut w3 = 0_u32;

    if payload_len > 0 {
        w0 |= bytes[0] as u32;
    }
    if payload_len > 1 {
        w1 |= (bytes[1] as u32) << 24;
    }
    if payload_len > 2 {
        w1 |= (bytes[2] as u32) << 16;
    }
    if payload_len > 3 {
        w1 |= (bytes[3] as u32) << 8;
    }
    if payload_len > 4 {
        w1 |= bytes[4] as u32;
    }
    if payload_len > 5 {
        w2 |= (bytes[5] as u32) << 24;
    }
    if payload_len > 6 {
        w2 |= (bytes[6] as u32) << 16;
    }
    if payload_len > 7 {
        w2 |= (bytes[7] as u32) << 8;
    }
    if payload_len > 8 {
        w2 |= bytes[8] as u32;
    }
    if payload_len > 9 {
        w3 |= (bytes[9] as u32) << 24;
    }
    if payload_len > 10 {
        w3 |= (bytes[10] as u32) << 16;
    }
    if payload_len > 11 {
        w3 |= (bytes[11] as u32) << 8;
    }
    if payload_len > 12 {
        w3 |= bytes[12] as u32;
    }

    Ump {
        data: [w0, w1, w2, w3],
    }
}

/// An iterator that borrows a `&[u8]` SysEx8 payload and emits zero-allocation 128-bit (MT=0x5) UMPs.
///
/// Designed for embedded / `no_std` environments with zero dynamic memory allocation.
#[derive(Debug, Clone)]
pub struct SysEx8Chunker<'a> {
    group: u8,
    stream_id: u8,
    payload: &'a [u8],
    offset: usize,
    finished: bool,
}

impl<'a> SysEx8Chunker<'a> {
    /// Creates a new `SysEx8Chunker` for the given group, stream_id, and payload.
    #[must_use]
    #[inline]
    pub fn new(group: u8, stream_id: u8, payload: &'a [u8]) -> Self {
        Self {
            group: group & 0xF,
            stream_id,
            payload,
            offset: 0,
            finished: false,
        }
    }
}

impl<'a> Iterator for SysEx8Chunker<'a> {
    type Item = Ump;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let total_len = self.payload.len();

        // Special case: empty payload fits into a single Complete packet with 0 payload bytes (num_bytes = 1 for stream_id).
        if total_len == 0 {
            self.finished = true;
            return Some(build_sysex8_ump(
                self.group,
                self.stream_id,
                SysEx8Format::Complete,
                &[],
            ));
        }

        let remaining = total_len - self.offset;
        let chunk_size = if remaining > 13 { 13 } else { remaining };
        let chunk = &self.payload[self.offset..self.offset + chunk_size];
        let is_first = self.offset == 0;
        let is_last = (self.offset + chunk_size) >= total_len;

        let format = match (is_first, is_last) {
            (true, true) => SysEx8Format::Complete,
            (true, false) => SysEx8Format::Start,
            (false, false) => SysEx8Format::Continue,
            (false, true) => SysEx8Format::End,
        };

        self.offset += chunk_size;
        if is_last {
            self.finished = true;
        }

        Some(build_sysex8_ump(self.group, self.stream_id, format, chunk))
    }
}

/// Error conditions that can occur during SysEx8 reassembly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysEx8ReceiverError {
    /// The incoming packet is not an MT=0x5 Data packet.
    NotSysEx8Packet,
    /// Group mismatch with the currently ongoing SysEx transmission.
    GroupMismatch,
    /// Stream ID mismatch with the currently ongoing SysEx transmission.
    StreamIdMismatch,
    /// Unexpected packet sequence format (e.g. Continue/End received when not transmitting, or Start when already transmitting).
    UnexpectedFormat,
    /// The incoming packet data would exceed the fixed buffer capacity.
    BufferOverflow,
    /// The packet's reported total byte count exceeds the valid maximum of 14 (or is 0).
    InvalidByteCount,
}

/// State of the `SysEx8Receiver` reassembler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysEx8ReceiverState {
    /// Idle, waiting for a `Complete` or `Start` packet.
    Idle,
    /// Currently receiving a multi-packet sequence.
    Receiving,
    /// A complete SysEx8 message has been collected in the buffer.
    Complete,
}

/// A zero-allocation, bounded-capacity receiver and reassembler for incoming SysEx8 UMP packets.
///
/// Designed specifically for embedded and `no_std` use cases where dynamic heap allocations
/// are forbidden and memory exhaustion attacks must be prevented.
pub struct SysEx8Receiver<const CAP: usize> {
    buffer: [u8; CAP],
    len: usize,
    active_group: u8,
    active_stream_id: u8,
    state: SysEx8ReceiverState,
}

impl<const CAP: usize> Default for SysEx8Receiver<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> SysEx8Receiver<CAP> {
    /// Creates a new, empty `SysEx8Receiver` with capacity `CAP`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: [0; CAP],
            len: 0,
            active_group: 0,
            active_stream_id: 0,
            state: SysEx8ReceiverState::Idle,
        }
    }

    /// Resets the receiver to idle state and clears accumulated data.
    pub fn reset(&mut self) {
        self.len = 0;
        self.state = SysEx8ReceiverState::Idle;
    }

    /// Returns the current state of the receiver.
    #[must_use]
    #[inline]
    pub fn state(&self) -> SysEx8ReceiverState {
        self.state
    }

    /// Returns the current active group (if receiving or complete).
    #[must_use]
    #[inline]
    pub fn group(&self) -> u8 {
        self.active_group
    }

    /// Returns the current active stream ID (if receiving or complete).
    #[must_use]
    #[inline]
    pub fn stream_id(&self) -> u8 {
        self.active_stream_id
    }

    /// Returns a slice of the accumulated SysEx8 payload bytes.
    #[must_use]
    #[inline]
    pub fn payload(&self) -> &[u8] {
        &self.buffer[..self.len]
    }

    /// Processes an incoming UMP packet.
    ///
    /// # Returns
    /// - `Ok(SysEx8ReceiverState::Complete)` when a complete SysEx8 message has been assembled.
    /// - `Ok(SysEx8ReceiverState::Receiving)` when more packets are expected.
    /// - `Err(SysEx8ReceiverError)` if an invalid, out-of-order, or overflowing packet was received.
    pub fn process_ump(&mut self, ump: &Ump) -> Result<SysEx8ReceiverState, SysEx8ReceiverError> {
        if ump.message_type() != MessageType::Data {
            return Err(SysEx8ReceiverError::NotSysEx8Packet);
        }

        let w0 = ump.data[0];
        let w1 = ump.data[1];
        let w2 = ump.data[2];
        let w3 = ump.data[3];

        let group = ((w0 >> 24) & 0xF) as u8;
        let format_nibble = ((w0 >> 20) & 0xF) as u8;
        let format =
            SysEx8Format::from_u8(format_nibble).ok_or(SysEx8ReceiverError::UnexpectedFormat)?;
        let num_bytes = ((w0 >> 16) & 0xF) as usize;

        // num_bytes includes stream_id (1 byte), so valid num_bytes is between 1 and 14.
        if num_bytes == 0 || num_bytes > 14 {
            return Err(SysEx8ReceiverError::InvalidByteCount);
        }

        let stream_id = ((w0 >> 8) & 0xFF) as u8;
        let payload_len = num_bytes - 1;

        // Validate state transitions
        match format {
            SysEx8Format::Complete | SysEx8Format::Start => {
                self.len = 0;
                self.active_group = group;
                self.active_stream_id = stream_id;
            }
            SysEx8Format::Continue | SysEx8Format::End => {
                if self.state != SysEx8ReceiverState::Receiving {
                    return Err(SysEx8ReceiverError::UnexpectedFormat);
                }
                if group != self.active_group {
                    return Err(SysEx8ReceiverError::GroupMismatch);
                }
                if stream_id != self.active_stream_id {
                    return Err(SysEx8ReceiverError::StreamIdMismatch);
                }
            }
        }

        // Check buffer capacity
        if self.len + payload_len > CAP {
            self.reset();
            return Err(SysEx8ReceiverError::BufferOverflow);
        }

        // Extract bytes (8-bit clean)
        let mut temp = [0_u8; 13];
        if payload_len > 0 {
            temp[0] = (w0 & 0xFF) as u8;
        }
        if payload_len > 1 {
            temp[1] = ((w1 >> 24) & 0xFF) as u8;
        }
        if payload_len > 2 {
            temp[2] = ((w1 >> 16) & 0xFF) as u8;
        }
        if payload_len > 3 {
            temp[3] = ((w1 >> 8) & 0xFF) as u8;
        }
        if payload_len > 4 {
            temp[4] = (w1 & 0xFF) as u8;
        }
        if payload_len > 5 {
            temp[5] = ((w2 >> 24) & 0xFF) as u8;
        }
        if payload_len > 6 {
            temp[6] = ((w2 >> 16) & 0xFF) as u8;
        }
        if payload_len > 7 {
            temp[7] = ((w2 >> 8) & 0xFF) as u8;
        }
        if payload_len > 8 {
            temp[8] = (w2 & 0xFF) as u8;
        }
        if payload_len > 9 {
            temp[9] = ((w3 >> 24) & 0xFF) as u8;
        }
        if payload_len > 10 {
            temp[10] = ((w3 >> 16) & 0xFF) as u8;
        }
        if payload_len > 11 {
            temp[11] = ((w3 >> 8) & 0xFF) as u8;
        }
        if payload_len > 12 {
            temp[12] = (w3 & 0xFF) as u8;
        }

        self.buffer[self.len..self.len + payload_len].copy_from_slice(&temp[..payload_len]);
        self.len += payload_len;

        match format {
            SysEx8Format::Complete | SysEx8Format::End => {
                self.state = SysEx8ReceiverState::Complete;
            }
            SysEx8Format::Start | SysEx8Format::Continue => {
                self.state = SysEx8ReceiverState::Receiving;
            }
        }

        Ok(self.state)
    }
}
