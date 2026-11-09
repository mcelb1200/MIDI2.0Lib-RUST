//! VideoCore Mailbox 0 Interface (ARM <- VC)
//!
//! Provides bare-metal property tag messaging to communicate with the VideoCore IV GPU firmware.
//! Used in MIDI 2.0 to:
//! - Lock the UART reference clock (`UART_CLK`) to exactly 48,000,000 Hz.
//! - Retrieve the hardware MAC address for Network MIDI 2.0 / AppleMIDI endpoint naming.

use crate::mmio::MAILBOX0_BASE;

/// BCM2837 Mailbox 0 Register Offsets
pub mod reg {
    /// Read register (offset 0x00)
    pub const READ: usize = 0x00;
    /// Status register (offset 0x18)
    pub const STATUS: usize = 0x18;
    /// Config register (offset 0x1C)
    pub const CONFIG: usize = 0x1C;
    /// Write register (offset 0x20)
    pub const WRITE: usize = 0x20;
}

/// Mailbox Status Flags
pub mod status {
    /// Mailbox is full (bit 31)
    pub const FULL: u32 = 0x8000_0000;
    /// Mailbox is empty (bit 30)
    pub const EMPTY: u32 = 0x4000_0000;
}

/// VideoCore Mailbox Channel identifiers
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum MailboxChannel {
    Power = 0,
    Framebuffer = 1,
    VirtualUart = 2,
    Vchiq = 3,
    Leds = 4,
    Buttons = 5,
    Touchscreen = 6,
    PropertyTagsArmToVc = 8,
    PropertyTagsVcToArm = 9,
}

/// Standard VideoCore Clock IDs
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum MailboxClockId {
    Emmc = 0x01,
    Uart = 0x02,
    Arm = 0x03,
    Core = 0x04,
    V3d = 0x05,
    H264 = 0x06,
    Isp = 0x07,
    Sdram = 0x08,
    Pixel = 0x09,
    Pwm = 0x0A,
}

/// Common Mailbox Property Tags
pub mod tags {
    /// Firmware revision tag
    pub const TAG_GET_FIRMWARE_REVISION: u32 = 0x0000_0001;
    /// Board model tag
    pub const TAG_GET_BOARD_MODEL: u32 = 0x0001_0001;
    /// Board revision tag
    pub const TAG_GET_BOARD_REVISION: u32 = 0x0001_0002;
    /// Hardware MAC address tag
    pub const TAG_GET_MAC_ADDRESS: u32 = 0x0001_0003;
    /// Board serial tag
    pub const TAG_GET_BOARD_SERIAL: u32 = 0x0001_0004;
    /// Get clock rate tag
    pub const TAG_GET_CLOCK_RATE: u32 = 0x0003_0002;
    /// Set clock rate tag
    pub const TAG_SET_CLOCK_RATE: u32 = 0x0003_8002;
    /// End of tags list
    pub const TAG_END: u32 = 0x0000_0000;
}

pub use tags as MailboxTag;

/// Driver for BCM2837 VideoCore Mailbox 0 interface
#[derive(Debug, Copy, Clone)]
pub struct Rpi3Mailbox {
    base_addr: usize,
}

impl Default for Rpi3Mailbox {
    fn default() -> Self {
        Self::new()
    }
}

impl Rpi3Mailbox {
    /// Creates a driver instance pointing to standard MMIO base.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            base_addr: MAILBOX0_BASE,
        }
    }

    /// Creates a driver instance with a custom base address.
    #[must_use]
    pub const fn with_base(base_addr: usize) -> Self {
        Self { base_addr }
    }

    /// Formats a property request buffer to set the UART reference clock rate.
    ///
    /// Locks UART clock (Clock ID 0x02) to `rate_hz` (e.g. 48,000,000 Hz) without skipping turbo.
    /// Returns the number of words written to `buf`.
    pub fn build_set_clock_rate_buffer(
        buf: &mut [u32; 12],
        clock_id: MailboxClockId,
        rate_hz: u32,
        skip_turbo: bool,
    ) -> usize {
        buf[0] = 12 * 4; // Buffer size in bytes
        buf[1] = 0x0000_0000; // Process request code
        buf[2] = tags::TAG_SET_CLOCK_RATE; // Tag ID
        buf[3] = 12; // Value buffer size in bytes
        buf[4] = 0; // Request/response indicator
        buf[5] = clock_id as u32; // Value 1: Clock ID
        buf[6] = rate_hz; // Value 2: Clock rate in Hz
        buf[7] = if skip_turbo { 1 } else { 0 }; // Value 3: Skip turbo setting
        buf[8] = tags::TAG_END; // End tag
        buf[9] = 0;
        buf[10] = 0;
        buf[11] = 0;
        12
    }

    /// Formats a property request buffer to acquire the hardware MAC address.
    /// Returns the number of words written to `buf`.
    pub fn build_get_mac_address_buffer(buf: &mut [u32; 8]) -> usize {
        buf[0] = 8 * 4; // Buffer size in bytes
        buf[1] = 0x0000_0000; // Process request code
        buf[2] = tags::TAG_GET_MAC_ADDRESS; // Tag ID
        buf[3] = 6; // Value buffer size (6 bytes for MAC)
        buf[4] = 0; // Request/response indicator
        buf[5] = 0; // Space for MAC[0..3]
        buf[6] = 0; // Space for MAC[4..5]
        buf[7] = tags::TAG_END;
        8
    }

    /// Parses the MAC address from a processed Mailbox response buffer.
    pub fn parse_mac_address(buf: &[u32; 8]) -> Option<[u8; 6]> {
        // buf[1] is 0x8000_0000 on success
        if buf[1] != 0x8000_0000 {
            return None;
        }
        let w0 = buf[5];
        let w1 = buf[6];
        Some([
            (w0 & 0xFF) as u8,
            ((w0 >> 8) & 0xFF) as u8,
            ((w0 >> 16) & 0xFF) as u8,
            ((w0 >> 24) & 0xFF) as u8,
            (w1 & 0xFF) as u8,
            ((w1 >> 8) & 0xFF) as u8,
        ])
    }

    /// Parses the returned clock rate from a processed `TAG_SET_CLOCK_RATE` or `TAG_GET_CLOCK_RATE` response.
    pub fn parse_clock_rate(buf: &[u32; 12]) -> Option<u32> {
        if buf[1] != 0x8000_0000 {
            return None;
        }
        Some(buf[6])
    }

    /// Sends a 32-bit value along with a Mailbox channel.
    ///
    /// # Safety
    /// Must only be executed on real hardware with mapped MMIO device memory.
    pub unsafe fn write_channel(&self, channel: MailboxChannel, data28: u32) {
        let status_ptr = (self.base_addr + reg::STATUS) as *const u32;
        let write_ptr = (self.base_addr + reg::WRITE) as *mut u32;

        while (core::ptr::read_volatile(status_ptr) & status::FULL) != 0 {}

        let val = (data28 & 0xFFFF_FFF0) | ((channel as u8) as u32 & 0x0F);
        core::ptr::write_volatile(write_ptr, val);
    }

    /// Reads a 32-bit value from a specific Mailbox channel.
    ///
    /// # Safety
    /// Must only be executed on real hardware with mapped MMIO device memory.
    pub unsafe fn read_channel(&self, channel: MailboxChannel) -> u32 {
        let status_ptr = (self.base_addr + reg::STATUS) as *const u32;
        let read_ptr = (self.base_addr + reg::READ) as *const u32;

        loop {
            while (core::ptr::read_volatile(status_ptr) & status::EMPTY) != 0 {}

            let val = core::ptr::read_volatile(read_ptr);
            let chan = (val & 0x0F) as u8;
            if chan == (channel as u8) {
                return val & 0xFFFF_FFF0;
            }
        }
    }
}
