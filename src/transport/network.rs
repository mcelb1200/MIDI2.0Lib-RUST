//! Network Transports (RTP-MIDI RFC 6295 & Network MIDI 2.0 / AppleMIDI)
//!
//! Provides zero-allocation framing and protocol handlers for transmitting and receiving
//! Universal MIDI Packets (UMP) over UDP/IP network links in `no_std` environments.
//!
//! Features:
//! - AppleMIDI session protocol command encoding/decoding (`IN`, `OK`, `NO`, `BY`, `CK`, `RS`, `RL`).
//! - Session state machine (`AppleMidiSession`) for Initiator and Responder roles.
//! - Clock synchronization (`ClockSync`) with round-trip timestamp calculation and latency offset.
//! - RTP MIDI packet framing (RFC 6295 / Network MIDI 2.0 UDP UMP).
//! - Streaming packetizer and depacketizer with bounds checking and zero heap allocations.
//! - `NetworkTransport` adapter implementing `Transport` and `EmbeddedTransport`.

use crate::transport::{EmbeddedTransport, Transport, TransportError};
use crate::ump::Ump;

/// AppleMIDI command identifiers (2 ASCII bytes)
#[repr(u16)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AppleMidiCommand {
    /// Invitation to establish a session ('IN' = 0x494E)
    Invitation = 0x494E,
    /// Invitation accepted ('OK' = 0x4F4B)
    InvitationAccepted = 0x4F4B,
    /// Invitation rejected ('NO' = 0x4E4F)
    InvitationRejected = 0x4E4F,
    /// End session ('BY' = 0x4259)
    EndSession = 0x4259,
    /// Clock synchronization ('CK' = 0x434B)
    ClockSynchronization = 0x434B,
    /// Receiver feedback / journaling sync ('RS' = 0x5253)
    ReceiverFeedback = 0x5253,
    /// Bitrate receive limit ('RL' = 0x524C)
    BitrateLimit = 0x524C,
    /// Unknown command
    Unknown(u16),
}

impl AppleMidiCommand {
    /// Parses a 16-bit big-endian command code.
    #[must_use]
    #[inline]
    pub const fn from_u16(val: u16) -> Self {
        match val {
            0x494E => Self::Invitation,
            0x4F4B => Self::InvitationAccepted,
            0x4E4F => Self::InvitationRejected,
            0x4259 => Self::EndSession,
            0x434B => Self::ClockSynchronization,
            0x5253 => Self::ReceiverFeedback,
            0x524C => Self::BitrateLimit,
            other => Self::Unknown(other),
        }
    }

    /// Returns the 16-bit big-endian command code.
    #[must_use]
    #[inline]
    pub const fn to_u16(self) -> u16 {
        match self {
            Self::Invitation => 0x494E,
            Self::InvitationAccepted => 0x4F4B,
            Self::InvitationRejected => 0x4E4F,
            Self::EndSession => 0x4259,
            Self::ClockSynchronization => 0x434B,
            Self::ReceiverFeedback => 0x5253,
            Self::BitrateLimit => 0x524C,
            Self::Unknown(val) => val,
        }
    }
}

/// Role of an AppleMIDI participant.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SessionRole {
    /// Initiator opens session by inviting the remote peer.
    Initiator,
    /// Responder listens for incoming session invitations.
    Responder,
}

/// State of an AppleMIDI session.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SessionState {
    /// Session is disconnected.
    Disconnected,
    /// Invitation sent on Control Port, waiting for OK.
    InvitingControl,
    /// Control port accepted; Invitation sent on Data/MIDI Port.
    InvitingData,
    /// Session established and clock synchronization in progress.
    Synchronizing,
    /// Session open and ready for MIDI 2.0 UMP exchange.
    Open,
}

/// AppleMIDI Session message header and payload parser/builder.
///
/// Format (16 bytes minimum):
/// - 0..2: Signature `0xFFFF`
/// - 2..4: Command (`IN`, `OK`, `NO`, `BY`, etc.)
/// - 4..8: Protocol Version (typically 2)
/// - 8..12: Initiator Token (random 32-bit nonce)
/// - 12..16: SSRC (Synchronization Source ID)
/// - 16..: Optional null-terminated UTF-8 session name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleMidiMessage<'a> {
    pub command: AppleMidiCommand,
    pub protocol_version: u32,
    pub initiator_token: u32,
    pub ssrc: u32,
    pub name: Option<&'a str>,
}

impl<'a> AppleMidiMessage<'a> {
    pub const SIGNATURE: u16 = 0xFFFF;
    pub const MIN_LEN: usize = 16;
    pub const PROTOCOL_VERSION: u32 = 2;

    /// Creates a new `AppleMidiMessage`.
    #[must_use]
    pub const fn new(
        command: AppleMidiCommand,
        initiator_token: u32,
        ssrc: u32,
        name: Option<&'a str>,
    ) -> Self {
        Self {
            command,
            protocol_version: Self::PROTOCOL_VERSION,
            initiator_token,
            ssrc,
            name,
        }
    }

    /// Serializes the AppleMIDI session command into `dst`.
    ///
    /// Returns the number of bytes written, or None if `dst` is too small.
    pub fn serialize(&self, dst: &mut [u8]) -> Option<usize> {
        let name_bytes = self.name.map(str::as_bytes).unwrap_or(&[]);
        let total_len = Self::MIN_LEN + name_bytes.len() + usize::from(!name_bytes.is_empty());
        if dst.len() < total_len {
            return None;
        }

        dst[0..2].copy_from_slice(&Self::SIGNATURE.to_be_bytes());
        dst[2..4].copy_from_slice(&self.command.to_u16().to_be_bytes());
        dst[4..8].copy_from_slice(&self.protocol_version.to_be_bytes());
        dst[8..12].copy_from_slice(&self.initiator_token.to_be_bytes());
        dst[12..16].copy_from_slice(&self.ssrc.to_be_bytes());

        if !name_bytes.is_empty() {
            dst[16..16 + name_bytes.len()].copy_from_slice(name_bytes);
            dst[16 + name_bytes.len()] = 0; // Null terminator
        }

        Some(total_len)
    }

    /// Parses an AppleMIDI session command from `src`.
    #[must_use]
    pub fn parse(src: &'a [u8]) -> Option<Self> {
        if src.len() < Self::MIN_LEN {
            return None;
        }

        let sig = u16::from_be_bytes([src[0], src[1]]);
        if sig != Self::SIGNATURE {
            return None;
        }

        let cmd_raw = u16::from_be_bytes([src[2], src[3]]);
        let command = AppleMidiCommand::from_u16(cmd_raw);
        let protocol_version = u32::from_be_bytes([src[4], src[5], src[6], src[7]]);
        let initiator_token = u32::from_be_bytes([src[8], src[9], src[10], src[11]]);
        let ssrc = u32::from_be_bytes([src[12], src[13], src[14], src[15]]);

        let name = if src.len() > Self::MIN_LEN {
            let mut name_end = Self::MIN_LEN;
            while name_end < src.len() && src[name_end] != 0 {
                name_end += 1;
            }
            core::str::from_utf8(&src[Self::MIN_LEN..name_end]).ok()
        } else {
            None
        };

        Some(Self {
            command,
            protocol_version,
            initiator_token,
            ssrc,
            name,
        })
    }
}

/// AppleMIDI Clock Synchronization Packet (`CK`).
///
/// Exchanged over the MIDI Data Port to synchronize 100-microsecond timestamp clocks.
/// Format:
/// - 0..2: Signature `0xFFFF`
/// - 2..4: Command `0x434B` (`CK`)
/// - 4..8: SSRC
/// - 8: Count (number of valid timestamps minus 1: 0, 1, or 2)
/// - 9..12: Padding / Reserved (3 bytes `0x00`)
/// - 12..20: Timestamp 1 (64-bit, 100-microsecond units)
/// - 20..28: Timestamp 2 (64-bit, 100-microsecond units)
/// - 28..36: Timestamp 3 (64-bit, 100-microsecond units)
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ClockSyncPacket {
    pub ssrc: u32,
    pub count: u8,
    pub timestamp1: u64,
    pub timestamp2: u64,
    pub timestamp3: u64,
}

impl ClockSyncPacket {
    pub const PACKET_LEN: usize = 36;

    /// Creates a new `ClockSyncPacket`.
    #[must_use]
    pub const fn new(ssrc: u32, count: u8, t1: u64, t2: u64, t3: u64) -> Self {
        Self {
            ssrc,
            count,
            timestamp1: t1,
            timestamp2: t2,
            timestamp3: t3,
        }
    }

    /// Serializes the clock sync packet into `dst`.
    pub fn serialize(&self, dst: &mut [u8]) -> Option<usize> {
        if dst.len() < Self::PACKET_LEN {
            return None;
        }

        dst[0..2].copy_from_slice(&AppleMidiMessage::SIGNATURE.to_be_bytes());
        dst[2..4].copy_from_slice(
            &AppleMidiCommand::ClockSynchronization
                .to_u16()
                .to_be_bytes(),
        );
        dst[4..8].copy_from_slice(&self.ssrc.to_be_bytes());
        dst[8] = self.count;
        dst[9] = 0;
        dst[10] = 0;
        dst[11] = 0;
        dst[12..20].copy_from_slice(&self.timestamp1.to_be_bytes());
        dst[20..28].copy_from_slice(&self.timestamp2.to_be_bytes());
        dst[28..36].copy_from_slice(&self.timestamp3.to_be_bytes());

        Some(Self::PACKET_LEN)
    }

    /// Parses a clock sync packet from `src`.
    #[must_use]
    pub fn parse(src: &[u8]) -> Option<Self> {
        if src.len() < Self::PACKET_LEN {
            return None;
        }

        let sig = u16::from_be_bytes([src[0], src[1]]);
        let cmd = u16::from_be_bytes([src[2], src[3]]);
        if sig != AppleMidiMessage::SIGNATURE
            || cmd != AppleMidiCommand::ClockSynchronization.to_u16()
        {
            return None;
        }

        let ssrc = u32::from_be_bytes([src[4], src[5], src[6], src[7]]);
        let count = src[8];
        let t1 = u64::from_be_bytes([
            src[12], src[13], src[14], src[15], src[16], src[17], src[18], src[19],
        ]);
        let t2 = u64::from_be_bytes([
            src[20], src[21], src[22], src[23], src[24], src[25], src[26], src[27],
        ]);
        let t3 = u64::from_be_bytes([
            src[28], src[29], src[30], src[31], src[32], src[33], src[34], src[35],
        ]);

        Some(Self {
            ssrc,
            count,
            timestamp1: t1,
            timestamp2: t2,
            timestamp3: t3,
        })
    }
}

/// Clock Synchronizer state and latency estimator.
///
/// Implements RFC 6295 / AppleMIDI clock synchronization formula:
/// `offset_estimate = ((t1 + t3) / 2) - t2`
/// `round_trip_latency = t3 - t1`
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ClockSync {
    pub local_ssrc: u32,
    pub remote_ssrc: u32,
    pub clock_offset: i64,
    pub round_trip_latency: u64,
    pub last_sync_time: u64,
}

impl ClockSync {
    #[must_use]
    pub const fn new(local_ssrc: u32) -> Self {
        Self {
            local_ssrc,
            remote_ssrc: 0,
            clock_offset: 0,
            round_trip_latency: 0,
            last_sync_time: 0,
        }
    }

    /// Evaluates a 3-way synchronization exchange:
    /// - `t1`: Initiator local time when CK0 was sent
    /// - `t2`: Responder local time when CK1 was replied
    /// - `t3`: Initiator local time when CK1 was received
    ///
    /// Computes the estimated clock offset and round-trip network latency.
    #[inline]
    pub fn calculate_offset(&mut self, t1: u64, t2: u64, t3: u64) -> i64 {
        let avg_initiator_time = ((t1 as i128) + (t3 as i128)) / 2;
        let offset = avg_initiator_time - (t2 as i128);
        self.clock_offset = offset as i64;
        self.round_trip_latency = t3.saturating_sub(t1);
        self.last_sync_time = t3;
        self.clock_offset
    }

    /// Converts a remote timestamp into a synchronized local timestamp.
    #[must_use]
    #[inline]
    pub fn remote_to_local_time(&self, remote_time: u64) -> u64 {
        let adjusted = (remote_time as i128) + (self.clock_offset as i128);
        if adjusted < 0 {
            0
        } else {
            adjusted as u64
        }
    }

    /// Converts a local timestamp into a remote-referenced timestamp.
    #[must_use]
    #[inline]
    pub fn local_to_remote_time(&self, local_time: u64) -> u64 {
        let adjusted = (local_time as i128) - (self.clock_offset as i128);
        if adjusted < 0 {
            0
        } else {
            adjusted as u64
        }
    }
}

/// Standard RTP Packet Header (RFC 3550 / RFC 6295).
///
/// Big-endian network format (12 bytes):
/// - 0..2: V (2 bits = 2), P (1 bit), X (1 bit), CC (4 bits), M (1 bit), PT (7 bits, typically 0x61 / 97)
/// - 2..4: Sequence Number (16-bit big-endian)
/// - 4..8: Timestamp (32-bit big-endian)
/// - 8..12: SSRC (32-bit big-endian)
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct RtpMidiHeader {
    pub payload_type: u8,
    pub marker: bool,
    pub sequence_number: u16,
    pub timestamp: u32,
    pub ssrc: u32,
}

impl RtpMidiHeader {
    pub const HEADER_LEN: usize = 12;
    pub const DEFAULT_PAYLOAD_TYPE: u8 = 0x61; // 97 dynamic RTP payload type

    #[must_use]
    pub const fn new(sequence_number: u16, timestamp: u32, ssrc: u32) -> Self {
        Self {
            payload_type: Self::DEFAULT_PAYLOAD_TYPE,
            marker: true,
            sequence_number,
            timestamp,
            ssrc,
        }
    }

    /// Serializes the 12-byte RTP header.
    #[inline]
    pub fn serialize(&self, dst: &mut [u8; 12]) {
        let byte0 = 0x80; // V=2, P=0, X=0, CC=0
        let byte1 = (if self.marker { 0x80 } else { 0x00 }) | (self.payload_type & 0x7F);

        dst[0] = byte0;
        dst[1] = byte1;
        dst[2..4].copy_from_slice(&self.sequence_number.to_be_bytes());
        dst[4..8].copy_from_slice(&self.timestamp.to_be_bytes());
        dst[8..12].copy_from_slice(&self.ssrc.to_be_bytes());
    }

    /// Parses a 12-byte RTP header from `src`.
    #[must_use]
    #[inline]
    pub fn parse(src: &[u8]) -> Option<Self> {
        if src.len() < Self::HEADER_LEN {
            return None;
        }

        let v = (src[0] >> 6) & 0x03;
        if v != 2 {
            return None; // Only RTP Version 2 supported
        }

        let marker = (src[1] & 0x80) != 0;
        let payload_type = src[1] & 0x7F;
        let sequence_number = u16::from_be_bytes([src[2], src[3]]);
        let timestamp = u32::from_be_bytes([src[4], src[5], src[6], src[7]]);
        let ssrc = u32::from_be_bytes([src[8], src[9], src[10], src[11]]);

        Some(Self {
            payload_type,
            marker,
            sequence_number,
            timestamp,
            ssrc,
        })
    }
}

/// Zero-copy Network MIDI 2.0 / RTP UMP Packetizer and Depacketizer.
///
/// In Network MIDI 2.0 (M2-124), packets transferred over UDP encapsulate
/// an RTP header followed immediately by an integral number of 32-bit Universal MIDI Packet (UMP) words.
pub struct NetworkMidiPacket;

impl NetworkMidiPacket {
    /// Packs an RTP header and slice of UMPs into a UDP payload buffer.
    ///
    /// UMP words are encoded in big-endian network byte order.
    /// Returns the number of bytes written to `dst` or None if `dst` is too small.
    pub fn pack_packet(header: &RtpMidiHeader, umps: &[Ump], dst: &mut [u8]) -> Option<usize> {
        if dst.len() < RtpMidiHeader::HEADER_LEN {
            return None;
        }

        let mut rtp_bytes = [0_u8; 12];
        header.serialize(&mut rtp_bytes);
        dst[0..12].copy_from_slice(&rtp_bytes);

        let mut offset = 12;
        for ump in umps {
            let words = ump.word_count();
            let needed = words * 4;
            if offset + needed > dst.len() {
                return None;
            }
            for i in 0..words {
                let word_be = ump.data[i].to_be_bytes();
                dst[offset..offset + 4].copy_from_slice(&word_be);
                offset += 4;
            }
        }

        Some(offset)
    }

    /// Unpacks an RTP header and decodes sequential UMPs from a UDP payload buffer.
    ///
    /// Reads big-endian 32-bit words, converts them to native UMPs, and invokes `callback` per UMP.
    /// Returns the parsed `RtpMidiHeader` and total UMPs decoded, or None on truncation.
    pub fn unpack_packet<F>(src: &[u8], mut callback: F) -> Option<(RtpMidiHeader, usize)>
    where
        F: FnMut(Ump),
    {
        let header = RtpMidiHeader::parse(src)?;
        let mut offset = RtpMidiHeader::HEADER_LEN;
        let mut ump_count = 0;

        while offset + 4 <= src.len() {
            let w0 = u32::from_be_bytes([
                src[offset],
                src[offset + 1],
                src[offset + 2],
                src[offset + 3],
            ]);
            let words = Ump::calculate_word_count(w0);
            let needed = words * 4;
            if offset + needed > src.len() {
                return None; // Incomplete packet / truncated trailing UMP
            }

            let mut data = [0_u32; 4];
            data[0] = w0;
            for (i, word) in data.iter_mut().enumerate().take(words).skip(1) {
                let off = offset + (i * 4);
                *word = u32::from_be_bytes([src[off], src[off + 1], src[off + 2], src[off + 3]]);
            }

            callback(Ump { data });
            ump_count += 1;
            offset += needed;
        }

        Some((header, ump_count))
    }
}

/// AppleMIDI Session State Machine Manager.
///
/// Orchestrates invitation sequences, clock synchronization, and session lifecycles
/// for an embedded device or network node.
#[derive(Debug)]
pub struct AppleMidiSession {
    pub role: SessionRole,
    pub state: SessionState,
    pub local_ssrc: u32,
    pub remote_ssrc: u32,
    pub initiator_token: u32,
    pub clock_sync: ClockSync,
    pub sequence_number: u16,
    pub timestamp_base: u32,
}

impl AppleMidiSession {
    /// Creates a new `AppleMidiSession`.
    #[must_use]
    pub const fn new(role: SessionRole, local_ssrc: u32, initiator_token: u32) -> Self {
        Self {
            role,
            state: SessionState::Disconnected,
            local_ssrc,
            remote_ssrc: 0,
            initiator_token,
            clock_sync: ClockSync::new(local_ssrc),
            sequence_number: 1,
            timestamp_base: 0,
        }
    }

    /// Initiates session connection by generating an Invitation message (`IN`) for the Control Port.
    pub fn initiate_invitation(&mut self, name: Option<&str>, dst: &mut [u8]) -> Option<usize> {
        if self.role != SessionRole::Initiator {
            return None;
        }
        self.state = SessionState::InvitingControl;
        let msg = AppleMidiMessage::new(
            AppleMidiCommand::Invitation,
            self.initiator_token,
            self.local_ssrc,
            name,
        );
        msg.serialize(dst)
    }

    /// Handles an incoming AppleMIDI command packet.
    ///
    /// Returns an optional reply message to be transmitted back to the peer.
    pub fn process_message<'a>(
        &mut self,
        src: &'a [u8],
        local_name: Option<&'a str>,
        reply_buf: &mut [u8],
    ) -> Result<Option<usize>, TransportError> {
        let msg = AppleMidiMessage::parse(src).ok_or(TransportError::HardwareError)?;

        match msg.command {
            AppleMidiCommand::Invitation => match self.state {
                SessionState::Disconnected => {
                    // Accept invitation on Control Port
                    self.remote_ssrc = msg.ssrc;
                    self.initiator_token = msg.initiator_token;
                    self.clock_sync.remote_ssrc = msg.ssrc;
                    self.state = SessionState::InvitingData;

                    let reply = AppleMidiMessage::new(
                        AppleMidiCommand::InvitationAccepted,
                        self.initiator_token,
                        self.local_ssrc,
                        local_name,
                    );
                    Ok(reply.serialize(reply_buf))
                }
                SessionState::InvitingData => {
                    // Accept invitation on Data Port -> Advance to Synchronizing
                    self.state = SessionState::Synchronizing;
                    let reply = AppleMidiMessage::new(
                        AppleMidiCommand::InvitationAccepted,
                        self.initiator_token,
                        self.local_ssrc,
                        local_name,
                    );
                    Ok(reply.serialize(reply_buf))
                }
                _ => {
                    // Already open or synchronizing, reject duplicate
                    let reject = AppleMidiMessage::new(
                        AppleMidiCommand::InvitationRejected,
                        msg.initiator_token,
                        self.local_ssrc,
                        None,
                    );
                    Ok(reject.serialize(reply_buf))
                }
            },
            AppleMidiCommand::InvitationAccepted => match self.state {
                SessionState::InvitingControl => {
                    self.remote_ssrc = msg.ssrc;
                    self.clock_sync.remote_ssrc = msg.ssrc;
                    self.state = SessionState::InvitingData;
                    // Initiator now sends invitation on Data Port
                    let data_invite = AppleMidiMessage::new(
                        AppleMidiCommand::Invitation,
                        self.initiator_token,
                        self.local_ssrc,
                        local_name,
                    );
                    Ok(data_invite.serialize(reply_buf))
                }
                SessionState::InvitingData => {
                    // Both control and data accepted -> Synchronizing / Open
                    self.state = SessionState::Synchronizing;
                    Ok(None)
                }
                _ => Ok(None),
            },
            AppleMidiCommand::InvitationRejected => {
                self.state = SessionState::Disconnected;
                Ok(None)
            }
            AppleMidiCommand::EndSession => {
                self.state = SessionState::Disconnected;
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Processes an incoming Clock Synchronization packet (`CK`).
    ///
    /// Generates the appropriate reply (e.g. CK1 in response to CK0, or CK2 in response to CK1).
    pub fn process_clock_sync(
        &mut self,
        sync: &ClockSyncPacket,
        current_time: u64,
        reply_buf: &mut [u8],
    ) -> Option<usize> {
        match sync.count {
            0 => {
                // Received CK0 from Initiator -> Responder generates CK1
                let reply =
                    ClockSyncPacket::new(self.local_ssrc, 1, sync.timestamp1, current_time, 0);
                self.state = SessionState::Open;
                reply.serialize(reply_buf)
            }
            1 => {
                // Received CK1 from Responder -> Initiator computes offset and sends CK2
                self.clock_sync
                    .calculate_offset(sync.timestamp1, sync.timestamp2, current_time);
                let reply = ClockSyncPacket::new(
                    self.local_ssrc,
                    2,
                    sync.timestamp1,
                    sync.timestamp2,
                    current_time,
                );
                self.state = SessionState::Open;
                reply.serialize(reply_buf)
            }
            2 => {
                // Received CK2 from Initiator -> Responder calculates offset
                self.clock_sync
                    .calculate_offset(sync.timestamp1, sync.timestamp2, sync.timestamp3);
                self.state = SessionState::Open;
                None
            }
            _ => None,
        }
    }

    /// Disconnects the session and serializes an `EndSession` (`BY`) command into `dst`.
    pub fn disconnect(&mut self, dst: &mut [u8]) -> Option<usize> {
        self.state = SessionState::Disconnected;
        let msg = AppleMidiMessage::new(
            AppleMidiCommand::EndSession,
            self.initiator_token,
            self.local_ssrc,
            None,
        );
        msg.serialize(dst)
    }

    /// Next packet sequence number (increments with modulo 2^16).
    #[inline]
    pub fn next_sequence_number(&mut self) -> u16 {
        let seq = self.sequence_number;
        self.sequence_number = self.sequence_number.wrapping_add(1);
        seq
    }
}

/// A zero-allocation Network MIDI 2.0 Transport.
///
/// Encapsulates outbound and inbound UMPs over an abstract network socket or buffer ring,
/// implementing the standard `Transport` and `EmbeddedTransport` interfaces.
#[derive(Debug)]
pub struct NetworkTransport<const CAP: usize> {
    pub session: AppleMidiSession,
    rx_buffer: [Ump; CAP],
    rx_head: usize,
    rx_tail: usize,
    rx_count: usize,
}

impl<const CAP: usize> NetworkTransport<CAP> {
    #[must_use]
    pub const fn new(session: AppleMidiSession) -> Self {
        assert!(CAP > 0, "NetworkTransport capacity must be greater than 0");
        Self {
            session,
            rx_buffer: [Ump { data: [0; 4] }; CAP],
            rx_head: 0,
            rx_tail: 0,
            rx_count: 0,
        }
    }

    /// Ingests a raw UDP datagram containing an RTP MIDI 2.0 packet into the internal ringbuffer.
    pub fn ingest_datagram(&mut self, datagram: &[u8]) -> Result<usize, TransportError> {
        let mut count = 0;
        let unpacked = NetworkMidiPacket::unpack_packet(datagram, |ump| {
            if self.rx_count < CAP {
                self.rx_buffer[self.rx_tail] = ump;
                self.rx_tail = (self.rx_tail + 1) % CAP;
                self.rx_count += 1;
                count += 1;
            }
        });

        if unpacked.is_none() {
            return Err(TransportError::HardwareError);
        }

        Ok(count)
    }

    /// Packs pending UMPs into an outgoing UDP datagram buffer.
    pub fn pack_outbound_datagram(
        &mut self,
        umps: &[Ump],
        timestamp: u32,
        dst: &mut [u8],
    ) -> Option<usize> {
        let seq = self.session.next_sequence_number();
        let header = RtpMidiHeader::new(seq, timestamp, self.session.local_ssrc);
        NetworkMidiPacket::pack_packet(&header, umps, dst)
    }
}

impl<const CAP: usize> Transport for NetworkTransport<CAP> {
    fn send(&mut self, _umps: &[Ump]) -> Result<(), TransportError> {
        if self.session.state != SessionState::Open {
            return Err(TransportError::NotConnected);
        }
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Ump>, TransportError> {
        if self.rx_count == 0 {
            return Ok(None);
        }

        let ump = self.rx_buffer[self.rx_head];
        self.rx_head = (self.rx_head + 1) % CAP;
        self.rx_count -= 1;
        Ok(Some(ump))
    }
}

impl<const CAP: usize> EmbeddedTransport for NetworkTransport<CAP> {
    type Error = TransportError;

    #[inline]
    fn read_ump(&mut self) -> Result<Option<Ump>, Self::Error> {
        self.recv()
    }

    #[inline]
    fn write_ump(&mut self, ump: &Ump) -> Result<(), Self::Error> {
        self.send(core::slice::from_ref(ump))
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        Transport::flush(self)
    }
}
