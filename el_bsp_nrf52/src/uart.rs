//! Nordic nRF52 UARTE Driver with EasyDMA support
//!
//! Provides zero-allocation bidirectional conversion between legacy MIDI 1.0 serial bytes
//! and MIDI 2.0 UMPs (MT=0x1 and MT=0x2) for Nordic nRF52 microcontrollers (nRF52832, nRF52840, nRF52833).

use el_midi2::transport::serial::{ByteStreamToUmp, UmpToByteStream};
use el_midi2::transport::{EmbeddedTransport, Transport, TransportError};
use el_midi2::ump::Ump;
use heapless::spsc::Queue;

/// nRF52 UARTE peripheral instance identifier
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Nrf52UarteInstance {
    Uarte0,
    Uarte1,
}

/// nRF52 UARTE hardware baud rate register constants
///
/// On nRF52, the BAUDRATE register uses standard 32-bit hardware encodings based on:
/// `BAUDRATE = (actual_baud * 2^32) / 16_000_000`
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Nrf52BaudRate {
    /// 31,250 baud standard MIDI 1.0: `(31,250 * 2^32) / 16,000,000 = 0x00800000` (8,388,608)
    Baud31250,
    /// 115,200 baud high-speed serial: `0x01D7E000`
    Baud115200,
    /// 230,400 baud high-speed serial: `0x03AFB000`
    Baud230400,
    /// 1,000,000 baud high-speed serial: `0x10000000`
    Baud1M,
    /// Custom baud rate configuration
    Custom(u32),
}

impl Nrf52BaudRate {
    /// Returns the raw 32-bit value to write to the `BAUDRATE` peripheral register.
    pub const fn register_value(&self) -> u32 {
        match *self {
            Self::Baud31250 => 0x0080_0000,
            Self::Baud115200 => 0x01D7_E000,
            Self::Baud230400 => 0x03AF_B000,
            Self::Baud1M => 0x1000_0000,
            Self::Custom(val) => val,
        }
    }

    /// Calculates register value from desired baud rate using nRF52 16MHz clock base.
    pub fn from_baud(baud: u32) -> Self {
        assert!(baud > 0, "Baud rate must be non-zero");
        match baud {
            31_250 => Self::Baud31250,
            115_200 => Self::Baud115200,
            230_400 => Self::Baud230400,
            1_000_000 => Self::Baud1M,
            other => {
                // Formula: (baud * 2^32) / 16_000_000 = baud * (2^26 / 250_000)
                let val = (((other as u64) << 32) / 16_000_000) as u32;
                Self::Custom(val)
            }
        }
    }
}

/// Nordic nRF52 UARTE Peripheral Driver for MIDI 2.0 with EasyDMA buffer hooks.
pub struct Nrf52UartDriver<const RX_CAP: usize = 128, const TX_CAP: usize = 128> {
    pub instance: Nrf52UarteInstance,
    pub baud: Nrf52BaudRate,
    pub group: u8,
    rx_converter: ByteStreamToUmp,
    tx_converter: UmpToByteStream,
    rx_ump_queue: Queue<Ump, 16>,
    tx_byte_queue: Queue<u8, TX_CAP>,
}

impl<const RX_CAP: usize, const TX_CAP: usize> Nrf52UartDriver<RX_CAP, TX_CAP> {
    pub fn new(instance: Nrf52UarteInstance, baud: Nrf52BaudRate, group: u8) -> Self {
        Self {
            instance,
            baud,
            group: group & 0x0F,
            rx_converter: ByteStreamToUmp::new(group & 0x0F),
            tx_converter: UmpToByteStream::new(),
            rx_ump_queue: Queue::new(),
            tx_byte_queue: Queue::new(),
        }
    }

    /// Handles an EasyDMA RX buffer chunk received from the hardware peripheral.
    pub fn handle_easydma_rx(&mut self, buffer: &[u8]) {
        for &byte in buffer {
            self.rx_converter.process_byte(byte);
            while let Some(ump) = self.rx_converter.pop_ump() {
                let _ = self.rx_ump_queue.enqueue(ump);
            }
        }
    }

    /// Directly ingests an incoming serial byte (e.g. from an interrupt service routine).
    pub fn push_rx_byte(&mut self, byte: u8) {
        self.rx_converter.process_byte(byte);
        while let Some(ump) = self.rx_converter.pop_ump() {
            let _ = self.rx_ump_queue.enqueue(ump);
        }
    }

    /// Pops the next byte waiting to be transmitted over UARTE TX via EasyDMA or byte write.
    pub fn pop_tx_byte(&mut self) -> Option<u8> {
        self.tx_byte_queue.dequeue()
    }

    /// Fills an EasyDMA TX buffer slice with pending TX bytes. Returns number of bytes transferred.
    pub fn fill_easydma_tx(&mut self, dest: &mut [u8]) -> usize {
        let mut count = 0;
        for slot in dest.iter_mut() {
            if let Some(b) = self.tx_byte_queue.dequeue() {
                *slot = b;
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    /// Number of bytes waiting in the TX queue.
    pub fn tx_pending(&self) -> usize {
        self.tx_byte_queue.len()
    }
}

impl<const RX_CAP: usize, const TX_CAP: usize> Transport for Nrf52UartDriver<RX_CAP, TX_CAP> {
    fn send(&mut self, packets: &[Ump]) -> Result<(), TransportError> {
        for ump in packets {
            self.tx_converter.process_ump(ump);
            while let Some(b) = self.tx_converter.pop_byte() {
                self.tx_byte_queue
                    .enqueue(b)
                    .map_err(|_| TransportError::TxBufferFull)?;
            }
        }
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Ump>, TransportError> {
        if let Some(ump) = self.rx_converter.pop_ump() {
            return Ok(Some(ump));
        }
        Ok(self.rx_ump_queue.dequeue())
    }
}

impl<const RX_CAP: usize, const TX_CAP: usize> EmbeddedTransport
    for Nrf52UartDriver<RX_CAP, TX_CAP>
{
    type Error = TransportError;

    fn read_ump(&mut self) -> Result<Option<Ump>, Self::Error> {
        self.recv()
    }

    fn write_ump(&mut self, ump: &Ump) -> Result<(), Self::Error> {
        self.send(core::slice::from_ref(ump))
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
