//! Standard PE Resources and Device Identity Provider (Session 7.3)
//!
//! Provides zero-heap, embedded-friendly Property Exchange resource generators for:
//! - `DeviceInfo` (M2-105: Foundational Resources)
//! - `ResourceList` (M2-105)
//! - `ChannelList` (M2-108: Channel Resources)
//!
//! Generates JSON chunks directly into caller-provided byte buffers without dynamic allocation
//! or intermediate heap string assembly.

use crate::ci::CiDeviceInfo;

/// Represents a standard resource identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardResource {
    DeviceInfo,
    ResourceList,
    ChannelList,
    Mode,
}

impl StandardResource {
    /// Matches a resource string to a `StandardResource`.
    #[must_use]
    pub fn parse_name(s: &str) -> Option<Self> {
        match s {
            "DeviceInfo" => Some(StandardResource::DeviceInfo),
            "ResourceList" => Some(StandardResource::ResourceList),
            "ChannelList" => Some(StandardResource::ChannelList),
            "Mode" => Some(StandardResource::Mode),
            _ => None,
        }
    }

    /// Returns the standard resource name string.
    #[must_use]
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            StandardResource::DeviceInfo => "DeviceInfo",
            StandardResource::ResourceList => "ResourceList",
            StandardResource::ChannelList => "ChannelList",
            StandardResource::Mode => "Mode",
        }
    }
}

/// Device identification metadata for Property Exchange `DeviceInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceInfoResource<'a> {
    pub manufacturer: &'a str,
    pub model: &'a str,
    pub serial_number: &'a str,
    pub software_revision: &'a str,
    pub ci_info: CiDeviceInfo,
}

impl<'a> DeviceInfoResource<'a> {
    /// Creates a new `DeviceInfoResource`.
    #[must_use]
    pub const fn new(
        manufacturer: &'a str,
        model: &'a str,
        serial_number: &'a str,
        software_revision: &'a str,
        ci_info: CiDeviceInfo,
    ) -> Self {
        Self {
            manufacturer,
            model,
            serial_number,
            software_revision,
            ci_info,
        }
    }

    /// Procedurally formats the `DeviceInfo` JSON payload into `buf` without heap allocation.
    ///
    /// Returns the number of bytes written, or `None` if `buf` is too small.
    pub fn write_json(&self, buf: &mut [u8]) -> Option<usize> {
        let mut cursor = 0;

        macro_rules! append {
            ($slice:expr) => {{
                let s: &[u8] = $slice;
                if cursor + s.len() > buf.len() {
                    return None;
                }
                buf[cursor..cursor + s.len()].copy_from_slice(s);
                cursor += s.len();
            }};
        }

        append!(b"{\"manufacturer\":\"");
        append!(self.manufacturer.as_bytes());
        append!(b"\",\"model\":\"");
        append!(self.model.as_bytes());
        append!(b"\",\"serialNumber\":\"");
        append!(self.serial_number.as_bytes());
        append!(b"\",\"softwareRevision\":\"");
        append!(self.software_revision.as_bytes());
        append!(b"\",\"maxSysEx\":");

        // Format integer maxSysEx
        let mut num_buf = [0_u8; 10];
        let mut n = self.ci_info.max_sysex;
        let mut idx = 10;
        if n == 0 {
            idx -= 1;
            num_buf[idx] = b'0';
        } else {
            while n > 0 && idx > 0 {
                idx -= 1;
                num_buf[idx] = b'0' + (n % 10) as u8;
                n /= 10;
            }
        }
        append!(&num_buf[idx..10]);

        append!(b"}");
        Some(cursor)
    }
}

/// Generates the standard `ResourceList` JSON array into `buf`.
pub fn write_resource_list_json(resources: &[&str], buf: &mut [u8]) -> Option<usize> {
    let mut cursor = 0;

    macro_rules! append {
        ($slice:expr) => {{
            let s: &[u8] = $slice;
            if cursor + s.len() > buf.len() {
                return None;
            }
            buf[cursor..cursor + s.len()].copy_from_slice(s);
            cursor += s.len();
        }};
    }

    append!(b"[");
    for (i, &res) in resources.iter().enumerate() {
        if i > 0 {
            append!(b",");
        }
        append!(b"{\"resource\":\"");
        append!(res.as_bytes());
        append!(b"\"}");
    }
    append!(b"]");
    Some(cursor)
}

/// Generates the standard `ChannelList` JSON array for active channels into `buf`.
pub fn write_channel_list_json(channel_mask: u16, buf: &mut [u8]) -> Option<usize> {
    let mut cursor = 0;

    macro_rules! append {
        ($slice:expr) => {{
            let s: &[u8] = $slice;
            if cursor + s.len() > buf.len() {
                return None;
            }
            buf[cursor..cursor + s.len()].copy_from_slice(s);
            cursor += s.len();
        }};
    }

    append!(b"[");
    let mut first = true;
    for ch in 0..16 {
        if (channel_mask & (1 << ch)) != 0 {
            if !first {
                append!(b",");
            }
            first = false;
            append!(b"{\"channel\":");
            let mut num_buf = [0_u8; 3];
            let mut n = ch;
            let mut idx = 3;
            if n == 0 {
                idx -= 1;
                num_buf[idx] = b'0';
            } else {
                while n > 0 && idx > 0 {
                    idx -= 1;
                    num_buf[idx] = b'0' + (n % 10) as u8;
                    n /= 10;
                }
            }
            append!(&num_buf[idx..3]);
            append!(b"}");
        }
    }
    append!(b"]");
    Some(cursor)
}
