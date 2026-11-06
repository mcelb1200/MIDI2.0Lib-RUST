//! MIDI-CI Property Exchange (PE) & Process Inquiry (PI)
//!
//! Provides no_std, zero-dynamic-allocation types, streaming JSON tokenizer,
//! transaction manager, chunker, and message builders for:
//! - MIDI-CI Property Exchange (Sub-ID 0x30 - 0x3F)
//! - MIDI-CI Process Inquiry (Sub-ID 0x40 - 0x44)

use crate::ci::{pack_7bit_u16, CiHeader};
use crate::sysex::{build_sysex7_ump, build_sysex8_ump, SysEx7Format, SysEx8Format};
use crate::ump::Ump;

// Property Exchange Sub-ID 2 constants
pub const MIDICI_PE_CAPABILITY: u8 = 0x30;
pub const MIDICI_PE_CAPABILITYREPLY: u8 = 0x31;
pub const MIDICI_PE_GET: u8 = 0x34;
pub const MIDICI_PE_GETREPLY: u8 = 0x35;
pub const MIDICI_PE_SET: u8 = 0x36;
pub const MIDICI_PE_SETREPLY: u8 = 0x37;
pub const MIDICI_PE_SUB: u8 = 0x38;
pub const MIDICI_PE_SUBREPLY: u8 = 0x39;
pub const MIDICI_PE_NOTIFY: u8 = 0x3F;

// Process Inquiry Sub-ID 2 constants
pub const MIDICI_PI_CAPABILITY: u8 = 0x40;
pub const MIDICI_PI_CAPABILITYREPLY: u8 = 0x41;
pub const MIDICI_PI_MM_REPORT: u8 = 0x42;
pub const MIDICI_PI_MM_REPORT_REPLY: u8 = 0x43;
pub const MIDICI_PI_MM_REPORT_END: u8 = 0x44;

/// Errors that can occur during Property Exchange serialization or parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeError {
    BufferTooSmall,
    MalformedPayload,
}

// Standard PE Status Codes
pub const PE_STATUS_OK: u16 = 200;
pub const PE_STATUS_ACCEPTED: u16 = 202;
pub const PE_STATUS_RESOURCE_UNAVAILABLE: u16 = 341;
pub const PE_STATUS_BAD_DATA: u16 = 342;
pub const PE_STATUS_TOO_MANY_REQS: u16 = 343;
pub const PE_STATUS_BAD_REQ: u16 = 400;
pub const PE_STATUS_REQ_UNAUTHORIZED: u16 = 403;
pub const PE_STATUS_RESOURCE_UNSUPPORTED: u16 = 404;
pub const PE_STATUS_RESOURCE_NOT_ALLOWED: u16 = 405;
pub const PE_STATUS_PAYLOAD_TOO_LARGE: u16 = 413;
pub const PE_STATUS_UNSUPPORTED_MEDIA_TYPE: u16 = 415;
pub const PE_STATUS_INVALID_DATA_VERSION: u16 = 445;
pub const PE_STATUS_INTERNAL_DEVICE_ERROR: u16 = 500;

/// Token types emitted by the zero-allocation SAX-style JSON tokenizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonTokenType<'a> {
    StartObject,
    EndObject,
    StartArray,
    EndArray,
    Key(&'a str),
    StringValue(&'a str),
    NumberValue(i64),
    BoolValue(bool),
    NullValue,
}

/// A zero-allocation, lightweight SAX-style JSON tokenizer.
///
/// Designed to parse Property Exchange JSON headers and simple bodies directly
/// from byte slices without heap allocation.
#[derive(Debug, Clone)]
pub struct JsonTokenizer<'a> {
    bytes: &'a [u8],
    cursor: usize,
    expect_key: bool,
}

impl<'a> JsonTokenizer<'a> {
    /// Creates a new `JsonTokenizer` over a byte slice.
    #[must_use]
    #[inline]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            cursor: 0,
            expect_key: false,
        }
    }

    fn skip_whitespace(&mut self) {
        while self.cursor < self.bytes.len() {
            match self.bytes[self.cursor] {
                b' ' | b'\t' | b'\n' | b'\r' => {
                    self.cursor += 1;
                }
                _ => break,
            }
        }
    }

    fn parse_string(&mut self) -> Option<&'a str> {
        if self.cursor >= self.bytes.len() || self.bytes[self.cursor] != b'"' {
            return None;
        }
        self.cursor += 1; // skip opening quote
        let start = self.cursor;
        while self.cursor < self.bytes.len() {
            if self.bytes[self.cursor] == b'"'
                && (self.cursor == start || self.bytes[self.cursor - 1] != b'\\')
            {
                let slice = &self.bytes[start..self.cursor];
                self.cursor += 1; // skip closing quote
                return core::str::from_utf8(slice).ok();
            }
            self.cursor += 1;
        }
        None
    }

    fn parse_number(&mut self) -> Option<i64> {
        let start = self.cursor;
        let mut is_negative = false;
        if self.cursor < self.bytes.len() && self.bytes[self.cursor] == b'-' {
            is_negative = true;
            self.cursor += 1;
        }

        let mut val: i64 = 0;
        let num_start = self.cursor;
        while self.cursor < self.bytes.len() && self.bytes[self.cursor].is_ascii_digit() {
            val = val
                .saturating_mul(10)
                .saturating_add((self.bytes[self.cursor] - b'0') as i64);
            self.cursor += 1;
        }

        if self.cursor == num_start {
            self.cursor = start;
            None
        } else if is_negative {
            Some(-val)
        } else {
            Some(val)
        }
    }
}

impl<'a> Iterator for JsonTokenizer<'a> {
    type Item = JsonTokenType<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_whitespace();
        if self.cursor >= self.bytes.len() {
            return None;
        }

        let b = self.bytes[self.cursor];
        match b {
            b'{' => {
                self.cursor += 1;
                self.expect_key = true;
                Some(JsonTokenType::StartObject)
            }
            b'}' => {
                self.cursor += 1;
                self.expect_key = false;
                Some(JsonTokenType::EndObject)
            }
            b'[' => {
                self.cursor += 1;
                self.expect_key = false;
                Some(JsonTokenType::StartArray)
            }
            b']' => {
                self.cursor += 1;
                self.expect_key = false;
                Some(JsonTokenType::EndArray)
            }
            b':' => {
                self.cursor += 1;
                self.expect_key = false;
                self.next()
            }
            b',' => {
                self.cursor += 1;
                self.expect_key = true;
                self.next()
            }
            b'"' => {
                let s = self.parse_string()?;
                if self.expect_key {
                    self.expect_key = false;
                    Some(JsonTokenType::Key(s))
                } else {
                    Some(JsonTokenType::StringValue(s))
                }
            }
            b't' if self.bytes[self.cursor..].starts_with(b"true") => {
                self.cursor += 4;
                Some(JsonTokenType::BoolValue(true))
            }
            b'f' if self.bytes[self.cursor..].starts_with(b"false") => {
                self.cursor += 5;
                Some(JsonTokenType::BoolValue(false))
            }
            b'n' if self.bytes[self.cursor..].starts_with(b"null") => {
                self.cursor += 4;
                Some(JsonTokenType::NullValue)
            }
            b'-' | b'0'..=b'9' => {
                let num = self.parse_number()?;
                Some(JsonTokenType::NumberValue(num))
            }
            _ => {
                self.cursor += 1;
                None
            }
        }
    }
}

/// Parsed Property Exchange Header view.
///
/// Borrows strings and numbers directly from the raw JSON payload with zero allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PeHeader<'a> {
    pub raw_json: &'a [u8],
    pub resource: Option<&'a str>,
    pub res_id: Option<&'a str>,
    pub action: Option<&'a str>,
    pub status: Option<u16>,
    pub cache_time: Option<u32>,
}

impl<'a> PeHeader<'a> {
    /// Parses a JSON byte slice into a `PeHeader`.
    #[must_use]
    pub fn parse(raw_json: &'a [u8]) -> Self {
        let mut header = Self {
            raw_json,
            resource: None,
            res_id: None,
            action: None,
            status: None,
            cache_time: None,
        };

        let mut tokenizer = JsonTokenizer::new(raw_json);
        let mut current_key: Option<&'a str> = None;

        while let Some(tok) = tokenizer.next() {
            match tok {
                JsonTokenType::Key(k) => {
                    current_key = Some(k);
                }
                JsonTokenType::StringValue(s) => {
                    if let Some(k) = current_key {
                        match k {
                            "resource" => header.resource = Some(s),
                            "resId" => header.res_id = Some(s),
                            "action" => header.action = Some(s),
                            _ => {}
                        }
                    }
                    current_key = None;
                }
                JsonTokenType::NumberValue(n) => {
                    if let Some(k) = current_key {
                        match k {
                            "status" if (0..=65535).contains(&n) => {
                                header.status = Some(n as u16);
                            }
                            "cacheTime" if n >= 0 => header.cache_time = Some(n as u32),
                            _ => {}
                        }
                    }
                    current_key = None;
                }
                JsonTokenType::StartObject | JsonTokenType::StartArray => {
                    tokenizer.expect_key = true;
                }
                JsonTokenType::EndObject | JsonTokenType::EndArray => {
                    current_key = None;
                }
                _ => {
                    current_key = None;
                }
            }
        }

        header
    }
}

/// Full Property Exchange message view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeMessage<'a> {
    pub request_id: u8,
    pub header: PeHeader<'a>,
    pub total_chunks: u16,
    pub chunk_num: u16,
    pub body: Option<&'a [u8]>,
}

/// Represents an active, pending Property Exchange Request.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PendingPeRequest {
    pub request_id: u8,
    pub destination_muid: u32,
    pub timestamp: u32,
}

/// Fixed-size transaction manager for tracking asynchronous Request IDs.
///
/// Ensures zero dynamic allocations and prevents Request ID collisions.
pub struct PeTransactionManager<const MAX_CONCURRENT: usize> {
    active_requests: [Option<PendingPeRequest>; MAX_CONCURRENT],
    next_request_id: u8,
}

impl<const MAX_CONCURRENT: usize> Default for PeTransactionManager<MAX_CONCURRENT> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const MAX_CONCURRENT: usize> PeTransactionManager<MAX_CONCURRENT> {
    /// Creates a new `PeTransactionManager`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            active_requests: [None; MAX_CONCURRENT],
            next_request_id: 1,
        }
    }

    /// Allocates a new Request ID for an outbound request to `destination_muid`.
    ///
    /// Returns `None` if the transaction pool is exhausted.
    pub fn start_request(&mut self, destination_muid: u32, timestamp: u32) -> Option<u8> {
        // Find empty slot
        for slot in &mut self.active_requests {
            if slot.is_none() {
                let req_id = self.next_request_id;
                // Increment wrapping around 1..=255 (skip 0)
                if self.next_request_id == 255 {
                    self.next_request_id = 1;
                } else {
                    self.next_request_id += 1;
                }

                *slot = Some(PendingPeRequest {
                    request_id: req_id,
                    destination_muid,
                    timestamp,
                });
                return Some(req_id);
            }
        }
        None
    }

    /// Matches an incoming reply's Request ID, verifying and releasing the transaction slot.
    ///
    /// Returns `true` if a matching pending request was found and completed.
    pub fn complete_request(&mut self, request_id: u8, source_muid: u32) -> bool {
        for slot in &mut self.active_requests {
            if let Some(req) = slot {
                if req.request_id == request_id && req.destination_muid == source_muid {
                    *slot = None;
                    return true;
                }
            }
        }
        false
    }

    /// Clears any transactions that have exceeded `timeout_duration` relative to `current_time`.
    pub fn clean_timeouts(&mut self, current_time: u32, timeout_duration: u32) {
        for slot in &mut self.active_requests {
            if let Some(req) = slot {
                if current_time.saturating_sub(req.timestamp) >= timeout_duration {
                    *slot = None;
                }
            }
        }
    }

    /// Returns the number of currently active pending requests.
    #[must_use]
    pub fn active_count(&self) -> usize {
        let mut count = 0;
        for slot in &self.active_requests {
            if slot.is_some() {
                count += 1;
            }
        }
        count
    }
}

/// Computes a 32-bit FNV-1a hash of a byte slice for zero-allocation resource tracking.
#[must_use]
#[inline]
pub fn hash_resource(resource: &str) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for &b in resource.as_bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

/// Represents an active Property Exchange subscription.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PeSubscription {
    pub subscriber_muid: u32,
    pub resource_hash: u32,
}

/// Fixed-size subscription manager for tracking property subscribers without dynamic memory.
pub struct SubscriptionManager<const MAX_SUBS: usize> {
    subscriptions: [Option<PeSubscription>; MAX_SUBS],
}

impl<const MAX_SUBS: usize> Default for SubscriptionManager<MAX_SUBS> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const MAX_SUBS: usize> SubscriptionManager<MAX_SUBS> {
    /// Creates a new `SubscriptionManager`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            subscriptions: [None; MAX_SUBS],
        }
    }

    /// Adds a subscription for a given subscriber MUID and resource hash.
    ///
    /// Returns `true` if registered or already subscribed, `false` if the subscription table is full.
    pub fn add_subscription(&mut self, subscriber_muid: u32, resource_hash: u32) -> bool {
        // Check if already subscribed
        for s in self.subscriptions.iter().flatten() {
            if s.subscriber_muid == subscriber_muid && s.resource_hash == resource_hash {
                return true;
            }
        }

        // Add to first available slot
        for sub in &mut self.subscriptions {
            if sub.is_none() {
                *sub = Some(PeSubscription {
                    subscriber_muid,
                    resource_hash,
                });
                return true;
            }
        }
        false
    }

    /// Removes a subscription.
    pub fn remove_subscription(&mut self, subscriber_muid: u32, resource_hash: u32) -> bool {
        for sub in &mut self.subscriptions {
            if let Some(s) = sub {
                if s.subscriber_muid == subscriber_muid && s.resource_hash == resource_hash {
                    *sub = None;
                    return true;
                }
            }
        }
        false
    }

    /// Fills `subscribers_out` with MUIDs that are subscribed to `resource_hash`.
    ///
    /// Returns the number of matching subscribers populated.
    pub fn get_subscribers(&self, resource_hash: u32, subscribers_out: &mut [u32]) -> usize {
        let mut count = 0;
        for s in self.subscriptions.iter().flatten() {
            if s.resource_hash == resource_hash && count < subscribers_out.len() {
                subscribers_out[count] = s.subscriber_muid;
                count += 1;
            }
        }
        count
    }

    /// Returns the total active subscriptions count.
    #[must_use]
    pub fn len(&self) -> usize {
        let mut count = 0;
        for sub in &self.subscriptions {
            if sub.is_some() {
                count += 1;
            }
        }
        count
    }

    /// Returns `true` if there are no active subscriptions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Message builder for MIDI-CI Property Exchange messages.
pub struct CiPe;

impl CiPe {
    /// Builds a PE Capability Request payload (14 or 16 bytes).
    #[must_use]
    pub fn build_capability_request(
        source_muid: u32,
        destination_muid: u32,
        ci_version: u8,
        num_simul_requests: u8,
        maj_ver: u8,
        min_ver: u8,
    ) -> ([u8; 16], usize) {
        let mut buf = [0_u8; 16];
        let hdr = CiHeader::new(
            0x7F,
            MIDICI_PE_CAPABILITY,
            ci_version,
            source_muid,
            destination_muid,
        );
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13] = num_simul_requests;
        if ci_version == 1 {
            (buf, 14)
        } else {
            buf[14] = maj_ver;
            buf[15] = min_ver;
            (buf, 16)
        }
    }

    /// Builds a PE Capability Reply payload (14 or 16 bytes).
    #[must_use]
    pub fn build_capability_reply(
        source_muid: u32,
        destination_muid: u32,
        ci_version: u8,
        num_simul_requests: u8,
        maj_ver: u8,
        min_ver: u8,
    ) -> ([u8; 16], usize) {
        let mut buf = [0_u8; 16];
        let hdr = CiHeader::new(
            0x7F,
            MIDICI_PE_CAPABILITYREPLY,
            ci_version,
            source_muid,
            destination_muid,
        );
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13] = num_simul_requests;
        if ci_version == 1 {
            (buf, 14)
        } else {
            buf[14] = maj_ver;
            buf[15] = min_ver;
            (buf, 16)
        }
    }

    /// Serializes a Header-Only PE Message (Get, Sub Reply, Notify, Set Reply) into `buf`.
    ///
    /// Layout:
    /// - 13 bytes CI Header
    /// - 1 byte Request ID
    /// - 2 bytes Header Length (7-bit packed)
    /// - Header data
    /// - 2 bytes Total Chunks = 1 (7-bit packed)
    /// - 2 bytes Chunk Number = 1 (7-bit packed)
    /// - 2 bytes Body Length = 0 (7-bit packed)
    pub fn serialize_pe_header_only(
        buf: &mut [u8],
        ci_type: u8,
        ci_version: u8,
        source_muid: u32,
        destination_muid: u32,
        request_id: u8,
        header: &[u8],
    ) -> Result<usize, PeError> {
        let needed = 13 + 1 + 2 + header.len() + 2 + 2 + 2;
        if buf.len() < needed {
            return Err(PeError::BufferTooSmall);
        }

        let hdr = CiHeader::new(0x7F, ci_type, ci_version, source_muid, destination_muid);
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13] = request_id;

        let mut offset = 14;
        let hdr_len_packed = pack_7bit_u16(header.len() as u16);
        buf[offset..offset + 2].copy_from_slice(&hdr_len_packed);
        offset += 2;

        buf[offset..offset + header.len()].copy_from_slice(header);
        offset += header.len();

        let one_packed = pack_7bit_u16(1);
        let zero_packed = pack_7bit_u16(0);

        buf[offset..offset + 2].copy_from_slice(&one_packed); // total chunks
        offset += 2;
        buf[offset..offset + 2].copy_from_slice(&one_packed); // chunk num
        offset += 2;
        buf[offset..offset + 2].copy_from_slice(&zero_packed); // body length
        offset += 2;

        Ok(offset)
    }

    /// Serializes a PE Message with Body (Set, Sub, Get Reply) into `buf`.
    #[allow(clippy::too_many_arguments)]
    pub fn serialize_pe_with_body(
        buf: &mut [u8],
        ci_type: u8,
        ci_version: u8,
        source_muid: u32,
        destination_muid: u32,
        request_id: u8,
        header: &[u8],
        total_chunks: u16,
        chunk_num: u16,
        body: &[u8],
    ) -> Result<usize, PeError> {
        let needed = 13 + 1 + 2 + header.len() + 2 + 2 + 2 + body.len();
        if buf.len() < needed {
            return Err(PeError::BufferTooSmall);
        }

        let hdr = CiHeader::new(0x7F, ci_type, ci_version, source_muid, destination_muid);
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13] = request_id;

        let mut offset = 14;
        let hdr_len_packed = pack_7bit_u16(header.len() as u16);
        buf[offset..offset + 2].copy_from_slice(&hdr_len_packed);
        offset += 2;

        buf[offset..offset + header.len()].copy_from_slice(header);
        offset += header.len();

        buf[offset..offset + 2].copy_from_slice(&pack_7bit_u16(total_chunks));
        offset += 2;
        buf[offset..offset + 2].copy_from_slice(&pack_7bit_u16(chunk_num));
        offset += 2;
        buf[offset..offset + 2].copy_from_slice(&pack_7bit_u16(body.len() as u16));
        offset += 2;

        buf[offset..offset + body.len()].copy_from_slice(body);
        offset += body.len();

        Ok(offset)
    }

    /// Parses a Property Exchange payload slice.
    #[must_use]
    pub fn parse_pe_message(payload: &[u8]) -> Option<(CiHeader, PeMessage<'_>)> {
        let hdr = CiHeader::parse(payload)?;
        if payload.len() < 16 {
            return None;
        }

        let request_id = payload[13];
        let header_len = crate::ci::unpack_7bit_u16(&payload[14..16]) as usize;
        let header_end = 16 + header_len;
        if payload.len() < header_end + 6 {
            return None;
        }

        let header_slice = &payload[16..header_end];
        let parsed_header = PeHeader::parse(header_slice);

        let total_chunks = crate::ci::unpack_7bit_u16(&payload[header_end..header_end + 2]);
        let chunk_num = crate::ci::unpack_7bit_u16(&payload[header_end + 2..header_end + 4]);
        let body_len =
            crate::ci::unpack_7bit_u16(&payload[header_end + 4..header_end + 6]) as usize;

        let body_start = header_end + 6;
        let body = if body_len > 0 && payload.len() >= body_start + body_len {
            Some(&payload[body_start..body_start + body_len])
        } else {
            None
        };

        Some((
            hdr,
            PeMessage {
                request_id,
                header: parsed_header,
                total_chunks,
                chunk_num,
                body,
            },
        ))
    }
}

/// Chunker format: SysEx7 (MT=0x3) or SysEx8 (MT=0x5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeChunkerTransport {
    SysEx7 { group: u8 },
    SysEx8 { group: u8, stream_id: u8 },
}

/// An iterator that chunks a full Property Exchange serialized buffer into sequential SysEx UMPs.
#[derive(Debug, Clone)]
pub struct PeReplyChunker<'a> {
    transport: PeChunkerTransport,
    payload: &'a [u8],
    offset: usize,
    finished: bool,
}

impl<'a> PeReplyChunker<'a> {
    /// Creates a chunker for SysEx7 (MT=0x3).
    #[must_use]
    pub fn new_sysex7(group: u8, payload: &'a [u8]) -> Self {
        Self {
            transport: PeChunkerTransport::SysEx7 { group: group & 0xF },
            payload,
            offset: 0,
            finished: false,
        }
    }

    /// Creates a chunker for SysEx8 (MT=0x5).
    #[must_use]
    pub fn new_sysex8(group: u8, stream_id: u8, payload: &'a [u8]) -> Self {
        Self {
            transport: PeChunkerTransport::SysEx8 {
                group: group & 0xF,
                stream_id,
            },
            payload,
            offset: 0,
            finished: false,
        }
    }
}

impl<'a> Iterator for PeReplyChunker<'a> {
    type Item = Ump;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let total_len = self.payload.len();
        match self.transport {
            PeChunkerTransport::SysEx7 { group } => {
                if total_len == 0 {
                    self.finished = true;
                    return Some(build_sysex7_ump(group, SysEx7Format::Complete, &[]));
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

                Some(build_sysex7_ump(group, format, chunk))
            }
            PeChunkerTransport::SysEx8 { group, stream_id } => {
                if total_len == 0 {
                    self.finished = true;
                    return Some(build_sysex8_ump(
                        group,
                        stream_id,
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

                Some(build_sysex8_ump(group, stream_id, format, chunk))
            }
        }
    }
}

/// MIDI-CI Process Inquiry (PI) message builders and helpers.
pub struct CiProcessInquiry;

impl CiProcessInquiry {
    /// Builds a Process Inquiry Capabilities Request payload (13 bytes).
    #[must_use]
    pub fn build_capabilities_request(
        source_muid: u32,
        destination_muid: u32,
        ci_version: u8,
    ) -> [u8; 13] {
        let hdr = CiHeader::new(
            0x7F,
            MIDICI_PI_CAPABILITY,
            ci_version,
            source_muid,
            destination_muid,
        );
        hdr.to_bytes()
    }

    /// Builds a Process Inquiry Capabilities Reply payload (14 bytes).
    #[must_use]
    pub fn build_capabilities_reply(
        source_muid: u32,
        destination_muid: u32,
        ci_version: u8,
        supported_features: u8,
    ) -> [u8; 14] {
        let mut buf = [0_u8; 14];
        let hdr = CiHeader::new(
            0x7F,
            MIDICI_PI_CAPABILITYREPLY,
            ci_version,
            source_muid,
            destination_muid,
        );
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13] = supported_features & 0x7F;
        buf
    }

    /// Builds a MIDI Message Report Inquiry payload (18 bytes).
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn build_midi_message_report(
        source_muid: u32,
        destination_muid: u32,
        destination: u8,
        ci_version: u8,
        mdc: u8,
        system_bitmap: u8,
        chan_cont_bitmap: u8,
        chan_note_bitmap: u8,
    ) -> [u8; 18] {
        let mut buf = [0_u8; 18];
        let hdr = CiHeader::new(
            destination & 0x7F,
            MIDICI_PI_MM_REPORT,
            ci_version,
            source_muid,
            destination_muid,
        );
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13] = mdc & 0x7F;
        buf[14] = system_bitmap & 0x7F;
        buf[15] = 0; // reserved
        buf[16] = chan_cont_bitmap & 0x7F;
        buf[17] = chan_note_bitmap & 0x7F;
        buf
    }

    /// Builds a MIDI Message Report Reply payload (17 bytes).
    #[must_use]
    pub fn build_midi_message_report_reply(
        source_muid: u32,
        destination_muid: u32,
        destination: u8,
        ci_version: u8,
        system_bitmap: u8,
        chan_cont_bitmap: u8,
        chan_note_bitmap: u8,
    ) -> [u8; 17] {
        let mut buf = [0_u8; 17];
        let hdr = CiHeader::new(
            destination & 0x7F,
            MIDICI_PI_MM_REPORT_REPLY,
            ci_version,
            source_muid,
            destination_muid,
        );
        buf[0..13].copy_from_slice(&hdr.to_bytes());
        buf[13] = system_bitmap & 0x7F;
        buf[14] = 0; // reserved
        buf[15] = chan_cont_bitmap & 0x7F;
        buf[16] = chan_note_bitmap & 0x7F;
        buf
    }

    /// Builds a MIDI Message Report End payload (13 bytes).
    #[must_use]
    pub fn build_midi_message_report_end(
        source_muid: u32,
        destination_muid: u32,
        destination: u8,
        ci_version: u8,
    ) -> [u8; 13] {
        let hdr = CiHeader::new(
            destination & 0x7F,
            MIDICI_PI_MM_REPORT_END,
            ci_version,
            source_muid,
            destination_muid,
        );
        hdr.to_bytes()
    }
}
