//! BCM2837B0 GPIO Controller Driver
//!
//! Provides pin multiplexing and pull-up/pull-down resistor configuration.
//! Used for:
//! - Configuring GPIO 14 (TXD0) and GPIO 15 (RXD0) to `ALT0` for PL011 UART0.
//! - Disabling internal pull-up/down resistors for external optocoupler MIDI lines.
//! - Driving MIDI status and activity LEDs.

use crate::mmio::GPIO_BASE;

/// BCM2837 GPIO Register Offsets
pub mod reg {
    /// Function Select 0 (GPIO 0-9)
    pub const GPFSEL0: usize = 0x00;
    /// Function Select 1 (GPIO 10-19)
    pub const GPFSEL1: usize = 0x04;
    /// Function Select 2 (GPIO 20-29)
    pub const GPFSEL2: usize = 0x08;
    /// Function Select 3 (GPIO 30-39)
    pub const GPFSEL3: usize = 0x0C;
    /// Function Select 4 (GPIO 40-49)
    pub const GPFSEL4: usize = 0x10;
    /// Function Select 5 (GPIO 50-53)
    pub const GPFSEL5: usize = 0x14;
    /// Pin Output Set 0 (GPIO 0-31)
    pub const GPSET0: usize = 0x1C;
    /// Pin Output Clear 0 (GPIO 0-31)
    pub const GPCLR0: usize = 0x28;
    /// Pin Level 0 (GPIO 0-31)
    pub const GPLEV0: usize = 0x34;
    /// Pull-up/pull-down register
    pub const GPPUD: usize = 0x94;
    /// Pull-up/pull-down Clock 0 (GPIO 0-31)
    pub const GPPUDCLK0: usize = 0x98;
}

/// GPIO Pin Function
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum GpioFunction {
    Input = 0b000,
    Output = 0b001,
    Alt0 = 0b100,
    Alt1 = 0b101,
    Alt2 = 0b110,
    Alt3 = 0b111,
    Alt4 = 0b011,
    Alt5 = 0b010,
}

/// GPIO Pull-Up / Pull-Down state
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum GpioPull {
    Off = 0b00,
    Down = 0b01,
    Up = 0b10,
}

/// GPIO Pin Number wrapper (0..53)
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct GpioPin(pub u8);

impl GpioPin {
    pub const UART0_TX: Self = Self(14);
    pub const UART0_RX: Self = Self(15);
}

/// BCM2837 GPIO Controller Driver
#[derive(Debug, Copy, Clone)]
pub struct Rpi3Gpio {
    base_addr: usize,
}

impl Default for Rpi3Gpio {
    fn default() -> Self {
        Self::new()
    }
}

impl Rpi3Gpio {
    /// Creates a driver instance pointing to standard MMIO base.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            base_addr: GPIO_BASE,
        }
    }

    /// Creates a driver instance with a custom base address.
    #[must_use]
    pub const fn with_base(base_addr: usize) -> Self {
        Self { base_addr }
    }

    /// Calculates GPFSEL register offset and bit shift for a given pin.
    #[must_use]
    pub const fn function_select_offset_and_shift(pin: u8) -> (usize, u32) {
        let reg_index = (pin / 10) as usize;
        let shift = ((pin % 10) * 3) as u32;
        (reg_index * 4, shift)
    }

    /// Sets the alternate function or direction of a GPIO pin.
    ///
    /// # Safety
    /// Must only be executed on real hardware with mapped MMIO device memory.
    pub unsafe fn set_function(&self, pin: GpioPin, func: GpioFunction) {
        let (offset, shift) = Self::function_select_offset_and_shift(pin.0);
        let ptr = (self.base_addr + offset) as *mut u32;

        let mut val = core::ptr::read_volatile(ptr);
        val &= !(0b111 << shift);
        val |= (func as u32) << shift;
        core::ptr::write_volatile(ptr, val);
    }

    /// Configures internal pull-up/down resistors for a pin.
    ///
    /// # Safety
    /// Must only be executed on real hardware with mapped MMIO device memory.
    pub unsafe fn set_pull(&self, pin: GpioPin, pull: GpioPull) {
        let pud_ptr = (self.base_addr + reg::GPPUD) as *mut u32;
        let pudclk_ptr = (self.base_addr + reg::GPPUDCLK0) as *mut u32;

        core::ptr::write_volatile(pud_ptr, pull as u32);
        // Delay 150 cycles as specified in BCM2837 manual
        for _ in 0..150 {
            core::hint::spin_loop();
        }

        let clk_bit = 1 << (pin.0 & 31);
        core::ptr::write_volatile(pudclk_ptr, clk_bit);
        for _ in 0..150 {
            core::hint::spin_loop();
        }

        core::ptr::write_volatile(pud_ptr, 0);
        core::ptr::write_volatile(pudclk_ptr, 0);
    }

    /// Sets up GPIO 14 (TXD0) and GPIO 15 (RXD0) for PL011 UART0:
    /// Sets `ALT0` and disables pull-up/down resistors.
    ///
    /// # Safety
    /// Must only be executed on real hardware with mapped MMIO device memory.
    pub unsafe fn setup_uart0_pins(&self) {
        self.set_function(GpioPin::UART0_TX, GpioFunction::Alt0);
        self.set_function(GpioPin::UART0_RX, GpioFunction::Alt0);
        self.set_pull(GpioPin::UART0_TX, GpioPull::Off);
        self.set_pull(GpioPin::UART0_RX, GpioPull::Off);
    }
}
