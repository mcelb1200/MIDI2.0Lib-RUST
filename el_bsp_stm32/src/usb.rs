//! STM32 USB-MIDI 2.0 Native Device Driver
//!
//! Implements USB-MIDI 2.0 endpoint packet framing and Group Terminal Blocks (GTB)
//! for STM32 USB OTG Full-Speed (OTG_FS) and High-Speed (OTG_HS) peripherals.

use el_midi2::transport::usb::{
    GroupTerminalBlock, GroupTerminalBlockType, GroupTerminalProtocol, UsbMidiPacketPacker,
};
use el_midi2::transport::{EmbeddedTransport, Transport, TransportError};
use el_midi2::ump::Ump;
use heapless::spsc::Queue;

/// STM32 USB Peripheral Core type
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Stm32UsbCore {
    /// Full-Speed On-The-Go Core (e.g. STM32F401, STM32F411, STM32G4)
    OtgFs,
    /// High-Speed On-The-Go Core (e.g. STM32F429, STM32F7, STM32H7)
    OtgHs,
}

/// STM32 Native USB-MIDI 2.0 Driver
pub struct Stm32UsbMidiDriver<const RX_CAP: usize = 64, const TX_CAP: usize = 64> {
    pub core: Stm32UsbCore,
    pub gtb: GroupTerminalBlock,
    rx_queue: Queue<Ump, RX_CAP>,
    tx_queue: Queue<Ump, TX_CAP>,
    connected: bool,
}

impl<const RX_CAP: usize, const TX_CAP: usize> Stm32UsbMidiDriver<RX_CAP, TX_CAP> {
    /// Creates a new STM32 USB-MIDI 2.0 driver with a default bidirectional Group Terminal Block.
    pub fn new(core: Stm32UsbCore, gtb_id: u8, group_first: u8, group_count: u8) -> Self {
        let gtb = GroupTerminalBlock::new(
            gtb_id,
            GroupTerminalBlockType::Bidirectional,
            group_first,
            group_count,
        )
        .protocol(GroupTerminalProtocol::Midi2_0);

        Self {
            core,
            gtb,
            rx_queue: Queue::new(),
            tx_queue: Queue::new(),
            connected: true,
        }
    }

    /// Sets connection status of the USB peripheral.
    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
    }

    /// Handles an incoming USB OUT endpoint transfer buffer containing Little-Endian UMP words.
    /// Ingests complete 32-bit words into the internal RX queue.
    pub fn handle_usb_out_packet(&mut self, packet: &[u8]) -> Result<usize, TransportError> {
        let mut offset = 0;
        let mut count = 0;

        while offset < packet.len() {
            if let Some((ump, consumed)) = UsbMidiPacketPacker::unpack_ump(&packet[offset..]) {
                self.rx_queue
                    .enqueue(ump)
                    .map_err(|_| TransportError::RxBufferOverflow)?;
                offset += consumed;
                count += ump.word_count();
            } else {
                break;
            }
        }

        Ok(count)
    }

    /// Prepares an outgoing USB IN endpoint transfer buffer with UMPs from the TX queue.
    /// Formats UMP 32-bit words as Little-Endian bytes into `out_buffer`.
    /// Returns the number of bytes written to `out_buffer`.
    pub fn fill_usb_in_packet(
        &mut self,
        packets: &[Ump],
        out_buffer: &mut [u8],
    ) -> Result<usize, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        UsbMidiPacketPacker::pack_umps(packets, out_buffer).ok_or(TransportError::TxBufferFull)
    }
}

impl<const RX_CAP: usize, const TX_CAP: usize> Transport for Stm32UsbMidiDriver<RX_CAP, TX_CAP> {
    fn send(&mut self, packets: &[Ump]) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        for ump in packets {
            self.tx_queue
                .enqueue(*ump)
                .map_err(|_| TransportError::TxBufferFull)?;
        }
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Ump>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        Ok(self.rx_queue.dequeue())
    }
}

impl<const RX_CAP: usize, const TX_CAP: usize> EmbeddedTransport
    for Stm32UsbMidiDriver<RX_CAP, TX_CAP>
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
