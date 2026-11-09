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
    ProgramList,
    LocalOn,
    DeviceState,
    ExternalSync,
    ControllerList,
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
            "ProgramList" => Some(StandardResource::ProgramList),
            "LocalOn" => Some(StandardResource::LocalOn),
            "DeviceState" => Some(StandardResource::DeviceState),
            "ExternalSync" => Some(StandardResource::ExternalSync),
            "ControllerList" => Some(StandardResource::ControllerList),
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
            StandardResource::ProgramList => "ProgramList",
            StandardResource::LocalOn => "LocalOn",
            StandardResource::DeviceState => "DeviceState",
            StandardResource::ExternalSync => "ExternalSync",
            StandardResource::ControllerList => "ControllerList",
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

/// Mode Resource (M2-106).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceMode {
    Standard,
    Mpe,
    MultiChannel,
}

impl DeviceMode {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            DeviceMode::Standard => "Standard",
            DeviceMode::Mpe => "MPE",
            DeviceMode::MultiChannel => "MultiChannel",
        }
    }
}

/// Procedurally writes the Mode resource JSON: `{"mode":"Standard"}` into `buf`.
pub fn write_mode_json(mode: DeviceMode, buf: &mut [u8]) -> Option<usize> {
    let mode_str = mode.as_str().as_bytes();
    let prefix = b"{\"mode\":\"";
    let suffix = b"\"}";
    let total = prefix.len() + mode_str.len() + suffix.len();
    if buf.len() < total {
        return None;
    }
    buf[0..prefix.len()].copy_from_slice(prefix);
    buf[prefix.len()..prefix.len() + mode_str.len()].copy_from_slice(mode_str);
    buf[prefix.len() + mode_str.len()..total].copy_from_slice(suffix);
    Some(total)
}

/// Program metadata for ProgramList resource (M2-107).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramItem<'a> {
    pub program: u8,
    pub bank_msb: u8,
    pub bank_lsb: u8,
    pub name: &'a str,
}

/// Procedurally streams a list of `ProgramItem`s into `buf` as a JSON array without heap allocation.
pub fn write_program_list_json(programs: &[ProgramItem<'_>], buf: &mut [u8]) -> Option<usize> {
    write_paginated_program_list_json(programs, 0, programs.len(), buf)
}

/// Procedurally streams a paginated slice of `ProgramItem`s into `buf` as a JSON array without heap allocation.
pub fn write_paginated_program_list_json(
    programs: &[ProgramItem<'_>],
    offset: usize,
    limit: usize,
    buf: &mut [u8],
) -> Option<usize> {
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
    for (idx, p) in programs.iter().skip(offset).take(limit).enumerate() {
        if idx > 0 {
            append!(b",");
        }
        append!(b"{\"program\":");
        let mut num_buf = [0_u8; 3];
        let mut n = p.program;
        let mut idx = 3;
        if n == 0 {
            idx -= 1;
            num_buf[idx] = b'0';
        } else {
            while n > 0 && idx > 0 {
                idx -= 1;
                num_buf[idx] = b'0' + (n % 10);
                n /= 10;
            }
        }
        append!(&num_buf[idx..3]);

        append!(b",\"bank\":");
        let bank = ((p.bank_msb as u16) << 7) | (p.bank_lsb as u16);
        let mut b_buf = [0_u8; 5];
        let mut bn = bank;
        let mut b_idx = 5;
        if bn == 0 {
            b_idx -= 1;
            b_buf[b_idx] = b'0';
        } else {
            while bn > 0 && b_idx > 0 {
                b_idx -= 1;
                b_buf[b_idx] = b'0' + (bn % 10) as u8;
                bn /= 10;
            }
        }
        append!(&b_buf[b_idx..5]);

        append!(b",\"title\":\"");
        append!(p.name.as_bytes());
        append!(b"\"}");
    }
    append!(b"]");
    Some(cursor)
}

/// Procedurally writes the LocalOn resource JSON (M2-109): `{"localOn":true}` into `buf`.
pub fn write_local_on_json(local_on: bool, buf: &mut [u8]) -> Option<usize> {
    let s: &[u8] = if local_on {
        b"{\"localOn\":true}"
    } else {
        b"{\"localOn\":false}"
    };
    if buf.len() < s.len() {
        return None;
    }
    buf[..s.len()].copy_from_slice(s);
    Some(s.len())
}

/// External Sync clock source options (M2-112).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncSource {
    Internal,
    Midi,
    Smpte,
    Audio,
}

impl SyncSource {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            SyncSource::Internal => "Internal",
            SyncSource::Midi => "MIDI",
            SyncSource::Smpte => "SMPTE",
            SyncSource::Audio => "Audio",
        }
    }
}

/// Procedurally writes the ExternalSync resource JSON (M2-112): `{"source":"Internal","active":true}` into `buf`.
pub fn write_external_sync_json(source: SyncSource, active: bool, buf: &mut [u8]) -> Option<usize> {
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

    append!(b"{\"source\":\"");
    append!(source.as_str().as_bytes());
    append!(b"\",\"active\":");
    if active {
        append!(b"true}");
    } else {
        append!(b"false}");
    }
    Some(cursor)
}

/// Controller definition for ControllerList resource (M2-117).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControllerDefinition<'a> {
    pub index: u8,
    pub name: &'a str,
    pub default_val: u32,
    pub unit: &'a str,
}

/// Procedurally writes the ControllerList resource JSON (M2-117) into `buf`.
pub fn write_controller_list_json(
    controllers: &[ControllerDefinition<'_>],
    buf: &mut [u8],
) -> Option<usize> {
    write_paginated_controller_list_json(controllers, 0, controllers.len(), buf)
}

/// Procedurally writes a paginated slice of ControllerList resource JSON (M2-117) into `buf`.
pub fn write_paginated_controller_list_json(
    controllers: &[ControllerDefinition<'_>],
    offset: usize,
    limit: usize,
    buf: &mut [u8],
) -> Option<usize> {
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
    for (idx, c) in controllers.iter().skip(offset).take(limit).enumerate() {
        if idx > 0 {
            append!(b",");
        }
        append!(b"{\"index\":");
        let mut num_buf = [0_u8; 3];
        let mut n = c.index;
        let mut idx = 3;
        if n == 0 {
            idx -= 1;
            num_buf[idx] = b'0';
        } else {
            while n > 0 && idx > 0 {
                idx -= 1;
                num_buf[idx] = b'0' + (n % 10);
                n /= 10;
            }
        }
        append!(&num_buf[idx..3]);

        append!(b",\"name\":\"");
        append!(c.name.as_bytes());
        append!(b"\",\"unit\":\"");
        append!(c.unit.as_bytes());
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
