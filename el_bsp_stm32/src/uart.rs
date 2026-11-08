//! STM32 UART Driver with DMA-friendly circular buffer / FIFO support
//!
//! Provides zero-allocation bidirectional conversion between legacy MIDI 1.0 serial bytes
//! and MIDI 2.0 UMPs (MT=0x1 and MT=0x2) for STM32 microcontrollers (STM32F4, G4, H7, F7).

use el_midi2::transport::serial::{ByteStreamToUmp, UmpToByteStream};
use el_midi2::transport::{EmbeddedTransport, Transport, TransportError};
use el_midi2::ump::Ump;
use heapless::spsc::Queue;

/// STM32 USART peripheral identifier
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Stm32UartPeripheral {
    Usart1,
    Usart2,
    Usart3,
    Uart4,
    Uart5,
    Usart6,
    Uart7,
    Uart8,
}

/// STM32 USART Baud Rate calculation result for USART_BRR register
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Stm32UartBaudConfig {
    /// Integer mantissa part for USART_BRR
    pub mantissa: u16,
    /// Fraction part for USART_BRR
    pub fraction: u8,
    /// Raw 16-bit register value for USART_BRR
    pub brr_val: u16,
}

impl Stm32UartBaudConfig {
    /// Computes BRR register value from peripheral bus clock (f_ck in Hz) and baud rate.
    /// Over8 mode: standard 16x oversampling (OVER8 = 0).
    ///
    /// Baud formula: `baud = f_ck / (16 * usartdiv)`
    /// `usartdiv = f_ck / (16 * baud)`
    /// `mantissa = (f_ck / (16 * baud))`
    /// `fraction = round(((f_ck % (16 * baud)) * 16) / (16 * baud))`
    pub fn from_clock_and_baud(f_ck: u32, baud: u32) -> Self {
        assert!(baud > 0, "Baud rate must be non-zero");
        assert!(f_ck > 0, "Clock frequency must be non-zero");

        let div16 = 16 * baud;
        let mantissa = (f_ck / div16) as u16;
        let rem = f_ck % div16;
        let fraction = (((rem * 16) + (div16 / 2)) / div16) as u8;

        let (final_mantissa, final_fraction) = if fraction >= 16 {
            (mantissa + 1, fraction - 16)
        } else {
            (mantissa, fraction)
        };

        let brr_val = (final_mantissa << 4) | ((final_fraction & 0x0F) as u16);

        Self {
            mantissa: final_mantissa,
            fraction: final_fraction,
            brr_val,
        }
    }
}

/// Circular DMA buffer tracking helper for STM32 DMA streams/channels.
/// Tracks DMA circular transfer head and software tail.
#[derive(Debug)]
pub struct Stm32DmaCircularBuffer<const CAP: usize> {
    buffer: [u8; CAP],
    tail: usize,
}

impl<const CAP: usize> Default for Stm32DmaCircularBuffer<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> Stm32DmaCircularBuffer<CAP> {
    pub const fn new() -> Self {
        assert!(CAP > 0, "DMA buffer capacity must be greater than zero");
        Self {
            buffer: [0_u8; CAP],
            tail: 0,
        }
    }

    /// Returns a mutable slice to the raw underlying buffer for DMA hardware peripheral transfer.
    pub fn raw_buffer_mut(&mut self) -> &mut [u8; CAP] {
        &mut self.buffer
    }

    /// Reads newly arrived bytes up to current DMA head position (e.g. `CAP - NDTR` on STM32).
    /// Calls `consumer` for each available byte and advances the software tail pointer.
    pub fn read_available<F: FnMut(u8)>(&mut self, dma_head: usize, mut consumer: F) -> usize {
        let head = dma_head % CAP;
        let mut count = 0;

        while self.tail != head {
            let b = self.buffer[self.tail];
            self.tail = (self.tail + 1) % CAP;
            consumer(b);
            count += 1;
        }

        count
    }

    /// Pushes a byte into the circular buffer simulating DMA hardware reception.
    pub fn write_dma_byte(&mut self, index: usize, byte: u8) {
        self.buffer[index % CAP] = byte;
    }
}

/// STM32 UART Peripheral Driver for MIDI 2.0 with DMA FIFO support.
///
/// Converts between incoming MIDI 1.0 byte streams and UMPs without memory allocation,
/// tracking running status and interleaving real-time bytes.
pub struct Stm32UartDriver<const RX_CAP: usize = 128, const TX_CAP: usize = 128> {
    pub peripheral: Stm32UartPeripheral,
    pub baud_config: Stm32UartBaudConfig,
    pub group: u8,
    rx_dma: Stm32DmaCircularBuffer<RX_CAP>,
    rx_converter: ByteStreamToUmp,
    tx_converter: UmpToByteStream,
    rx_ump_queue: Queue<Ump, 16>,
    tx_byte_queue: Queue<u8, TX_CAP>,
}

impl<const RX_CAP: usize, const TX_CAP: usize> Stm32UartDriver<RX_CAP, TX_CAP> {
    pub fn new(peripheral: Stm32UartPeripheral, f_ck: u32, baud: u32, group: u8) -> Self {
        let baud_config = Stm32UartBaudConfig::from_clock_and_baud(f_ck, baud);
        Self {
            peripheral,
            baud_config,
            group: group & 0x0F,
            rx_dma: Stm32DmaCircularBuffer::new(),
            rx_converter: ByteStreamToUmp::new(group & 0x0F),
            tx_converter: UmpToByteStream::new(),
            rx_ump_queue: Queue::new(),
            tx_byte_queue: Queue::new(),
        }
    }

    /// Access the DMA circular buffer for hardware configuration
    pub fn dma_buffer_mut(&mut self) -> &mut Stm32DmaCircularBuffer<RX_CAP> {
        &mut self.rx_dma
    }

    /// Polls DMA circular buffer given the current DMA write head position (0..CAP).
    /// Ingests bytes into the `ByteStreamToUmp` parser and queues generated UMPs.
    pub fn poll_dma_rx(&mut self, dma_head: usize) {
        let converter = &mut self.rx_converter;
        let ump_queue = &mut self.rx_ump_queue;

        self.rx_dma.read_available(dma_head, |byte| {
            converter.process_byte(byte);
            while let Some(ump) = converter.pop_ump() {
                let _ = ump_queue.enqueue(ump);
            }
        });
    }

    /// Directly ingests an incoming serial byte (e.g. From an RX interrupt service routine).
    pub fn push_rx_byte(&mut self, byte: u8) {
        self.rx_converter.process_byte(byte);
        while let Some(ump) = self.rx_converter.pop_ump() {
            let _ = self.rx_ump_queue.enqueue(ump);
        }
    }

    /// Pops the next byte waiting to be transmitted over UART TX.
    pub fn pop_tx_byte(&mut self) -> Option<u8> {
        self.tx_byte_queue.dequeue()
    }

    /// Number of bytes waiting in the TX queue.
    pub fn tx_pending(&self) -> usize {
        self.tx_byte_queue.len()
    }
}

impl<const RX_CAP: usize, const TX_CAP: usize> Transport for Stm32UartDriver<RX_CAP, TX_CAP> {
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
    for Stm32UartDriver<RX_CAP, TX_CAP>
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
