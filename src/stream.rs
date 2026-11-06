use crate::ump::{MessageType, Ump};

/// Status codes for MIDI 2.0 UMP Stream Messages (MT=0xF).
pub const STATUS_ENDPOINT_DISCOVERY: u16 = 0x000;
pub const STATUS_ENDPOINT_INFO_NOTIFICATION: u16 = 0x001;
pub const STATUS_DEVICE_IDENTITY_NOTIFICATION: u16 = 0x002;
pub const STATUS_ENDPOINT_NAME_NOTIFICATION: u16 = 0x003;
pub const STATUS_PRODUCT_INSTANCE_ID_NOTIFICATION: u16 = 0x004;
pub const STATUS_STREAM_CONFIG_REQUEST: u16 = 0x005;
pub const STATUS_STREAM_CONFIG_NOTIFICATION: u16 = 0x006;
pub const STATUS_FUNCTION_BLOCK_DISCOVERY: u16 = 0x010;
pub const STATUS_FUNCTION_BLOCK_INFO_NOTIFICATION: u16 = 0x011;
pub const STATUS_FUNCTION_BLOCK_NAME_NOTIFICATION: u16 = 0x012;
pub const STATUS_START_OF_SEQUENCE: u16 = 0x020;
pub const STATUS_END_OF_FILE: u16 = 0x021;

/// Format of multi-packet Stream text notifications (bits 26-27 of word 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StreamTextFormat {
    /// Complete text fits within a single 128-bit UMP packet.
    Complete = 0x0,
    /// Start of a multi-packet text sequence.
    Start = 0x1,
    /// Continuation packet of a multi-packet text sequence.
    Continue = 0x2,
    /// End packet of a multi-packet text sequence.
    End = 0x3,
}

impl StreamTextFormat {
    /// Converts a 2-bit value into a `StreamTextFormat`.
    #[must_use]
    #[inline]
    pub fn from_u8(val: u8) -> Self {
        match val & 0x3 {
            0x0 => StreamTextFormat::Complete,
            0x1 => StreamTextFormat::Start,
            0x2 => StreamTextFormat::Continue,
            0x3 => StreamTextFormat::End,
            _ => unreachable!(),
        }
    }
}

/// Direction of a Function Block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FunctionBlockDirection {
    /// Input only (0x0).
    Input = 0x0,
    /// Output only (0x1).
    Output = 0x1,
    /// Bidirectional (0x2).
    Bidirectional = 0x2,
}

impl FunctionBlockDirection {
    /// Converts a 2-bit value into a `FunctionBlockDirection`.
    #[must_use]
    #[inline]
    pub fn from_u8(val: u8) -> Self {
        match val & 0x3 {
            0x0 => FunctionBlockDirection::Input,
            0x1 => FunctionBlockDirection::Output,
            _ => FunctionBlockDirection::Bidirectional,
        }
    }
}

/// Representation of a MIDI 2.0 Function Block describing internal device topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionBlock {
    /// Function Block index (0-31).
    pub fb_idx: u8,
    /// Whether this Function Block is currently active.
    pub active: bool,
    /// Directionality (Input, Output, Bidirectional).
    pub direction: FunctionBlockDirection,
    /// True if this Function Block can transmit messages.
    pub sender: bool,
    /// True if this Function Block can receive messages.
    pub receiver: bool,
    /// First UMP group spanned by this block (0-15).
    pub first_group: u8,
    /// Number of UMP groups spanned by this block (1-16).
    pub group_length: u8,
    /// Supported MIDI-CI version level (0 = not supported, 1 = v1.1, 2 = v1.2).
    pub midi_ci_version: u8,
    /// 0 = Not restricted, 1 = Restricted to MIDI 1.0 throughput / features.
    pub is_midi1: u8,
    /// Maximum number of concurrent SysEx8 streams supported.
    pub max_s8_streams: u8,
}

impl FunctionBlock {
    /// Creates a new `FunctionBlock` with default active status and sender/receiver configured according to direction.
    #[must_use]
    #[inline]
    pub fn new(
        fb_idx: u8,
        direction: FunctionBlockDirection,
        first_group: u8,
        group_length: u8,
    ) -> Self {
        let (sender, receiver) = match direction {
            FunctionBlockDirection::Input => (false, true),
            FunctionBlockDirection::Output => (true, false),
            FunctionBlockDirection::Bidirectional => (true, true),
        };
        Self {
            fb_idx,
            active: true,
            direction,
            sender,
            receiver,
            first_group,
            group_length,
            midi_ci_version: 0,
            is_midi1: 0,
            max_s8_streams: 0,
        }
    }

    /// Sets the MIDI-CI version supported by this Function Block.
    #[must_use]
    #[inline]
    pub fn midi_ci_version(mut self, version: u8) -> Self {
        self.midi_ci_version = version;
        self
    }

    /// Sets whether this Function Block is restricted to MIDI 1.0.
    #[must_use]
    #[inline]
    pub fn is_midi1(mut self, midi1: u8) -> Self {
        self.is_midi1 = midi1;
        self
    }

    /// Sets the maximum concurrent SysEx8 streams supported.
    #[must_use]
    #[inline]
    pub fn max_s8_streams(mut self, streams: u8) -> Self {
        self.max_s8_streams = streams;
        self
    }

    /// Generates the Function Block Info Notification UMP message (Status=0x011).
    #[must_use]
    #[inline]
    pub fn to_info_notification(&self) -> Ump {
        StreamFactory::function_block_info_notify(
            self.fb_idx,
            self.active,
            self.direction as u8,
            self.sender,
            self.receiver,
            self.first_group,
            self.group_length,
            self.midi_ci_version,
            self.is_midi1,
            self.max_s8_streams,
        )
    }

    /// Generates a Function Block Discovery Request UMP message (Status=0x010).
    #[must_use]
    #[inline]
    pub fn discovery_request(fb_idx: u8, filter_bitmap: u8) -> Ump {
        StreamFactory::function_block_discovery(fb_idx, filter_bitmap)
    }
}

/// Message builder for MIDI 2.0 UMP Stream Messages (MT=0xF).
pub struct StreamFactory;

impl StreamFactory {
    /// Creates an Endpoint Discovery UMP message.
    ///
    /// # Arguments
    /// * `major_version` - UMP Major Version (e.g. 1)
    /// * `minor_version` - UMP Minor Version (e.g. 1)
    /// * `filter_bitmap` - Filter bitmap specifying requested notifications
    #[must_use]
    #[inline]
    pub fn endpoint_discovery(major_version: u8, minor_version: u8, filter_bitmap: u8) -> Ump {
        let w0 = (0xF_u32 << 28)
            | ((STATUS_ENDPOINT_DISCOVERY as u32) << 16)
            | ((major_version as u32) << 8)
            | (minor_version as u32);
        let w1 = (filter_bitmap as u32) << 24;
        Ump {
            data: [w0, w1, 0, 0],
        }
    }

    /// Creates an Endpoint Info Notification UMP message.
    ///
    /// # Arguments
    /// * `major_version` - UMP Major Version
    /// * `minor_version` - UMP Minor Version
    /// * `num_func_blocks` - Number of function blocks
    /// * `midi2_supported` - True if MIDI 2.0 protocol is supported
    /// * `midi1_supported` - True if MIDI 1.0 protocol is supported
    /// * `rxjr_supported` - True if Jitter Reduction Rx is supported
    /// * `txjr_supported` - True if Jitter Reduction Tx is supported
    #[must_use]
    pub fn endpoint_info_notify(
        major_version: u8,
        minor_version: u8,
        num_func_blocks: u8,
        midi2_supported: bool,
        midi1_supported: bool,
        rxjr_supported: bool,
        txjr_supported: bool,
    ) -> Ump {
        let w0 = (0xF_u32 << 28)
            | ((STATUS_ENDPOINT_INFO_NOTIFICATION as u32) << 16)
            | ((major_version as u32) << 8)
            | (minor_version as u32);
        let w1 = ((num_func_blocks as u32) << 24)
            | ((midi2_supported as u32) << 9)
            | ((midi1_supported as u32) << 8)
            | ((rxjr_supported as u32) << 1)
            | (txjr_supported as u32);
        Ump {
            data: [w0, w1, 0, 0],
        }
    }

    /// Creates a Device Identity Notification UMP message.
    ///
    /// # Arguments
    /// * `man_id` - 3-byte System Exclusive Manufacturer ID
    /// * `family_id` - 2-byte Device Family ID
    /// * `model_id` - 2-byte Device Model Number
    /// * `software_revision` - 4-byte Software Revision Level
    #[must_use]
    pub fn endpoint_device_info_notify(
        man_id: [u8; 3],
        family_id: [u8; 2],
        model_id: [u8; 2],
        software_revision: [u8; 4],
    ) -> Ump {
        let w0 = (0xF_u32 << 28) | ((STATUS_DEVICE_IDENTITY_NOTIFICATION as u32) << 16);
        let w1 = ((man_id[0] as u32) << 16) | ((man_id[1] as u32) << 8) | (man_id[2] as u32);
        let w2 = ((family_id[0] as u32) << 24)
            | ((family_id[1] as u32) << 16)
            | ((model_id[0] as u32) << 8)
            | (model_id[1] as u32);
        let w3 = ((software_revision[0] as u32) << 24)
            | ((software_revision[1] as u32) << 16)
            | ((software_revision[2] as u32) << 8)
            | (software_revision[3] as u32);
        Ump {
            data: [w0, w1, w2, w3],
        }
    }

    /// Creates a Stream Configuration Request UMP message.
    ///
    /// # Arguments
    /// * `protocol` - Selected Protocol (1 = MIDI 1.0, 2 = MIDI 2.0)
    /// * `rxjr` - Receive Jitter Reduction enabled
    /// * `txjr` - Transmit Jitter Reduction enabled
    #[must_use]
    #[inline]
    pub fn stream_config_request(protocol: u8, rxjr: bool, txjr: bool) -> Ump {
        let w0 = (0xF_u32 << 28)
            | ((STATUS_STREAM_CONFIG_REQUEST as u32) << 16)
            | ((protocol as u32) << 8)
            | ((rxjr as u32) << 1)
            | (txjr as u32);
        Ump {
            data: [w0, 0, 0, 0],
        }
    }

    /// Creates a Stream Configuration Notification UMP message.
    ///
    /// # Arguments
    /// * `protocol` - Selected Protocol (1 = MIDI 1.0, 2 = MIDI 2.0)
    /// * `rxjr` - Receive Jitter Reduction enabled
    /// * `txjr` - Transmit Jitter Reduction enabled
    #[must_use]
    #[inline]
    pub fn stream_config_notify(protocol: u8, rxjr: bool, txjr: bool) -> Ump {
        let w0 = (0xF_u32 << 28)
            | ((STATUS_STREAM_CONFIG_NOTIFICATION as u32) << 16)
            | ((protocol as u32) << 8)
            | ((rxjr as u32) << 1)
            | (txjr as u32);
        Ump {
            data: [w0, 0, 0, 0],
        }
    }

    /// Creates a Function Block Discovery UMP message.
    ///
    /// # Arguments
    /// * `fb_idx` - Function Block index (0-31), or 0xFF to request all function blocks
    /// * `filter_bitmap` - Filter bitmap specifying requested notifications
    #[must_use]
    #[inline]
    pub fn function_block_discovery(fb_idx: u8, filter_bitmap: u8) -> Ump {
        let w0 = (0xF_u32 << 28)
            | ((STATUS_FUNCTION_BLOCK_DISCOVERY as u32) << 16)
            | ((fb_idx as u32) << 8)
            | (filter_bitmap as u32);
        Ump {
            data: [w0, 0, 0, 0],
        }
    }

    /// Creates a Function Block Info Notification UMP message.
    ///
    /// # Arguments
    /// * `fb_idx` - Function Block index (0-31)
    /// * `active` - Whether the function block is active
    /// * `direction` - 0 = Input, 1 = Output, 2 = Bidirectional
    /// * `sender` - True if sender
    /// * `receiver` - True if receiver
    /// * `first_group` - First UMP group spanned
    /// * `group_length` - Number of groups spanned
    /// * `midi_ci_version` - MIDI-CI support version
    /// * `is_midi1` - 0 = Not restricted, 1 = Restricted to MIDI 1.0
    /// * `max_s8_streams` - Max SysEx8 streams supported
    #[must_use]
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
            | ((STATUS_FUNCTION_BLOCK_INFO_NOTIFICATION as u32) << 16)
            | ((active as u32) << 15)
            | ((fb_idx as u32) << 8)
            | ((sender as u32) << 5)
            | ((receiver as u32) << 4)
            | (((is_midi1 & 0x3) as u32) << 2)
            | ((direction & 0x3) as u32);
        let w1 = ((first_group as u32) << 24)
            | ((group_length as u32) << 16)
            | ((midi_ci_version as u32) << 8)
            | (max_s8_streams as u32);
        Ump {
            data: [w0, w1, 0, 0],
        }
    }

    /// Creates a Start of Sequence UMP message.
    #[must_use]
    #[inline]
    pub fn start_of_sequence() -> Ump {
        Ump {
            data: [
                (0xF_u32 << 28) | ((STATUS_START_OF_SEQUENCE as u32) << 16),
                0,
                0,
                0,
            ],
        }
    }

    /// Creates an End of File UMP message.
    #[must_use]
    #[inline]
    pub fn end_of_file() -> Ump {
        Ump {
            data: [
                (0xF_u32 << 28) | ((STATUS_END_OF_FILE as u32) << 16),
                0,
                0,
                0,
            ],
        }
    }
}

/// Helper function to construct a single 128-bit (MT=0xF) Stream Text Notification UMP.
///
/// For Endpoint Name (0x003) and Product Instance ID (0x004):
/// Word 0 payload space: 2 bytes (bytes [0..1]).
/// Words 1-3 payload space: 4 bytes each. Total packet payload capacity: 14 bytes.
///
/// For Function Block Name (0x012):
/// Word 0 byte 1 holds fb_idx, leaving 1 byte (byte 0) in Word 0.
/// Words 1-3 payload space: 4 bytes each. Total packet payload capacity: 13 bytes.
#[must_use]
pub fn build_stream_text_ump(
    status: u16,
    format: StreamTextFormat,
    fb_idx: Option<u8>,
    bytes: &[u8],
) -> Ump {
    let mut w0 =
        (0xF_u32 << 28) | (((format as u8 as u32) & 0x3) << 26) | (((status as u32) & 0x3FF) << 16);

    let is_fb = fb_idx.is_some();
    if let Some(idx) = fb_idx {
        w0 |= ((idx as u32) & 0x7F) << 8;
    }

    let mut w1 = 0_u32;
    let mut w2 = 0_u32;
    let mut w3 = 0_u32;

    let mut offset = 0;
    let max_bytes = if is_fb { 13 } else { 14 };
    let n = if bytes.len() > max_bytes {
        max_bytes
    } else {
        bytes.len()
    };

    // Pack into Word 0
    if !is_fb && offset < n {
        w0 |= ((bytes[offset] as u32) & 0xFF) << 8;
        offset += 1;
    }
    if offset < n {
        w0 |= (bytes[offset] as u32) & 0xFF;
        offset += 1;
    }

    // Pack into Word 1
    if offset < n {
        w1 |= ((bytes[offset] as u32) & 0xFF) << 24;
        offset += 1;
    }
    if offset < n {
        w1 |= ((bytes[offset] as u32) & 0xFF) << 16;
        offset += 1;
    }
    if offset < n {
        w1 |= ((bytes[offset] as u32) & 0xFF) << 8;
        offset += 1;
    }
    if offset < n {
        w1 |= (bytes[offset] as u32) & 0xFF;
        offset += 1;
    }

    // Pack into Word 2
    if offset < n {
        w2 |= ((bytes[offset] as u32) & 0xFF) << 24;
        offset += 1;
    }
    if offset < n {
        w2 |= ((bytes[offset] as u32) & 0xFF) << 16;
        offset += 1;
    }
    if offset < n {
        w2 |= ((bytes[offset] as u32) & 0xFF) << 8;
        offset += 1;
    }
    if offset < n {
        w2 |= (bytes[offset] as u32) & 0xFF;
        offset += 1;
    }

    // Pack into Word 3
    if offset < n {
        w3 |= ((bytes[offset] as u32) & 0xFF) << 24;
        offset += 1;
    }
    if offset < n {
        w3 |= ((bytes[offset] as u32) & 0xFF) << 16;
        offset += 1;
    }
    if offset < n {
        w3 |= ((bytes[offset] as u32) & 0xFF) << 8;
        offset += 1;
    }
    if offset < n {
        w3 |= (bytes[offset] as u32) & 0xFF;
    }

    Ump {
        data: [w0, w1, w2, w3],
    }
}

/// An iterator that borrows a `&[u8]` text slice and emits zero-allocation MT=0xF UMP text notifications.
#[derive(Debug, Clone)]
pub struct StreamTextChunker<'a> {
    status: u16,
    fb_idx: Option<u8>,
    text: &'a [u8],
    offset: usize,
    finished: bool,
}

impl<'a> StreamTextChunker<'a> {
    /// Creates a chunker for Endpoint Name notification.
    #[must_use]
    #[inline]
    pub fn endpoint_name(text: &'a [u8]) -> Self {
        Self {
            status: STATUS_ENDPOINT_NAME_NOTIFICATION,
            fb_idx: None,
            text,
            offset: 0,
            finished: false,
        }
    }

    /// Creates a chunker for Product Instance ID notification.
    #[must_use]
    #[inline]
    pub fn product_instance_id(text: &'a [u8]) -> Self {
        Self {
            status: STATUS_PRODUCT_INSTANCE_ID_NOTIFICATION,
            fb_idx: None,
            text,
            offset: 0,
            finished: false,
        }
    }

    /// Creates a chunker for Function Block Name notification.
    #[must_use]
    #[inline]
    pub fn function_block_name(fb_idx: u8, text: &'a [u8]) -> Self {
        Self {
            status: STATUS_FUNCTION_BLOCK_NAME_NOTIFICATION,
            fb_idx: Some(fb_idx),
            text,
            offset: 0,
            finished: false,
        }
    }
}

impl<'a> Iterator for StreamTextChunker<'a> {
    type Item = Ump;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let total_len = self.text.len();
        let chunk_cap = if self.fb_idx.is_some() { 13 } else { 14 };

        if total_len == 0 {
            self.finished = true;
            return Some(build_stream_text_ump(
                self.status,
                StreamTextFormat::Complete,
                self.fb_idx,
                &[],
            ));
        }

        let remaining = total_len - self.offset;
        let chunk_size = if remaining > chunk_cap {
            chunk_cap
        } else {
            remaining
        };
        let chunk = &self.text[self.offset..self.offset + chunk_size];
        let is_first = self.offset == 0;
        let is_last = (self.offset + chunk_size) >= total_len;

        let format = match (is_first, is_last) {
            (true, true) => StreamTextFormat::Complete,
            (true, false) => StreamTextFormat::Start,
            (false, false) => StreamTextFormat::Continue,
            (false, true) => StreamTextFormat::End,
        };

        self.offset += chunk_size;
        if is_last {
            self.finished = true;
        }

        Some(build_stream_text_ump(
            self.status,
            format,
            self.fb_idx,
            chunk,
        ))
    }
}

/// Errors occurring during stream text reassembly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamTextReceiverError {
    /// Incoming packet is not an MT=0xF Stream message.
    NotStreamPacket,
    /// Unexpected status code or mismatch with ongoing reassembly.
    StatusMismatch,
    /// Function block index mismatch.
    FunctionBlockMismatch,
    /// Unexpected format sequence (e.g. Continue/End received without Start).
    UnexpectedFormat,
    /// Reassembled text exceeds fixed buffer capacity.
    BufferOverflow,
}

/// State of the `StreamTextReceiver`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamTextReceiverState {
    Idle,
    Receiving,
    Complete,
}

/// A zero-allocation, bounded receiver for collecting MT=0xF multi-packet text notifications.
pub struct StreamTextReceiver<const CAP: usize> {
    buffer: [u8; CAP],
    len: usize,
    active_status: u16,
    active_fb_idx: Option<u8>,
    state: StreamTextReceiverState,
}

impl<const CAP: usize> Default for StreamTextReceiver<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> StreamTextReceiver<CAP> {
    /// Creates a new, empty `StreamTextReceiver` with capacity `CAP`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: [0; CAP],
            len: 0,
            active_status: 0,
            active_fb_idx: None,
            state: StreamTextReceiverState::Idle,
        }
    }

    /// Resets the receiver to idle state.
    pub fn reset(&mut self) {
        self.len = 0;
        self.state = StreamTextReceiverState::Idle;
    }

    /// Returns current state.
    #[must_use]
    #[inline]
    pub fn state(&self) -> StreamTextReceiverState {
        self.state
    }

    /// Returns current active status code.
    #[must_use]
    #[inline]
    pub fn status(&self) -> u16 {
        self.active_status
    }

    /// Returns current function block index (if applicable).
    #[must_use]
    #[inline]
    pub fn function_block_index(&self) -> Option<u8> {
        self.active_fb_idx
    }

    /// Returns accumulated text as byte slice.
    #[must_use]
    #[inline]
    pub fn text(&self) -> &[u8] {
        &self.buffer[..self.len]
    }

    /// Processes an incoming UMP packet.
    pub fn process_ump(
        &mut self,
        ump: &Ump,
    ) -> Result<StreamTextReceiverState, StreamTextReceiverError> {
        if ump.message_type() != MessageType::Stream {
            return Err(StreamTextReceiverError::NotStreamPacket);
        }

        let w0 = ump.data[0];
        let w1 = ump.data[1];
        let w2 = ump.data[2];
        let w3 = ump.data[3];

        let format_bits = ((w0 >> 26) & 0x3) as u8;
        let format = StreamTextFormat::from_u8(format_bits);
        let status = ((w0 >> 16) & 0x3FF) as u16;

        let is_fb = status == STATUS_FUNCTION_BLOCK_NAME_NOTIFICATION;
        let fb_idx = if is_fb {
            Some(((w0 >> 8) & 0x7F) as u8)
        } else {
            None
        };

        // State validation
        match format {
            StreamTextFormat::Complete | StreamTextFormat::Start => {
                self.len = 0;
                self.active_status = status;
                self.active_fb_idx = fb_idx;
            }
            StreamTextFormat::Continue | StreamTextFormat::End => {
                if self.state != StreamTextReceiverState::Receiving {
                    return Err(StreamTextReceiverError::UnexpectedFormat);
                }
                if status != self.active_status {
                    return Err(StreamTextReceiverError::StatusMismatch);
                }
                if fb_idx != self.active_fb_idx {
                    return Err(StreamTextReceiverError::FunctionBlockMismatch);
                }
            }
        }

        // Extract bytes
        let mut temp = [0_u8; 14];
        let mut n = 0;

        if !is_fb {
            let b0 = ((w0 >> 8) & 0xFF) as u8;
            if b0 != 0 {
                temp[n] = b0;
                n += 1;
            }
        }
        let b1 = (w0 & 0xFF) as u8;
        if b1 != 0 {
            temp[n] = b1;
            n += 1;
        }

        // Word 1
        for shift in [24, 16, 8, 0] {
            let b = ((w1 >> shift) & 0xFF) as u8;
            if b != 0 {
                temp[n] = b;
                n += 1;
            }
        }
        // Word 2
        for shift in [24, 16, 8, 0] {
            let b = ((w2 >> shift) & 0xFF) as u8;
            if b != 0 {
                temp[n] = b;
                n += 1;
            }
        }
        // Word 3
        for shift in [24, 16, 8, 0] {
            let b = ((w3 >> shift) & 0xFF) as u8;
            if b != 0 {
                temp[n] = b;
                n += 1;
            }
        }

        if self.len + n > CAP {
            self.reset();
            return Err(StreamTextReceiverError::BufferOverflow);
        }

        self.buffer[self.len..self.len + n].copy_from_slice(&temp[..n]);
        self.len += n;

        match format {
            StreamTextFormat::Complete | StreamTextFormat::End => {
                self.state = StreamTextReceiverState::Complete;
            }
            StreamTextFormat::Start | StreamTextFormat::Continue => {
                self.state = StreamTextReceiverState::Receiving;
            }
        }

        Ok(self.state)
    }
}
