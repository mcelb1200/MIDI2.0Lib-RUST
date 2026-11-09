//! BCM2837B0 System Timer Driver
//!
//! The System Timer provides a continuous 1 MHz hardware counter (1 microsecond resolution),
//! driven by the crystal oscillator.
//!
//! In MIDI 2.0, this timer is utilized for:
//! - AppleMIDI / RTP-MIDI 64-bit 100-microsecond clock synchronization timestamps (`CK0`, `CK1`, `CK2`).
//! - UMP Jitter Reduction (JR) 16-bit clock timestamps (`JR Clock` and `JR Timestamp`).

use crate::mmio::SYSTEM_TIMER_BASE;

/// System Timer frequency in Hz (1 MHz = 1 µs per tick)
pub const SYSTEM_TIMER_FREQ_HZ: u64 = 1_000_000;

/// BCM2837 System Timer Register Offsets
pub mod reg {
    /// Control / Status register (offset 0x00)
    pub const CS: usize = 0x00;
    /// Counter Lower 32 bits (offset 0x04)
    pub const CLO: usize = 0x04;
    /// Counter Higher 32 bits (offset 0x08)
    pub const CHI: usize = 0x08;
    /// Compare 0 (offset 0x0C)
    pub const C0: usize = 0x0C;
    /// Compare 1 (offset 0x10)
    pub const C1: usize = 0x10;
    /// Compare 2 (offset 0x14)
    pub const C2: usize = 0x14;
    /// Compare 3 (offset 0x18)
    pub const C3: usize = 0x18;
}

/// Raspberry Pi 3B+ BCM2837 System Timer Driver
#[derive(Debug, Copy, Clone)]
pub struct Rpi3SystemTimer {
    base_addr: usize,
}

impl Default for Rpi3SystemTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl Rpi3SystemTimer {
    /// Creates a driver instance pointing to standard MMIO base.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            base_addr: SYSTEM_TIMER_BASE,
        }
    }

    /// Creates a driver instance with a custom base address (useful for simulation/testing).
    #[must_use]
    pub const fn with_base(base_addr: usize) -> Self {
        Self { base_addr }
    }

    /// Returns the base address of the System Timer MMIO registers.
    #[must_use]
    #[inline]
    pub const fn base_addr(&self) -> usize {
        self.base_addr
    }

    /// Reads the lower 32 bits of the 1 MHz counter directly from MMIO.
    ///
    /// # Safety
    /// Must only be executed on real hardware with mapped MMIO device memory.
    #[inline]
    pub unsafe fn read_clo(&self) -> u32 {
        let ptr = (self.base_addr + reg::CLO) as *const u32;
        core::ptr::read_volatile(ptr)
    }

    /// Reads the upper 32 bits of the 1 MHz counter directly from MMIO.
    ///
    /// # Safety
    /// Must only be executed on real hardware with mapped MMIO device memory.
    #[inline]
    pub unsafe fn read_chi(&self) -> u32 {
        let ptr = (self.base_addr + reg::CHI) as *const u32;
        core::ptr::read_volatile(ptr)
    }

    /// Reads the full 64-bit microsecond counter safely avoiding rollover tearing.
    ///
    /// # Safety
    /// Must only be executed on real hardware with mapped MMIO device memory.
    pub unsafe fn read_counter_ticks(&self) -> u64 {
        loop {
            let chi1 = self.read_chi();
            let clo = self.read_clo();
            let chi2 = self.read_chi();
            if chi1 == chi2 {
                return ((chi1 as u64) << 32) | (clo as u64);
            }
        }
    }

    /// Converts 1 MHz System Timer ticks (microseconds) to 100-microsecond units
    /// used by AppleMIDI / RTP-MIDI clock synchronization packets (`CK0`, `CK1`, `CK2`).
    #[must_use]
    #[inline]
    pub const fn ticks_to_apple_midi_timestamp(ticks: u64) -> u64 {
        ticks / 100
    }

    /// Converts 100-microsecond AppleMIDI timestamp units back to 1 MHz System Timer ticks.
    #[must_use]
    #[inline]
    pub const fn apple_midi_timestamp_to_ticks(timestamp: u64) -> u64 {
        timestamp * 100
    }

    /// Converts 1 MHz System Timer ticks to 16-bit Jitter Reduction (JR) timestamps.
    ///
    /// UMP JR Timestamp uses 16-bit clock ticks with 31.25 µs resolution (1/32000 sec).
    /// For 1 MHz input counter:
    /// `jr_timestamp = (ticks_us * 32_000) / 1_000_000 = (ticks_us * 32) / 1_000 = (ticks_us * 4) / 125`
    #[must_use]
    #[inline]
    pub const fn ticks_to_jr_timestamp(ticks_us: u64) -> u16 {
        ((ticks_us * 4 / 125) & 0xFFFF) as u16
    }
}
