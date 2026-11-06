//! ESP32 Hardware UART Peripheral Driver & MIDI 1.0 / MIDI 2.0 Bridge
//!
//! Provides a zero-allocation UART FIFO driver and bidirectional byte-stream to UMP converter
//! tailored for ESP32 hardware UART peripherals (UART0, UART1, UART2).
//! Compatible with both Xtensa (ESP32, ESP32-S2, ESP32-S3) and RISC-V (ESP32-C3, ESP32-C6, ESP32-H2) architectures.

use el_midi2::transport::{
    ByteStreamToUmp, EmbeddedTransport, Transport, TransportError, UmpToByteStream,
};
use el_midi2::ump::Ump;

/// Hardware UART port selection on ESP32 microcontrollers.
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Esp32UartPort {
    Uart0 = 0,
    Uart1 = 1,
    Uart2 = 2,
}

/// Baud rate configuration for MIDI communication.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MidiBaudRate {
    /// Standard MIDI 1.0 DIN / TRS 5-pin baud rate (31,250 baud)
    Midi1_31250,
    /// High-Speed Serial UMP rate (e.g. 115,200 baud)
    HighSpeed115200,
    /// Ultra-High-Speed Serial UMP rate (e.g. 1,000,000 baud)
    UltraSpeed1M,
    /// Custom baud rate in bits-per-second
    Custom(u32),
}

impl MidiBaudRate {
    #[must_use]
    #[inline]
    pub const fn to_baud(self) -> u32 {
        match self {
            Self::Midi1_31250 => 31_250,
            Self::HighSpeed115200 => 115_200,
            Self::UltraSpeed1M => 1_000_000,
            Self::Custom(baud) => baud,
        }
    }
}

/// ESP32 Hardware UART MIDI Driver.
///
/// Encapsulates UART hardware FIFO interaction, real-time byte interleaving,
/// and bidirectional UMP conversion without dynamic allocations.
pub struct Esp32UartDriver<const RX_CAP: usize = 128, const TX_CAP: usize = 128> {
    pub port: Esp32UartPort,
    pub baud_rate: MidiBaudRate,
    rx_converter: ByteStreamToUmp,
    tx_converter: UmpToByteStream,
    rx_fifo: [u8; RX_CAP],
    rx_fifo_len: usize,
    tx_fifo: [u8; TX_CAP],
    tx_fifo_len: usize,
    pub group: u8,
}

impl<const RX_CAP: usize, const TX_CAP: usize> Esp32UartDriver<RX_CAP, TX_CAP> {
    /// Creates a new ESP32 UART MIDI driver for the specified port.
    #[must_use]
    pub const fn new(port: Esp32UartPort, baud: MidiBaudRate, group: u8) -> Self {
        assert!(RX_CAP > 0, "RX FIFO capacity must be > 0");
        assert!(TX_CAP > 0, "TX FIFO capacity must be > 0");
        Self {
            port,
            baud_rate: baud,
            rx_converter: ByteStreamToUmp::new(group),
            tx_converter: UmpToByteStream::new(),
            rx_fifo: [0; RX_CAP],
            rx_fifo_len: 0,
            tx_fifo: [0; TX_CAP],
            tx_fifo_len: 0,
            group,
        }
    }

    /// Simulates hardware FIFO ingestion from UART RX interrupt/DMA.
    pub fn push_rx_byte(&mut self, byte: u8) -> Result<(), TransportError> {
        if self.rx_fifo_len >= RX_CAP {
            return Err(TransportError::RxBufferOverflow);
        }
        self.rx_fifo[self.rx_fifo_len] = byte;
        self.rx_fifo_len += 1;
        Ok(())
    }

    /// Pulls a byte from the TX FIFO to be written to the UART TX hardware register.
    pub fn pop_tx_byte(&mut self) -> Option<u8> {
        if self.tx_fifo_len == 0 {
            return None;
        }
        let b = self.tx_fifo[0];
        // Shift remaining bytes
        for i in 1..self.tx_fifo_len {
            self.tx_fifo[i - 1] = self.tx_fifo[i];
        }
        self.tx_fifo_len -= 1;
        Some(b)
    }

    /// Returns the number of bytes queued for transmission.
    #[must_use]
    #[inline]
    pub fn tx_pending(&self) -> usize {
        self.tx_fifo_len
    }

    /// Clears internal RX and TX FIFOs.
    pub fn reset_fifos(&mut self) {
        self.rx_fifo_len = 0;
        self.tx_fifo_len = 0;
        self.rx_converter.reset();
        self.tx_converter.reset();
    }
}

impl<const RX_CAP: usize, const TX_CAP: usize> Transport for Esp32UartDriver<RX_CAP, TX_CAP> {
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
            // Shift remaining
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
    for Esp32UartDriver<RX_CAP, TX_CAP>
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
