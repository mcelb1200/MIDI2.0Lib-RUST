//! RP2040 Programmable I/O (PIO) MIDI 2.0 Driver
//!
//! Provides hardware-timed state machine UART emulation and high-speed serial UMP framing
//! using Raspberry Pi RP2040 Programmable I/O (PIO0 / PIO1) state machines.
//! Supports sample-accurate jitter-free transmission and reception for standard MIDI 1.0 (31,250 baud)
//! and high-speed serial UMP.

use el_midi2::transport::{
    ByteStreamToUmp, EmbeddedTransport, Transport, TransportError, UmpToByteStream,
};
use el_midi2::ump::Ump;

/// RP2040 PIO instance selector.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PioBlock {
    Pio0,
    Pio1,
}

/// RP2040 State Machine index within a PIO block (SM0..SM3).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PioStateMachine {
    Sm0 = 0,
    Sm1 = 1,
    Sm2 = 2,
    Sm3 = 3,
}

/// RP2040 PIO Clock Divider configuration.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PioClockDiv {
    pub int_div: u16,
    pub frac_div: u8,
}

impl PioClockDiv {
    /// Computes PIO clock divider for standard 125 MHz system clock.
    ///
    /// For 8 cycles per bit UART program:
    /// - 31,250 baud: 125,000,000 / (31,250 * 8) = 500 (exact)
    /// - 115,200 baud: 125,000,000 / (115,200 * 8) = 135.63 -> 135 + 162/256
    #[must_use]
    pub const fn from_baud(baud: u32) -> Self {
        if baud == 31_250 {
            Self {
                int_div: 500,
                frac_div: 0,
            }
        } else if baud == 115_200 {
            Self {
                int_div: 135,
                frac_div: 162,
            }
        } else if baud == 1_000_000 {
            Self {
                int_div: 15,
                frac_div: 160,
            }
        } else {
            // Generic fallback
            let sys_clk = 125_000_000_u64;
            let target_freq = (baud as u64) * 8;
            if target_freq == 0 {
                return Self {
                    int_div: 500,
                    frac_div: 0,
                };
            }
            let div_scaled = (sys_clk << 8) / target_freq;
            let int_part = (div_scaled >> 8) as u16;
            let frac_part = (div_scaled & 0xFF) as u8;
            Self {
                int_div: int_part,
                frac_div: frac_part,
            }
        }
    }
}

/// RP2040 PIO MIDI Driver.
///
/// Manages PIO RX/TX FIFOs, word framing, running status, and bidirectional UMP conversion.
pub struct Rp2040PioDriver<const RX_CAP: usize = 128, const TX_CAP: usize = 128> {
    pub pio: PioBlock,
    pub sm: PioStateMachine,
    pub clock_div: PioClockDiv,
    rx_converter: ByteStreamToUmp,
    tx_converter: UmpToByteStream,
    rx_fifo: [u8; RX_CAP],
    rx_fifo_len: usize,
    tx_fifo: [u8; TX_CAP],
    tx_fifo_len: usize,
    pub group: u8,
}

impl<const RX_CAP: usize, const TX_CAP: usize> Rp2040PioDriver<RX_CAP, TX_CAP> {
    /// Creates a new RP2040 PIO MIDI driver.
    #[must_use]
    pub const fn new(pio: PioBlock, sm: PioStateMachine, baud: u32, group: u8) -> Self {
        assert!(RX_CAP > 0, "RX FIFO capacity must be > 0");
        assert!(TX_CAP > 0, "TX FIFO capacity must be > 0");
        Self {
            pio,
            sm,
            clock_div: PioClockDiv::from_baud(baud),
            rx_converter: ByteStreamToUmp::new(group),
            tx_converter: UmpToByteStream::new(),
            rx_fifo: [0; RX_CAP],
            rx_fifo_len: 0,
            tx_fifo: [0; TX_CAP],
            tx_fifo_len: 0,
            group,
        }
    }

    /// Ingests a raw byte pushed by the PIO RX FIFO interrupt / DMA service routine.
    pub fn push_rx_byte(&mut self, byte: u8) -> Result<(), TransportError> {
        if self.rx_fifo_len >= RX_CAP {
            return Err(TransportError::RxBufferOverflow);
        }
        self.rx_fifo[self.rx_fifo_len] = byte;
        self.rx_fifo_len += 1;
        Ok(())
    }

    /// Pulls a byte ready to be pushed to the PIO TX FIFO register.
    pub fn pop_tx_byte(&mut self) -> Option<u8> {
        if self.tx_fifo_len == 0 {
            return None;
        }
        let b = self.tx_fifo[0];
        for i in 1..self.tx_fifo_len {
            self.tx_fifo[i - 1] = self.tx_fifo[i];
        }
        self.tx_fifo_len -= 1;
        Some(b)
    }

    /// Returns the number of bytes queued in the TX FIFO.
    #[must_use]
    #[inline]
    pub fn tx_pending(&self) -> usize {
        self.tx_fifo_len
    }

    /// Resets all internal FIFOs and protocol converter states.
    pub fn reset_fifos(&mut self) {
        self.rx_fifo_len = 0;
        self.tx_fifo_len = 0;
        self.rx_converter.reset();
        self.tx_converter.reset();
    }
}

impl<const RX_CAP: usize, const TX_CAP: usize> Transport for Rp2040PioDriver<RX_CAP, TX_CAP> {
    fn send(&mut self, umps: &[Ump]) -> Result<(), TransportError> {
        for ump in umps {
            self.tx_converter.process_ump(ump);
            while let Some(byte) = self.tx_converter.pop_byte() {
                if self.tx_fifo_len >= TX_CAP {
                    return Err(TransportError::TxBufferFull);
                }
                self.tx_fifo[self.tx_fifo_len] = byte;
                self.tx_fifo_len += 1;
            }
        }
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Ump>, TransportError> {
        if let Some(ump) = self.rx_converter.pop_ump() {
            return Ok(Some(ump));
        }

        while self.rx_fifo_len > 0 {
            let b = self.rx_fifo[0];
            for i in 1..self.rx_fifo_len {
                self.rx_fifo[i - 1] = self.rx_fifo[i];
            }
            self.rx_fifo_len -= 1;

            self.rx_converter.process_byte(b);
            if let Some(ump) = self.rx_converter.pop_ump() {
                return Ok(Some(ump));
            }
        }
        Ok(None)
    }
}

impl<const RX_CAP: usize, const TX_CAP: usize> EmbeddedTransport
    for Rp2040PioDriver<RX_CAP, TX_CAP>
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
        Transport::flush(self)
    }
}
