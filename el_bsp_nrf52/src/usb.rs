//! Nordic nRF52 USB-MIDI 2.0 Native Device Driver
//!
//! Implements USB-MIDI 2.0 endpoint packet framing and Group Terminal Blocks (GTB)
//! for Nordic nRF52840 and nRF52833 USBD peripherals.

use el_midi2::transport::usb::{
    GroupTerminalBlock, GroupTerminalBlockType, GroupTerminalProtocol, UsbMidiPacketPacker,
};
use el_midi2::transport::{EmbeddedTransport, Transport, TransportError};
use el_midi2::ump::Ump;
use heapless::spsc::Queue;

/// Nordic nRF52 Native USB-MIDI 2.0 Driver
pub struct Nrf52UsbMidiDriver<const RX_CAP: usize = 64, const TX_CAP: usize = 64> {
    pub gtb: GroupTerminalBlock,
    rx_queue: Queue<Ump, RX_CAP>,
    tx_queue: Queue<Ump, TX_CAP>,
    connected: bool,
}

impl<const RX_CAP: usize, const TX_CAP: usize> Nrf52UsbMidiDriver<RX_CAP, TX_CAP> {
    /// Creates a new nRF52 USB-MIDI 2.0 driver with a bidirectional Group Terminal Block.
    pub fn new(gtb_id: u8, group_first: u8, group_count: u8) -> Self {
        let gtb = GroupTerminalBlock::new(
            gtb_id,
            GroupTerminalBlockType::Bidirectional,
            group_first,
            group_count,
        )
        .protocol(GroupTerminalProtocol::Midi2_0);

        Self {
            gtb,
            rx_queue: Queue::new(),
            tx_queue: Queue::new(),
            connected: true,
        }
    }

    /// Sets connection status of the USBD peripheral.
    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
    }

    /// Handles an incoming USB OUT endpoint transfer buffer containing Little-Endian UMP words.
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

impl<const RX_CAP: usize, const TX_CAP: usize> Transport for Nrf52UsbMidiDriver<RX_CAP, TX_CAP> {
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
    for Nrf52UsbMidiDriver<RX_CAP, TX_CAP>
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
