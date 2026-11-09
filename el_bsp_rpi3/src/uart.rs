//! ARM PrimeCell PL011 UART0 Driver
//!
//! Provides a jitter-free, zero-allocation UART transceiver for the BCM2837B0 PL011 UART0 core.
//! Used for:
//! - Standard MIDI 1.0 (31,250 baud) DIN/TRS communication with running status and real-time interleaving.
//! - High-speed serial Universal MIDI Packet (UMP) communication (1 MBaud, 3 MBaud, or custom rates).
//! - Direct hardware FIFO interaction and ringbuffer abstraction.

use crate::mmio::UART0_BASE;
use el_midi2::transport::queue::PacketQueue;
use el_midi2::transport::serial::{ByteStreamToUmp, UmpToByteStream};
use el_midi2::transport::{EmbeddedTransport, Transport, TransportError};
use el_midi2::ump::Ump;

/// Default UART reference clock locked via VideoCore Mailbox (48 MHz)
pub const UART_REF_CLOCK_HZ: u32 = 48_000_000;

/// PL011 Register Offsets
pub mod reg {
    /// Data Register (offset 0x00)
    pub const DR: usize = 0x00;
    /// Receive Status / Error Clear (offset 0x04)
    pub const RSRECR: usize = 0x04;
    /// Flag Register (offset 0x18)
    pub const FR: usize = 0x18;
    /// Integer Baud Rate Divisor (offset 0x24)
    pub const IBRD: usize = 0x24;
    /// Fractional Baud Rate Divisor (offset 0x28)
    pub const FBRD: usize = 0x28;
    /// Line Control Register (offset 0x2C)
    pub const LCRH: usize = 0x2C;
    /// Control Register (offset 0x30)
    pub const CR: usize = 0x30;
    /// Interrupt FIFO Level Select (offset 0x34)
    pub const IFLS: usize = 0x34;
    /// Interrupt Mask Set/Clear (offset 0x38)
    pub const IMSC: usize = 0x38;
    /// Raw Interrupt Status (offset 0x3C)
    pub const RIS: usize = 0x3C;
    /// Masked Interrupt Status (offset 0x40)
    pub const MIS: usize = 0x40;
    /// Interrupt Clear Register (offset 0x44)
    pub const ICR: usize = 0x44;
}

/// PL011 Flag Register bit flags
pub mod fr_flags {
    /// Clear to send (bit 0)
    pub const CTS: u32 = 1 << 0;
    /// Busy transmitting (bit 3)
    pub const BUSY: u32 = 1 << 3;
    /// Receive FIFO empty (bit 4)
    pub const RXFE: u32 = 1 << 4;
    /// Transmit FIFO full (bit 5)
    pub const TXFF: u32 = 1 << 5;
    /// Receive FIFO full (bit 6)
    pub const RXFF: u32 = 1 << 6;
    /// Transmit FIFO empty (bit 7)
    pub const TXFE: u32 = 1 << 7;
}

/// PL011 Line Control Register bit flags
pub mod lcrh_flags {
    /// Send break (bit 0)
    pub const BRK: u32 = 1 << 0;
    /// Parity enable (bit 1)
    pub const PEN: u32 = 1 << 1;
    /// Even parity select (bit 2)
    pub const EPS: u32 = 1 << 2;
    /// Two stop bits select (bit 3)
    pub const STP2: u32 = 1 << 3;
    /// Enable FIFOs (bit 4)
    pub const FEN: u32 = 1 << 4;
    /// Word length 8 bits (bits 5..6 = 0b11)
    pub const WLEN8: u32 = 0b11 << 5;
}

/// PL011 Control Register bit flags
pub mod cr_flags {
    /// UART enable (bit 0)
    pub const UARTEN: u32 = 1 << 0;
    /// Transmit enable (bit 8)
    pub const TXE: u32 = 1 << 8;
    /// Receive enable (bit 9)
    pub const RXE: u32 = 1 << 9;
}

/// Baud rate options supported by Rpi3UartDriver
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Rpi3BaudRate {
    /// Standard MIDI 1.0 (31,250 baud)
    Midi1_31250,
    /// High-Speed Serial UMP (1,000,000 baud / 1 MBaud)
    Ump1M,
    /// High-Speed Serial UMP (3,000,000 baud / 3 MBaud)
    Ump3M,
    /// Standard Diagnostic / Host Link (115,200 baud)
    Standard115200,
    /// Custom baud rate in Hz
    Custom(u32),
}

impl Rpi3BaudRate {
    #[must_use]
    #[inline]
    pub const fn to_baud(self) -> u32 {
        match self {
            Self::Midi1_31250 => 31_250,
            Self::Ump1M => 1_000_000,
            Self::Ump3M => 3_000_000,
            Self::Standard115200 => 115_200,
            Self::Custom(b) => b,
        }
    }
}

/// Computed integer and fractional baud rate divisor for PL011
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Rpi3BaudDivisor {
    pub ibrd: u16,
    pub fbrd: u8,
}

impl Rpi3BaudDivisor {
    /// Calculates PL011 baud divisor registers from UART clock and target baud rate.
    ///
    /// Formulas from ARM PrimeCell PL011 TRM:
    /// `Baud Divisor = UART_CLK / (16 * Target Baud)`
    /// `IBRD = floor(Baud Divisor)`
    /// `FBRD = floor((Baud Divisor - IBRD) * 64 + 0.5)`
    #[must_use]
    pub const fn calculate(uart_clk_hz: u32, baud: u32) -> Self {
        assert!(baud > 0, "Baud rate must be non-zero");
        assert!(uart_clk_hz > 0, "UART reference clock must be non-zero");

        let div16 = 16 * (baud as u64);
        let uart_clk = uart_clk_hz as u64;

        let ibrd = (uart_clk / div16) as u16;
        let remainder = uart_clk % div16;

        // Multiply remainder by 64 and round to nearest: (remainder * 64 + div16 / 2) / div16
        let frac = (((remainder * 64) + (div16 / 2)) / div16) as u8;

        if frac >= 64 {
            Self {
                ibrd: ibrd + 1,
                fbrd: 0,
            }
        } else {
            Self { ibrd, fbrd: frac }
        }
    }
}

/// PL011 UART0 Driver for Bare Metal Raspberry Pi 3B+
///
/// Implements `Transport` and `EmbeddedTransport` with zero dynamic allocations.
pub struct Rpi3UartDriver<const RX_CAP: usize = 128, const TX_CAP: usize = 128> {
    base_addr: usize,
    default_group: u8,
    baud_rate: Rpi3BaudRate,
    byte_to_ump: ByteStreamToUmp,
    ump_to_byte: UmpToByteStream,
    rx_queue: PacketQueue<RX_CAP>,
    tx_queue: PacketQueue<TX_CAP>,
    tx_byte_fifo: [u8; TX_CAP],
    tx_byte_head: usize,
    tx_byte_tail: usize,
    tx_byte_count: usize,
}

impl<const RX_CAP: usize, const TX_CAP: usize> Rpi3UartDriver<RX_CAP, TX_CAP> {
    /// Creates a new `Rpi3UartDriver` instance.
    #[must_use]
    pub const fn new(baud_rate: Rpi3BaudRate, default_group: u8) -> Self {
        Self::with_base(UART0_BASE, baud_rate, default_group)
    }

    /// Creates an instance with a custom MMIO base address.
    #[must_use]
    pub const fn with_base(base_addr: usize, baud_rate: Rpi3BaudRate, default_group: u8) -> Self {
        assert!(RX_CAP > 1, "RX_CAP must be > 1");
        assert!(TX_CAP > 1, "TX_CAP must be > 1");

        let grp = default_group & 0x0F;
        Self {
            base_addr,
            default_group: grp,
            baud_rate,
            byte_to_ump: ByteStreamToUmp::new(grp),
            ump_to_byte: UmpToByteStream::new(),
            rx_queue: PacketQueue::new(),
            tx_queue: PacketQueue::new(),
            tx_byte_fifo: [0; TX_CAP],
            tx_byte_head: 0,
            tx_byte_tail: 0,
            tx_byte_count: 0,
        }
    }

    /// Returns the active baud rate.
    #[must_use]
    #[inline]
    pub const fn baud_rate(&self) -> Rpi3BaudRate {
        self.baud_rate
    }

    /// Returns the default UMP group index (0..15).
    #[must_use]
    #[inline]
    pub const fn default_group(&self) -> u8 {
        self.default_group
    }

    /// Sets the default UMP group index (0..15).
    pub fn set_default_group(&mut self, group: u8) {
        self.default_group = group & 0x0F;
        self.byte_to_ump.default_group = self.default_group;
    }

    /// Returns the computed PL011 baud divisor for the configured baud rate under 48 MHz clock.
    #[must_use]
    #[inline]
    pub const fn baud_divisor(&self) -> Rpi3BaudDivisor {
        Rpi3BaudDivisor::calculate(UART_REF_CLOCK_HZ, self.baud_rate.to_baud())
    }

    /// Initializes the physical PL011 hardware registers for 8-N-1 with FIFOs enabled.
    ///
    /// # Safety
    /// Must only be executed on real hardware with mapped MMIO device memory.
    pub unsafe fn init_hardware(&mut self) {
        let cr_ptr = (self.base_addr + reg::CR) as *mut u32;
        let icr_ptr = (self.base_addr + reg::ICR) as *mut u32;
        let ibrd_ptr = (self.base_addr + reg::IBRD) as *mut u32;
        let fbrd_ptr = (self.base_addr + reg::FBRD) as *mut u32;
        let lcrh_ptr = (self.base_addr + reg::LCRH) as *mut u32;
        let imsc_ptr = (self.base_addr + reg::IMSC) as *mut u32;

        // 1. Disable UART
        core::ptr::write_volatile(cr_ptr, 0);

        // 2. Clear all pending interrupts
        core::ptr::write_volatile(icr_ptr, 0x07FF);

        // 3. Set baud divisor
        let div = self.baud_divisor();
        core::ptr::write_volatile(ibrd_ptr, div.ibrd as u32);
        core::ptr::write_volatile(fbrd_ptr, div.fbrd as u32);

        // 4. Set 8-bit word length, enable FIFOs, 1 stop bit, no parity (8-N-1)
        core::ptr::write_volatile(lcrh_ptr, lcrh_flags::WLEN8 | lcrh_flags::FEN);

        // 5. Mask all interrupts initially (polled mode)
        core::ptr::write_volatile(imsc_ptr, 0);

        // 6. Enable UART, TX, and RX
        core::ptr::write_volatile(cr_ptr, cr_flags::UARTEN | cr_flags::TXE | cr_flags::RXE);
    }

    /// Direct hardware read from PL011 Data Register (`DR`).
    ///
    /// # Safety
    /// Volatile MMIO read.
    #[inline]
    pub unsafe fn read_dr(&self) -> u8 {
        let dr_ptr = (self.base_addr + reg::DR) as *const u32;
        (core::ptr::read_volatile(dr_ptr) & 0xFF) as u8
    }

    /// Direct hardware write to PL011 Data Register (`DR`).
    ///
    /// # Safety
    /// Volatile MMIO write.
    #[inline]
    pub unsafe fn write_dr(&self, byte: u8) {
        let dr_ptr = (self.base_addr + reg::DR) as *mut u32;
        core::ptr::write_volatile(dr_ptr, byte as u32);
    }

    /// Checks if hardware transmit FIFO is full (`TXFF` in `FR`).
    ///
    /// # Safety
    /// Volatile MMIO read.
    #[inline]
    pub unsafe fn is_tx_fifo_full(&self) -> bool {
        let fr_ptr = (self.base_addr + reg::FR) as *const u32;
        (core::ptr::read_volatile(fr_ptr) & fr_flags::TXFF) != 0
    }

    /// Checks if hardware receive FIFO is empty (`RXFE` in `FR`).
    ///
    /// # Safety
    /// Volatile MMIO read.
    #[inline]
    pub unsafe fn is_rx_fifo_empty(&self) -> bool {
        let fr_ptr = (self.base_addr + reg::FR) as *const u32;
        (core::ptr::read_volatile(fr_ptr) & fr_flags::RXFE) != 0
    }

    /// Polls physical PL011 hardware RX FIFO and processes incoming bytes into UMPs.
    ///
    /// # Safety
    /// Volatile MMIO operations.
    pub unsafe fn poll_hardware_rx(&mut self) -> usize {
        let mut count = 0;
        while !self.is_rx_fifo_empty() {
            let byte = self.read_dr();
            self.push_rx_byte(byte);
            count += 1;
        }
        count
    }

    /// Flushes queued TX bytes into physical PL011 hardware TX FIFO.
    ///
    /// # Safety
    /// Volatile MMIO operations.
    pub unsafe fn flush_hardware_tx(&mut self) -> usize {
        let mut count = 0;
        while !self.is_tx_fifo_full() && self.tx_byte_count > 0 {
            if let Some(b) = self.pop_tx_byte() {
                self.write_dr(b);
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    /// Ingests a raw byte (e.g. from an RX interrupt service routine or loopback simulation).
    /// Converts legacy MIDI 1.0 serial bytes to UMPs, tracking running status and interleaving real-time bytes.
    pub fn push_rx_byte(&mut self, byte: u8) {
        self.byte_to_ump.process_byte(byte);
        while let Some(ump) = self.byte_to_ump.pop_ump() {
            let _ = self.rx_queue.push(ump);
        }
    }

    /// Pops the next byte waiting to be transmitted over the serial wire.
    pub fn pop_tx_byte(&mut self) -> Option<u8> {
        if self.tx_byte_count == 0 {
            return None;
        }
        let byte = self.tx_byte_fifo[self.tx_byte_tail];
        self.tx_byte_tail = (self.tx_byte_tail + 1) % TX_CAP;
        self.tx_byte_count -= 1;
        Some(byte)
    }

    /// Number of serial bytes pending in the software TX FIFO.
    #[must_use]
    #[inline]
    pub const fn tx_pending(&self) -> usize {
        self.tx_byte_count
    }

    /// Access pending high-speed UMP TX queue.
    pub fn tx_queue(&self) -> &PacketQueue<TX_CAP> {
        &self.tx_queue
    }

    /// Access mutable pending high-speed UMP TX queue.
    pub fn tx_queue_mut(&mut self) -> &mut PacketQueue<TX_CAP> {
        &mut self.tx_queue
    }

    /// Access pending UMP RX queue.
    pub fn rx_queue(&self) -> &PacketQueue<RX_CAP> {
        &self.rx_queue
    }

    /// Access mutable pending UMP RX queue.
    pub fn rx_queue_mut(&mut self) -> &mut PacketQueue<RX_CAP> {
        &mut self.rx_queue
    }
}

impl<const RX_CAP: usize, const TX_CAP: usize> Transport for Rpi3UartDriver<RX_CAP, TX_CAP> {
    fn send(&mut self, packets: &[Ump]) -> Result<(), TransportError> {
        for ump in packets {
            // Queue into UMP TX queue if using high-speed serial UMP
            let _ = self.tx_queue.push(*ump);

            self.ump_to_byte.process_ump(ump);
            while let Some(byte) = self.ump_to_byte.pop_byte() {
                if self.tx_byte_count >= TX_CAP {
                    return Err(TransportError::TxBufferFull);
                }
                self.tx_byte_fifo[self.tx_byte_head] = byte;
                self.tx_byte_head = (self.tx_byte_head + 1) % TX_CAP;
                self.tx_byte_count += 1;
            }
        }
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Ump>, TransportError> {
        if let Some(ump) = self.byte_to_ump.pop_ump() {
            return Ok(Some(ump));
        }
        Ok(self.rx_queue.pop())
    }
}

impl<const RX_CAP: usize, const TX_CAP: usize> EmbeddedTransport
    for Rpi3UartDriver<RX_CAP, TX_CAP>
{
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
        Ok(())
    }
}
