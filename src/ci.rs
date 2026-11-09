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

// Sub-ID 2: Profile Configuration
pub const MIDICI_PROFILE_INQUIRY: u8 = 0x20;
pub const MIDICI_PROFILE_INQUIRYREPLY: u8 = 0x21;
pub const MIDICI_PROFILE_SETON: u8 = 0x22;
pub const MIDICI_PROFILE_SETOFF: u8 = 0x23;
pub const MIDICI_PROFILE_ENABLED: u8 = 0x24;
pub const MIDICI_PROFILE_DISABLED: u8 = 0x25;
pub const MIDICI_PROFILE_ADD: u8 = 0x26;
pub const MIDICI_PROFILE_REMOVE: u8 = 0x27;
pub const MIDICI_PROFILE_DETAILS_INQUIRY: u8 = 0x28;
pub const MIDICI_PROFILE_DETAILS_REPLY: u8 = 0x29;
pub const MIDICI_PROFILE_SPECIFIC_DATA: u8 = 0x2F;

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

/// Profile Identifier (5 bytes).
///
/// Follows MIDI-CI Profile specifications (M2-102):
/// - Standard Profile: Byte 0 is `0x7E` (standard profile) or non-standard manufacturer ID.
/// - Byte 1: Bank (or standard profile bank)
/// - Byte 2: Profile number / index
/// - Bytes 3..4: Profile version (or manufacturer defined)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileId {
    pub raw: [u8; 5],
}

impl ProfileId {
    /// Creates a new `ProfileId` from a 5-byte array.
    #[must_use]
    #[inline]
    pub const fn new(bytes: [u8; 5]) -> Self {
        Self { raw: bytes }
    }

    /// Creates a Standard MIDI Profile ID (`0x7E`, bank, number, version, level).
    #[must_use]
    #[inline]
    pub const fn standard(bank: u8, number: u8, version: u8, level: u8) -> Self {
        Self {
            raw: [
                0x7E,
                bank & 0x7F,
                number & 0x7F,
                version & 0x7F,
                level & 0x7F,
            ],
        }
    }

    /// Returns `true` if this is a standard profile (`raw[0] == 0x7E`).
    #[must_use]
    #[inline]
    pub const fn is_standard(&self) -> bool {
        self.raw[0] == 0x7E
    }

    /// Returns the raw 5-byte slice.
    #[must_use]
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 5] {
        &self.raw
    }
}

/// Builder and parser for MIDI-CI Profile Configuration messages.
pub struct CiProfile;

impl CiProfile {
    /// Builds a Profile Inquiry payload (13 bytes).
    #[must_use]
    pub fn build_inquiry_payload(
        source_muid: u32,
        destination_muid: u32,
        destination: u8,
        ci_version: u8,
    ) -> [u8; 13] {
        let hdr = CiHeader::new(
            destination & 0x7F,
            MIDICI_PROFILE_INQUIRY,
            ci_version,
            source_muid,
            destination_muid,
        );
        hdr.to_bytes()
    }

    /// Builds a Profile Inquiry Reply payload into a fixed 128-byte buffer.
    ///
    /// Layout:
    /// - Header: 13 bytes
    /// - Enabled Profiles Total Byte Count: 2 bytes (7-bit packed)
    /// - Enabled Profiles: 5 bytes each
    /// - Disabled Profiles Total Byte Count: 2 bytes (7-bit packed)
    /// - Disabled Profiles: 5 bytes each
    #[must_use]
    pub fn build_inquiry_reply_payload(
        source_muid: u32,
        destination_muid: u32,
        destination: u8,
        ci_version: u8,
        enabled_profiles: &[ProfileId],
        disabled_profiles: &[ProfileId],
    ) -> ([u8; 128], usize) {
        let mut buf = [0_u8; 128];
        let hdr = CiHeader::new(
            destination & 0x7F,
            MIDICI_PROFILE_INQUIRYREPLY,
            ci_version,
            source_muid,
            destination_muid,
        );
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        let mut offset = 13;

        // Enabled profiles count (in total bytes = num_profiles * 5)
        let en_byte_len = ((enabled_profiles.len() * 5).min(50)) as u16;
        let en_len_packed = pack_7bit_u16(en_byte_len);
        buf[offset..offset + 2].copy_from_slice(&en_len_packed);
        offset += 2;

        let num_en = (en_byte_len / 5) as usize;
        for p in enabled_profiles.iter().take(num_en) {
            buf[offset..offset + 5].copy_from_slice(p.as_bytes());
            offset += 5;
        }

        // Disabled profiles count (in total bytes = num_profiles * 5)
        let dis_byte_len = ((disabled_profiles.len() * 5).min(50)) as u16;
        let dis_len_packed = pack_7bit_u16(dis_byte_len);
        buf[offset..offset + 2].copy_from_slice(&dis_len_packed);
        offset += 2;

        let num_dis = (dis_byte_len / 5) as usize;
        for p in disabled_profiles.iter().take(num_dis) {
            buf[offset..offset + 5].copy_from_slice(p.as_bytes());
            offset += 5;
        }

        (buf, offset)
    }

    /// Builds a Profile Message (Set Profile On, Set Profile Off, Profile Enabled Report, Profile Disabled Report, Profile Add, Profile Remove).
    ///
    /// For CI v1, payload is 18 bytes (13 header + 5 profile).
    /// For CI v2 (except Add/Remove), payload is 20 bytes (13 header + 5 profile + 2 num_channels packed).
    #[must_use]
    pub fn build_profile_message(
        ci_type: u8,
        source_muid: u32,
        destination_muid: u32,
        destination: u8,
        ci_version: u8,
        profile: ProfileId,
        num_channels: u16,
    ) -> ([u8; 20], usize) {
        let mut buf = [0_u8; 20];
        let hdr = CiHeader::new(
            destination & 0x7F,
            ci_type,
            ci_version,
            source_muid,
            destination_muid,
        );
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13..18].copy_from_slice(profile.as_bytes());

        let len =
            if ci_version == 1 || ci_type == MIDICI_PROFILE_ADD || ci_type == MIDICI_PROFILE_REMOVE
            {
                18
            } else {
                let ch_packed = pack_7bit_u16(num_channels);
                buf[18..20].copy_from_slice(&ch_packed);
                20
            };

        (buf, len)
    }

    /// Builds a Profile Added notification message payload (18 bytes).
    #[must_use]
    #[inline]
    pub fn build_profile_added(
        source_muid: u32,
        destination_muid: u32,
        destination: u8,
        ci_version: u8,
        profile: ProfileId,
    ) -> ([u8; 20], usize) {
        Self::build_profile_message(
            MIDICI_PROFILE_ADD,
            source_muid,
            destination_muid,
            destination,
            ci_version,
            profile,
            0,
        )
    }

    /// Builds a Profile Removed notification message payload (18 bytes).
    #[must_use]
    #[inline]
    pub fn build_profile_removed(
        source_muid: u32,
        destination_muid: u32,
        destination: u8,
        ci_version: u8,
        profile: ProfileId,
    ) -> ([u8; 20], usize) {
        Self::build_profile_message(
            MIDICI_PROFILE_REMOVE,
            source_muid,
            destination_muid,
            destination,
            ci_version,
            profile,
            0,
        )
    }

    /// Builds a Profile Details Inquiry message payload (19 bytes).
    ///
    /// Layout:
    /// - Header: 13 bytes
    /// - Profile ID: 5 bytes
    /// - Inquiry Target: 1 byte
    #[must_use]
    pub fn build_profile_details_inquiry(
        source_muid: u32,
        destination_muid: u32,
        destination: u8,
        ci_version: u8,
        profile: ProfileId,
        inquiry_target: u8,
    ) -> [u8; 19] {
        let mut buf = [0_u8; 19];
        let hdr = CiHeader::new(
            destination & 0x7F,
            MIDICI_PROFILE_DETAILS_INQUIRY,
            ci_version,
            source_muid,
            destination_muid,
        );
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13..18].copy_from_slice(profile.as_bytes());
        buf[18] = inquiry_target & 0x7F;
        buf
    }

    /// Parses a Profile Details Inquiry message payload.
    #[must_use]
    pub fn parse_profile_details_inquiry(payload: &[u8]) -> Option<(CiHeader, ProfileId, u8)> {
        let hdr = CiHeader::parse(payload)?;
        if hdr.ci_type != MIDICI_PROFILE_DETAILS_INQUIRY || payload.len() < 19 {
            return None;
        }
        let mut raw = [0_u8; 5];
        raw.copy_from_slice(&payload[13..18]);
        let profile = ProfileId::new(raw);
        let inquiry_target = payload[18] & 0x7F;
        Some((hdr, profile, inquiry_target))
    }

    /// Builds a Profile Details Reply message payload into a fixed 128-byte buffer.
    ///
    /// Layout:
    /// - Header: 13 bytes
    /// - Profile ID: 5 bytes
    /// - Inquiry Target: 1 byte
    /// - Data Length: 2 bytes (7-bit packed u16)
    /// - Data: variable length
    #[must_use]
    pub fn build_profile_details_reply(
        source_muid: u32,
        destination_muid: u32,
        destination: u8,
        ci_version: u8,
        profile: ProfileId,
        inquiry_target: u8,
        data: &[u8],
    ) -> ([u8; 128], usize) {
        let mut buf = [0_u8; 128];
        let hdr = CiHeader::new(
            destination & 0x7F,
            MIDICI_PROFILE_DETAILS_REPLY,
            ci_version,
            source_muid,
            destination_muid,
        );
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13..18].copy_from_slice(profile.as_bytes());
        buf[18] = inquiry_target & 0x7F;

        let max_data = 128 - 21;
        let datalen = data.len().min(max_data);
        let len_packed = pack_7bit_u16(datalen as u16);
        buf[19..21].copy_from_slice(&len_packed);
        buf[21..21 + datalen].copy_from_slice(&data[..datalen]);

        (buf, 21 + datalen)
    }

    /// Parses a Profile Details Reply message payload.
    #[must_use]
    pub fn parse_profile_details_reply(payload: &[u8]) -> Option<(CiHeader, ProfileId, u8, &[u8])> {
        let hdr = CiHeader::parse(payload)?;
        if hdr.ci_type != MIDICI_PROFILE_DETAILS_REPLY || payload.len() < 21 {
            return None;
        }
        let mut raw = [0_u8; 5];
        raw.copy_from_slice(&payload[13..18]);
        let profile = ProfileId::new(raw);
        let inquiry_target = payload[18] & 0x7F;
        let datalen = unpack_7bit_u16(&payload[19..21]) as usize;
        let data_end = (21 + datalen).min(payload.len());
        Some((hdr, profile, inquiry_target, &payload[21..data_end]))
    }

    /// Builds a Set Profile On message payload.
    #[must_use]
    #[inline]
    pub fn build_set_profile_on(
        source_muid: u32,
        destination_muid: u32,
        destination: u8,
        ci_version: u8,
        profile: ProfileId,
        num_channels: u16,
    ) -> ([u8; 20], usize) {
        Self::build_profile_message(
            MIDICI_PROFILE_SETON,
            source_muid,
            destination_muid,
            destination,
            ci_version,
            profile,
            num_channels,
        )
    }

    /// Builds a Set Profile Off message payload.
    #[must_use]
    #[inline]
    pub fn build_set_profile_off(
        source_muid: u32,
        destination_muid: u32,
        destination: u8,
        ci_version: u8,
        profile: ProfileId,
    ) -> ([u8; 20], usize) {
        Self::build_profile_message(
            MIDICI_PROFILE_SETOFF,
            source_muid,
            destination_muid,
            destination,
            ci_version,
            profile,
            0,
        )
    }

    /// Builds a Profile Enabled Report message payload.
    #[must_use]
    #[inline]
    pub fn build_profile_enabled_report(
        source_muid: u32,
        destination_muid: u32,
        destination: u8,
        ci_version: u8,
        profile: ProfileId,
        num_channels: u16,
    ) -> ([u8; 20], usize) {
        Self::build_profile_message(
            MIDICI_PROFILE_ENABLED,
            source_muid,
            destination_muid,
            destination,
            ci_version,
            profile,
            num_channels,
        )
    }

    /// Builds a Profile Disabled Report message payload.
    #[must_use]
    #[inline]
    pub fn build_profile_disabled_report(
        source_muid: u32,
        destination_muid: u32,
        destination: u8,
        ci_version: u8,
        profile: ProfileId,
        num_channels: u16,
    ) -> ([u8; 20], usize) {
        Self::build_profile_message(
            MIDICI_PROFILE_DISABLED,
            source_muid,
            destination_muid,
            destination,
            ci_version,
            profile,
            num_channels,
        )
    }

    /// Builds a Profile Specific Data message payload into a fixed 128-byte buffer.
    ///
    /// Layout:
    /// - Header: 13 bytes
    /// - Profile ID: 5 bytes
    /// - Length: 4 bytes (7-bit packed u32)
    /// - Data: variable
    #[must_use]
    pub fn build_profile_specific_data(
        source_muid: u32,
        destination_muid: u32,
        destination: u8,
        ci_version: u8,
        profile: ProfileId,
        data: &[u8],
    ) -> ([u8; 128], usize) {
        let mut buf = [0_u8; 128];
        let hdr = CiHeader::new(
            destination & 0x7F,
            MIDICI_PROFILE_SPECIFIC_DATA,
            ci_version,
            source_muid,
            destination_muid,
        );
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13..18].copy_from_slice(profile.as_bytes());

        let datalen = data.len().min(128 - 22);
        let len_packed = pack_7bit_u32(datalen as u32);
        buf[18..22].copy_from_slice(&len_packed);
        buf[22..22 + datalen].copy_from_slice(&data[..datalen]);
        (buf, 22 + datalen)
    }

    /// Parses a Profile message payload (Set On/Off, Enabled/Disabled Report, Add/Remove).
    #[must_use]
    pub fn parse_profile_message(payload: &[u8]) -> Option<(CiHeader, ProfileId, u16)> {
        let hdr = CiHeader::parse(payload)?;
        if payload.len() < 18 {
            return None;
        }
        let mut raw = [0_u8; 5];
        raw.copy_from_slice(&payload[13..18]);
        let profile = ProfileId::new(raw);

        let num_channels = if payload.len() >= 20 {
            unpack_7bit_u16(&payload[18..20])
        } else {
            0
        };

        Some((hdr, profile, num_channels))
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
    /// Profile Configuration in progress.
    ProfileConfiguring,
    /// Handshake failed or rejected.
    Failed,
}

/// MIDI-CI Initiator State Machine.
///
/// Orchestrates the handshake from Discovery to Protocol Negotiation and Profile Configuration.
pub struct CiInitiator {
    local_muid: u32,
    remote_muid: Option<u32>,
    device_info: CiDeviceInfo,
    remote_device_info: Option<CiDeviceInfo>,
    state: CiInitiatorState,
    negotiated_protocol: Option<[u8; 5]>,
    enabled_profiles: [Option<ProfileId>; 8],
    enabled_profiles_count: usize,
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
            enabled_profiles: [None; 8],
            enabled_profiles_count: 0,
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
            (
                CiInitiatorState::Negotiated | CiInitiatorState::ProfileConfiguring,
                MIDICI_PROFILE_ENABLED,
            ) => {
                if let Some((_, profile, _)) = CiProfile::parse_profile_message(payload) {
                    if self.enabled_profiles_count < 8 {
                        self.enabled_profiles[self.enabled_profiles_count] = Some(profile);
                        self.enabled_profiles_count += 1;
                    }
                    self.state = CiInitiatorState::ProfileConfiguring;
                    Ok(self.state)
                } else {
                    Err(CiError::MalformedPayload)
                }
            }
            (
                CiInitiatorState::Negotiated | CiInitiatorState::ProfileConfiguring,
                MIDICI_PROFILE_DISABLED,
            ) => {
                if let Some((_, profile, _)) = CiProfile::parse_profile_message(payload) {
                    for i in 0..self.enabled_profiles_count {
                        if self.enabled_profiles[i] == Some(profile) {
                            self.enabled_profiles[i] =
                                self.enabled_profiles[self.enabled_profiles_count - 1];
                            self.enabled_profiles[self.enabled_profiles_count - 1] = None;
                            self.enabled_profiles_count -= 1;
                            break;
                        }
                    }
                    self.state = CiInitiatorState::ProfileConfiguring;
                    Ok(self.state)
                } else {
                    Err(CiError::MalformedPayload)
                }
            }
            (
                CiInitiatorState::Negotiated | CiInitiatorState::ProfileConfiguring,
                MIDICI_PROFILE_ADD | MIDICI_PROFILE_REMOVE | MIDICI_PROFILE_DETAILS_REPLY,
            ) => {
                self.state = CiInitiatorState::ProfileConfiguring;
                Ok(self.state)
            }
            _ => Ok(self.state),
        }
    }

    /// Returns a slice of the enabled profiles.
    #[must_use]
    pub fn enabled_profiles(&self) -> &[Option<ProfileId>] {
        &self.enabled_profiles[..self.enabled_profiles_count]
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
    /// Profile Configuration active.
    ProfileConfiguring,
}

/// MIDI-CI Responder State Machine.
///
/// Handles incoming Discovery Inquiries, Protocol Negotiation, and Profile Configuration requests.
pub struct CiResponder {
    local_muid: u32,
    device_info: CiDeviceInfo,
    supported_protocols: [[u8; 5]; 2],
    supported_count: usize,
    supported_profiles: [Option<ProfileId>; 8],
    supported_profiles_count: usize,
    enabled_profiles: [Option<ProfileId>; 8],
    enabled_profiles_count: usize,
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
            supported_profiles: [None; 8],
            supported_profiles_count: 0,
            enabled_profiles: [None; 8],
            enabled_profiles_count: 0,
            state: CiResponderState::Idle,
            remote_muid: None,
        }
    }

    /// Adds a supported profile to the responder.
    pub fn add_supported_profile(&mut self, profile: ProfileId) -> bool {
        if self.supported_profiles_count < 8 {
            self.supported_profiles[self.supported_profiles_count] = Some(profile);
            self.supported_profiles_count += 1;
            true
        } else {
            false
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

    /// Handles an incoming Profile Inquiry and generates the Profile Inquiry Reply.
    pub fn handle_profile_inquiry(&mut self, payload: &[u8]) -> Option<([u8; 128], usize)> {
        let hdr = CiHeader::parse(payload)?;
        if hdr.ci_type != MIDICI_PROFILE_INQUIRY {
            return None;
        }

        self.state = CiResponderState::ProfileConfiguring;

        let mut enabled = [ProfileId::new([0; 5]); 8];
        let mut en_count = 0;
        for p in self.enabled_profiles[..self.enabled_profiles_count]
            .iter()
            .flatten()
        {
            enabled[en_count] = *p;
            en_count += 1;
        }

        let mut disabled = [ProfileId::new([0; 5]); 8];
        let mut dis_count = 0;
        for p in self.supported_profiles[..self.supported_profiles_count]
            .iter()
            .flatten()
        {
            // If not in enabled
            let is_enabled = enabled[..en_count].contains(p);
            if !is_enabled {
                disabled[dis_count] = *p;
                dis_count += 1;
            }
        }

        let (reply, len) = CiProfile::build_inquiry_reply_payload(
            self.local_muid,
            hdr.source_muid,
            hdr.device_id,
            hdr.ci_version,
            &enabled[..en_count],
            &disabled[..dis_count],
        );

        Some((reply, len))
    }

    /// Handles an incoming Set Profile On request, enabling the profile if supported and generating a Profile Enabled Report.
    pub fn handle_set_profile_on(&mut self, payload: &[u8]) -> Option<([u8; 20], usize)> {
        let (hdr, profile, num_channels) = CiProfile::parse_profile_message(payload)?;
        if hdr.ci_type != MIDICI_PROFILE_SETON {
            return None;
        }

        // Check if profile is supported
        let supported =
            self.supported_profiles[..self.supported_profiles_count].contains(&Some(profile));

        if !supported {
            return None;
        }

        // Add to enabled if not already enabled
        let already_enabled =
            self.enabled_profiles[..self.enabled_profiles_count].contains(&Some(profile));

        if !already_enabled && self.enabled_profiles_count < 8 {
            self.enabled_profiles[self.enabled_profiles_count] = Some(profile);
            self.enabled_profiles_count += 1;
        }

        self.state = CiResponderState::ProfileConfiguring;

        let (report, len) = CiProfile::build_profile_enabled_report(
            self.local_muid,
            hdr.source_muid,
            hdr.device_id,
            hdr.ci_version,
            profile,
            num_channels,
        );

        Some((report, len))
    }

    /// Handles an incoming Set Profile Off request, disabling the profile and generating a Profile Disabled Report.
    pub fn handle_set_profile_off(&mut self, payload: &[u8]) -> Option<([u8; 20], usize)> {
        let (hdr, profile, num_channels) = CiProfile::parse_profile_message(payload)?;
        if hdr.ci_type != MIDICI_PROFILE_SETOFF {
            return None;
        }

        for i in 0..self.enabled_profiles_count {
            if self.enabled_profiles[i] == Some(profile) {
                self.enabled_profiles[i] = self.enabled_profiles[self.enabled_profiles_count - 1];
                self.enabled_profiles[self.enabled_profiles_count - 1] = None;
                self.enabled_profiles_count -= 1;
                break;
            }
        }

        self.state = CiResponderState::ProfileConfiguring;

        let (report, len) = CiProfile::build_profile_disabled_report(
            self.local_muid,
            hdr.source_muid,
            hdr.device_id,
            hdr.ci_version,
            profile,
            num_channels,
        );

        Some((report, len))
    }

    /// Handles an incoming Profile Details Inquiry request, returning Profile Details Reply if supported.
    pub fn handle_profile_details_inquiry(
        &mut self,
        payload: &[u8],
        details_data: &[u8],
    ) -> Option<([u8; 128], usize)> {
        let (hdr, profile, target) = CiProfile::parse_profile_details_inquiry(payload)?;
        if hdr.ci_type != MIDICI_PROFILE_DETAILS_INQUIRY {
            return None;
        }

        self.state = CiResponderState::ProfileConfiguring;

        let (reply, len) = CiProfile::build_profile_details_reply(
            self.local_muid,
            hdr.source_muid,
            hdr.device_id,
            hdr.ci_version,
            profile,
            target,
            details_data,
        );

        Some((reply, len))
    }
}
