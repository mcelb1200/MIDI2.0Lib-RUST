//! MIDI Capability Inquiry (MIDI-CI) Core Implementation
//!
//! Provides no_std, zero-dynamic-allocation types and state machines for:
//! - Universal SysEx MIDI-CI Header composition and parsing
//! - Discovery Inquiry and Reply messages
//! - Protocol Negotiation messages
//! - Initiator (`CiInitiator`) and Responder (`CiResponder`) state machines

use crate::sysex::SysEx7Chunker;

// Universal SysEx Constants
pub const S7UNIVERSAL_NRT: u8 = 0x7E;
pub const S7UNIVERSAL_RT: u8 = 0x7F;
pub const S7MIDICI: u8 = 0x0D;

// Sub-ID 2: Discovery
pub const MIDICI_DISCOVERY: u8 = 0x70;
pub const MIDICI_DISCOVERYREPLY: u8 = 0x71;
pub const MIDICI_ENDPOINTINFO: u8 = 0x72;
pub const MIDICI_ENDPOINTINFO_REPLY: u8 = 0x73;
pub const MIDICI_ACK: u8 = 0x7D;
pub const MIDICI_INVALIDATEMUID: u8 = 0x7E;
pub const MIDICI_NAK: u8 = 0x7F;

// Sub-ID 2: Protocol Negotiation
pub const MIDICI_PROTOCOL_NEGOTIATION: u8 = 0x10;
pub const MIDICI_PROTOCOL_NEGOTIATION_REPLY: u8 = 0x11;
pub const MIDICI_PROTOCOL_SET: u8 = 0x12;
pub const MIDICI_PROTOCOL_TEST: u8 = 0x13;
pub const MIDICI_PROTOCOL_TEST_RESPONDER: u8 = 0x14;
pub const MIDICI_PROTOCOL_CONFIRM: u8 = 0x15;

// Broadcast MUID (0x7F7F7F7F in 7-bit words, i.e., 28 bits with all 7-bit nibbles set to 0x7F = 0x0FFFFFFF)
pub const M2_CI_BROADCAST: u32 = 0x0FFF_FFFF;

/// CI Protocol Types (5 bytes each)
pub const MIDI_PROTOCOL_1: [u8; 5] = [0x01, 0x00, 0x00, 0x00, 0x00];
pub const MIDI_PROTOCOL_2: [u8; 5] = [0x02, 0x00, 0x00, 0x00, 0x00];

/// Packs a `u16` into 2 7-bit MIDI bytes (LSB first).
#[must_use]
#[inline]
pub fn pack_7bit_u16(val: u16) -> [u8; 2] {
    [(val & 0x7F) as u8, ((val >> 7) & 0x7F) as u8]
}

/// Unpacks 2 7-bit MIDI bytes (LSB first) into a `u16`.
#[must_use]
#[inline]
pub fn unpack_7bit_u16(bytes: &[u8]) -> u16 {
    if bytes.len() < 2 {
        return 0;
    }
    (bytes[0] as u16 & 0x7F) | ((bytes[1] as u16 & 0x7F) << 7)
}

/// Packs a `u32` into 4 7-bit MIDI bytes (LSB first).
#[must_use]
#[inline]
pub fn pack_7bit_u32(val: u32) -> [u8; 4] {
    [
        (val & 0x7F) as u8,
        ((val >> 7) & 0x7F) as u8,
        ((val >> 14) & 0x7F) as u8,
        ((val >> 21) & 0x7F) as u8,
    ]
}

/// Unpacks 4 7-bit MIDI bytes (LSB first) into a `u32`.
#[must_use]
#[inline]
pub fn unpack_7bit_u32(bytes: &[u8]) -> u32 {
    if bytes.len() < 4 {
        return 0;
    }
    (bytes[0] as u32 & 0x7F)
        | ((bytes[1] as u32 & 0x7F) << 7)
        | ((bytes[2] as u32 & 0x7F) << 14)
        | ((bytes[3] as u32 & 0x7F) << 21)
}

/// Universal SysEx MIDI-CI Header (13 bytes total).
///
/// Layout:
/// - Byte 0: Universal Non-Real-Time SysEx (0x7E)
/// - Byte 1: Device ID (0x7F for broadcast / whole device)
/// - Byte 2: MIDI-CI Sub-ID 1 (0x0D)
/// - Byte 3: Sub-ID 2 (CI message type)
/// - Byte 4: CI Version (e.g. 1 or 2)
/// - Bytes 5..8: Source MUID (4 x 7-bit bytes, LSB first)
/// - Bytes 9..12: Destination MUID (4 x 7-bit bytes, LSB first)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CiHeader {
    pub device_id: u8,
    pub ci_type: u8,
    pub ci_version: u8,
    pub source_muid: u32,
    pub destination_muid: u32,
}

impl CiHeader {
    /// Creates a new `CiHeader`.
    #[must_use]
    #[inline]
    pub const fn new(
        device_id: u8,
        ci_type: u8,
        ci_version: u8,
        source_muid: u32,
        destination_muid: u32,
    ) -> Self {
        Self {
            device_id: device_id & 0x7F,
            ci_type,
            ci_version,
            source_muid,
            destination_muid,
        }
    }

    /// Serializes the CI header into a 13-byte array.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 13] {
        let src = pack_7bit_u32(self.source_muid);
        let dst = pack_7bit_u32(self.destination_muid);
        [
            S7UNIVERSAL_NRT,
            self.device_id & 0x7F,
            S7MIDICI,
            self.ci_type,
            self.ci_version,
            src[0],
            src[1],
            src[2],
            src[3],
            dst[0],
            dst[1],
            dst[2],
            dst[3],
        ]
    }

    /// Parses a CI header from a byte slice (must be at least 13 bytes).
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 13 {
            return None;
        }
        if bytes[0] != S7UNIVERSAL_NRT || bytes[2] != S7MIDICI {
            return None;
        }
        Some(Self {
            device_id: bytes[1] & 0x7F,
            ci_type: bytes[3],
            ci_version: bytes[4],
            source_muid: unpack_7bit_u32(&bytes[5..9]),
            destination_muid: unpack_7bit_u32(&bytes[9..13]),
        })
    }
}

/// Device identification and capabilities structure for Discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CiDeviceInfo {
    pub manufacturer_id: [u8; 3],
    pub family_id: [u8; 2],
    pub model_id: [u8; 2],
    pub version: [u8; 4],
    pub ci_support: u8,
    pub max_sysex: u32,
    pub output_path_id: u8,
    pub fb_idx: u8,
}

impl Default for CiDeviceInfo {
    fn default() -> Self {
        Self {
            manufacturer_id: [0; 3],
            family_id: [0; 2],
            model_id: [0; 2],
            version: [0; 4],
            ci_support: 0x07, // Protocol, Profile, Property Exchange support bits
            max_sysex: 512,
            output_path_id: 0,
            fb_idx: 0,
        }
    }
}

/// Builder and generator for MIDI-CI Discovery Inquiry and Discovery Reply messages.
pub struct CiDiscovery;

impl CiDiscovery {
    /// Builds a Discovery Inquiry payload as a 30-byte array (for CI version 2).
    ///
    /// Message size:
    /// - Header: 13 bytes
    /// - Manufacturer: 3 bytes
    /// - Family: 2 bytes
    /// - Model: 2 bytes
    /// - Version: 4 bytes
    /// - CI Support: 1 byte
    /// - Max SysEx: 4 bytes (7-bit packed)
    /// - Output Path ID: 1 byte
    ///
    /// Total = 30 bytes
    #[must_use]
    pub fn build_inquiry_payload(
        source_muid: u32,
        device_info: &CiDeviceInfo,
        ci_version: u8,
    ) -> [u8; 30] {
        let mut buf = [0_u8; 30];
        let hdr = CiHeader::new(
            0x7F,
            MIDICI_DISCOVERY,
            ci_version,
            source_muid,
            M2_CI_BROADCAST,
        );
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13..16].copy_from_slice(&device_info.manufacturer_id);
        buf[16..18].copy_from_slice(&device_info.family_id);
        buf[18..20].copy_from_slice(&device_info.model_id);
        buf[20..24].copy_from_slice(&device_info.version);
        buf[24] = device_info.ci_support & 0x7F;
        let max_sysex_bytes = pack_7bit_u32(device_info.max_sysex);
        buf[25..29].copy_from_slice(&max_sysex_bytes);
        buf[29] = device_info.output_path_id;
        buf
    }

    /// Returns the active length of the Discovery Inquiry payload.
    ///
    /// For CI version 1: 29 bytes (without output_path_id).
    /// For CI version 2: 30 bytes (with output_path_id).
    #[must_use]
    #[inline]
    pub fn inquiry_payload_len(ci_version: u8) -> usize {
        if ci_version < 2 {
            29
        } else {
            30
        }
    }

    /// Builds a Discovery Reply payload as a 31-byte array (for CI version 2).
    ///
    /// Length:
    /// - 30 bytes identical to inquiry + 1 byte fb_idx = 31 bytes
    #[must_use]
    pub fn build_reply_payload(
        source_muid: u32,
        destination_muid: u32,
        device_info: &CiDeviceInfo,
        ci_version: u8,
    ) -> [u8; 31] {
        let mut buf = [0_u8; 31];
        let hdr = CiHeader::new(
            0x7F,
            MIDICI_DISCOVERYREPLY,
            ci_version,
            source_muid,
            destination_muid,
        );
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13..16].copy_from_slice(&device_info.manufacturer_id);
        buf[16..18].copy_from_slice(&device_info.family_id);
        buf[18..20].copy_from_slice(&device_info.model_id);
        buf[20..24].copy_from_slice(&device_info.version);
        buf[24] = device_info.ci_support & 0x7F;
        let max_sysex_bytes = pack_7bit_u32(device_info.max_sysex);
        buf[25..29].copy_from_slice(&max_sysex_bytes);
        buf[29] = device_info.output_path_id;
        buf[30] = device_info.fb_idx;
        buf
    }

    /// Returns the active length of the Discovery Reply payload.
    ///
    /// For CI version 1: 29 bytes.
    /// For CI version 2: 31 bytes.
    #[must_use]
    #[inline]
    pub fn reply_payload_len(ci_version: u8) -> usize {
        if ci_version < 2 {
            29
        } else {
            31
        }
    }

    /// Creates an iterator yielding MT=0x3 UMPs for a Discovery Inquiry.
    #[must_use]
    #[inline]
    pub fn create_inquiry_ump_chunker<'a>(group: u8, payload: &'a [u8]) -> SysEx7Chunker<'a> {
        SysEx7Chunker::new(group, payload)
    }

    /// Parses a Discovery message (Inquiry or Reply) payload from a slice.
    #[must_use]
    pub fn parse_discovery(payload: &[u8]) -> Option<(CiHeader, CiDeviceInfo)> {
        let hdr = CiHeader::parse(payload)?;
        if hdr.ci_type != MIDICI_DISCOVERY && hdr.ci_type != MIDICI_DISCOVERYREPLY {
            return None;
        }
        if payload.len() < 29 {
            return None;
        }

        let mut manu = [0_u8; 3];
        manu.copy_from_slice(&payload[13..16]);
        let mut fam = [0_u8; 2];
        fam.copy_from_slice(&payload[16..18]);
        let mut model = [0_u8; 2];
        model.copy_from_slice(&payload[18..20]);
        let mut ver = [0_u8; 4];
        ver.copy_from_slice(&payload[20..24]);

        let ci_support = payload[24] & 0x7F;
        let max_sysex = unpack_7bit_u32(&payload[25..29]);

        let output_path_id = if payload.len() > 29 { payload[29] } else { 0 };
        let fb_idx = if payload.len() > 30 { payload[30] } else { 0 };

        Some((
            hdr,
            CiDeviceInfo {
                manufacturer_id: manu,
                family_id: fam,
                model_id: model,
                version: ver,
                ci_support,
                max_sysex,
                output_path_id,
                fb_idx,
            },
        ))
    }
}

/// Helper for Protocol Negotiation messages.
pub struct CiProtocolNegotiation;

impl CiProtocolNegotiation {
    /// Builds a Protocol Negotiation Inquiry payload into a fixed 25-byte buffer (supporting up to 2 protocols).
    ///
    /// Header: 13 bytes
    /// Authority Level: 1 byte (byte 13)
    /// Num Protocols: 1 byte (byte 14)
    /// Protocols: num_protocols * 5 bytes
    #[must_use]
    pub fn build_inquiry_payload(
        source_muid: u32,
        destination_muid: u32,
        authority_level: u8,
        protocols: &[[u8; 5]],
    ) -> ([u8; 30], usize) {
        let mut buf = [0_u8; 30];
        let hdr = CiHeader::new(
            0x7F,
            MIDICI_PROTOCOL_NEGOTIATION,
            1,
            source_muid,
            destination_muid,
        );
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13] = authority_level;
        let count = protocols.len().min(3);
        buf[14] = count as u8;
        let mut offset = 15;
        for proto in protocols.iter().take(count) {
            buf[offset..offset + 5].copy_from_slice(proto);
            offset += 5;
        }
        (buf, offset)
    }

    /// Builds a Protocol Negotiation Reply payload.
    #[must_use]
    pub fn build_reply_payload(
        source_muid: u32,
        destination_muid: u32,
        authority_level: u8,
        protocols: &[[u8; 5]],
    ) -> ([u8; 30], usize) {
        let mut buf = [0_u8; 30];
        let hdr = CiHeader::new(
            0x7F,
            MIDICI_PROTOCOL_NEGOTIATION_REPLY,
            1,
            source_muid,
            destination_muid,
        );
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13] = authority_level;
        let count = protocols.len().min(3);
        buf[14] = count as u8;
        let mut offset = 15;
        for proto in protocols.iter().take(count) {
            buf[offset..offset + 5].copy_from_slice(proto);
            offset += 5;
        }
        (buf, offset)
    }

    /// Builds a Set Protocol message payload (19 bytes).
    #[must_use]
    pub fn build_set_protocol_payload(
        source_muid: u32,
        destination_muid: u32,
        authority_level: u8,
        protocol: [u8; 5],
    ) -> [u8; 19] {
        let mut buf = [0_u8; 19];
        let hdr = CiHeader::new(0x7F, MIDICI_PROTOCOL_SET, 1, source_muid, destination_muid);
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13] = authority_level;
        buf[14..19].copy_from_slice(&protocol);
        buf
    }

    /// Builds a Protocol Test message payload (62 bytes: 13 header + 1 authority + 48 test bytes).
    #[must_use]
    pub fn build_protocol_test_payload(
        source_muid: u32,
        destination_muid: u32,
        authority_level: u8,
    ) -> [u8; 62] {
        let mut buf = [0_u8; 62];
        let hdr = CiHeader::new(0x7F, MIDICI_PROTOCOL_TEST, 1, source_muid, destination_muid);
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13] = authority_level;
        for i in 0..48 {
            buf[14 + i] = i as u8;
        }
        buf
    }

    /// Builds a Protocol Test Responder message payload (62 bytes).
    #[must_use]
    pub fn build_protocol_test_responder_payload(
        source_muid: u32,
        destination_muid: u32,
        authority_level: u8,
    ) -> [u8; 62] {
        let mut buf = [0_u8; 62];
        let hdr = CiHeader::new(
            0x7F,
            MIDICI_PROTOCOL_TEST_RESPONDER,
            1,
            source_muid,
            destination_muid,
        );
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13] = authority_level;
        for i in 0..48 {
            buf[14 + i] = i as u8;
        }
        buf
    }

    /// Builds a Protocol Confirm message payload (14 bytes).
    #[must_use]
    pub fn build_protocol_confirm_payload(
        source_muid: u32,
        destination_muid: u32,
        authority_level: u8,
    ) -> [u8; 14] {
        let mut buf = [0_u8; 14];
        let hdr = CiHeader::new(
            0x7F,
            MIDICI_PROTOCOL_CONFIRM,
            1,
            source_muid,
            destination_muid,
        );
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13] = authority_level;
        buf
    }
}

/// State of the `CiInitiator` state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiInitiatorState {
    /// Initialized, ready to start discovery.
    Uninitialized,
    /// Discovery Inquiry has been sent, waiting for reply.
    DiscoverySent,
    /// Discovery Reply has been received from responder.
    DiscoveryReceived,
    /// Protocol Negotiation in progress.
    ProtocolNegotiating,
    /// Negotiated protocol established and confirmed.
    Negotiated,
    /// Handshake failed or rejected.
    Failed,
}

/// MIDI-CI Initiator State Machine.
///
/// Orchestrates the handshake from Discovery to Protocol Negotiation.
pub struct CiInitiator {
    local_muid: u32,
    remote_muid: Option<u32>,
    device_info: CiDeviceInfo,
    remote_device_info: Option<CiDeviceInfo>,
    state: CiInitiatorState,
    negotiated_protocol: Option<[u8; 5]>,
}

/// Error conditions during MIDI-CI message processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiError {
    /// Payload is too short or malformed.
    MalformedPayload,
    /// Header parsing failed.
    InvalidHeader,
}

impl CiInitiator {
    /// Creates a new `CiInitiator`.
    #[must_use]
    pub const fn new(local_muid: u32, device_info: CiDeviceInfo) -> Self {
        Self {
            local_muid,
            remote_muid: None,
            device_info,
            remote_device_info: None,
            state: CiInitiatorState::Uninitialized,
            negotiated_protocol: None,
        }
    }

    /// Returns the current state of the initiator.
    #[must_use]
    #[inline]
    pub fn state(&self) -> CiInitiatorState {
        self.state
    }

    /// Returns the remote responder's MUID once discovered.
    #[must_use]
    #[inline]
    pub fn remote_muid(&self) -> Option<u32> {
        self.remote_muid
    }

    /// Returns the remote responder's device info once discovered.
    #[must_use]
    #[inline]
    pub fn remote_device_info(&self) -> Option<&CiDeviceInfo> {
        self.remote_device_info.as_ref()
    }

    /// Starts Discovery by producing the Discovery Inquiry payload.
    ///
    /// Transitions state to `DiscoverySent`.
    pub fn start_discovery(&mut self, ci_version: u8) -> ([u8; 30], usize) {
        let payload =
            CiDiscovery::build_inquiry_payload(self.local_muid, &self.device_info, ci_version);
        let len = CiDiscovery::inquiry_payload_len(ci_version);
        self.state = CiInitiatorState::DiscoverySent;
        (payload, len)
    }

    /// Processes an incoming SysEx payload for the initiator.
    pub fn process_message(&mut self, payload: &[u8]) -> Result<CiInitiatorState, CiError> {
        let hdr = CiHeader::parse(payload).ok_or(CiError::InvalidHeader)?;

        // Ignore messages not addressed to us (unless broadcast)
        if hdr.destination_muid != self.local_muid && hdr.destination_muid != M2_CI_BROADCAST {
            return Ok(self.state);
        }

        match (self.state, hdr.ci_type) {
            (CiInitiatorState::DiscoverySent, MIDICI_DISCOVERYREPLY) => {
                if let Some((_, remote_info)) = CiDiscovery::parse_discovery(payload) {
                    self.remote_muid = Some(hdr.source_muid);
                    self.remote_device_info = Some(remote_info);
                    self.state = CiInitiatorState::DiscoveryReceived;
                    Ok(self.state)
                } else {
                    Err(CiError::MalformedPayload)
                }
            }
            (
                CiInitiatorState::DiscoveryReceived | CiInitiatorState::ProtocolNegotiating,
                MIDICI_PROTOCOL_NEGOTIATION_REPLY,
            ) => {
                if payload.len() < 20 {
                    return Err(CiError::MalformedPayload);
                }
                let num_protocols = payload[14];
                if num_protocols == 0 {
                    self.state = CiInitiatorState::Failed;
                    return Ok(self.state);
                }
                let mut chosen = [0_u8; 5];
                chosen.copy_from_slice(&payload[15..20]);
                self.negotiated_protocol = Some(chosen);
                self.state = CiInitiatorState::ProtocolNegotiating;
                Ok(self.state)
            }
            (CiInitiatorState::ProtocolNegotiating, MIDICI_PROTOCOL_CONFIRM) => {
                self.state = CiInitiatorState::Negotiated;
                Ok(self.state)
            }
            (CiInitiatorState::ProtocolNegotiating, MIDICI_NAK) => {
                self.state = CiInitiatorState::Failed;
                Ok(self.state)
            }
            _ => Ok(self.state),
        }
    }
}

/// State of the `CiResponder` state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiResponderState {
    /// Idle, waiting for inquiry.
    Idle,
    /// Discovered by an initiator.
    Discovered,
    /// Protocol Negotiation active.
    ProtocolNegotiating,
    /// Protocol confirmed.
    Negotiated,
}

/// MIDI-CI Responder State Machine.
///
/// Handles incoming Discovery Inquiries and Protocol Negotiation requests.
pub struct CiResponder {
    local_muid: u32,
    device_info: CiDeviceInfo,
    supported_protocols: [[u8; 5]; 2],
    supported_count: usize,
    state: CiResponderState,
    remote_muid: Option<u32>,
}

impl CiResponder {
    /// Creates a new `CiResponder`.
    #[must_use]
    pub const fn new(local_muid: u32, device_info: CiDeviceInfo) -> Self {
        Self {
            local_muid,
            device_info,
            supported_protocols: [MIDI_PROTOCOL_2, MIDI_PROTOCOL_1],
            supported_count: 2,
            state: CiResponderState::Idle,
            remote_muid: None,
        }
    }

    /// Returns the current responder state.
    #[must_use]
    #[inline]
    pub fn state(&self) -> CiResponderState {
        self.state
    }

    /// Handles an incoming Discovery Inquiry and generates the Discovery Reply if valid.
    pub fn handle_discovery_inquiry(&mut self, payload: &[u8]) -> Option<([u8; 31], usize)> {
        let (hdr, _) = CiDiscovery::parse_discovery(payload)?;
        if hdr.ci_type != MIDICI_DISCOVERY {
            return None;
        }

        self.remote_muid = Some(hdr.source_muid);
        self.state = CiResponderState::Discovered;

        let reply = CiDiscovery::build_reply_payload(
            self.local_muid,
            hdr.source_muid,
            &self.device_info,
            hdr.ci_version,
        );
        let len = CiDiscovery::reply_payload_len(hdr.ci_version);
        Some((reply, len))
    }

    /// Handles an incoming Protocol Negotiation Inquiry and generates the Reply.
    pub fn handle_protocol_inquiry(&mut self, payload: &[u8]) -> Option<([u8; 30], usize)> {
        let hdr = CiHeader::parse(payload)?;
        if hdr.ci_type != MIDICI_PROTOCOL_NEGOTIATION || payload.len() < 15 {
            return None;
        }

        let authority = payload[13];
        self.state = CiResponderState::ProtocolNegotiating;

        let (reply, len) = CiProtocolNegotiation::build_reply_payload(
            self.local_muid,
            hdr.source_muid,
            authority,
            &self.supported_protocols[..self.supported_count],
        );
        Some((reply, len))
    }
}
