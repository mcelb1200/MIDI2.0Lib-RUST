//! RP2040 Native USB Peripheral MIDI 2.0 Driver
//!
//! Provides USB-MIDI 2.0 endpoint packet handling, Group Terminal Block (GTB) configuration,
//! and Little-Endian UMP packet framing for Raspberry Pi RP2040 microcontrollers.

use el_midi2::transport::{
    EmbeddedTransport, GroupTerminalBlock, GroupTerminalBlockType, GroupTerminalProtocol,
    Transport, TransportError, UsbMidiPacketPacker,
};
use el_midi2::ump::Ump;

/// RP2040 Native USB-MIDI 2.0 Device Driver.
///
/// Bounded zero-allocation queue for USB Bulk/Interrupt endpoints.
pub struct Rp2040UsbMidiDriver<const EP_SIZE: usize = 64, const QUEUE_SIZE: usize = 16> {
    pub gtb: GroupTerminalBlock,
    rx_queue: [Ump; QUEUE_SIZE],
    rx_head: usize,
    rx_tail: usize,
    rx_count: usize,
    tx_packet_buf: [u8; EP_SIZE],
    tx_packet_len: usize,
    connected: bool,
}

impl<const EP_SIZE: usize, const QUEUE_SIZE: usize> Rp2040UsbMidiDriver<EP_SIZE, QUEUE_SIZE> {
    /// Creates a new RP2040 USB-MIDI 2.0 driver with a default bidirectional Group Terminal Block.
    #[must_use]
    pub const fn new(block_id: u8, group_first: u8, group_count: u8) -> Self {
        assert!(EP_SIZE >= 4, "Endpoint packet size must be >= 4 bytes");
        assert!(QUEUE_SIZE > 0, "Queue size must be > 0");

        let gtb = GroupTerminalBlock::new(
            block_id,
            GroupTerminalBlockType::Bidirectional,
            group_first,
            group_count,
        )
        .protocol(GroupTerminalProtocol::Midi2_0);

        Self {
            gtb,
            rx_queue: [Ump { data: [0; 4] }; QUEUE_SIZE],
            rx_head: 0,
            rx_tail: 0,
            rx_count: 0,
            tx_packet_buf: [0; EP_SIZE],
            tx_packet_len: 0,
            connected: true,
        }
    }

    /// Sets connection status of the USB bus.
    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
    }

    /// Ingests a raw USB packet received from an OUT endpoint into UMPs.
    pub fn handle_usb_out_packet(&mut self, packet: &[u8]) -> Result<usize, TransportError> {
        let mut offset = 0;
        let mut count = 0;

        while offset < packet.len() {
            if let Some((ump, consumed)) = UsbMidiPacketPacker::unpack_ump(&packet[offset..]) {
                if self.rx_count < QUEUE_SIZE {
                    self.rx_queue[self.rx_tail] = ump;
                    self.rx_tail = (self.rx_tail + 1) % QUEUE_SIZE;
                    self.rx_count += 1;
                    count += 1;
                }
                offset += consumed;
            } else {
                break;
            }
        }

        Ok(count)
    }

    /// Prepares an IN endpoint packet from pending UMPs to be transmitted over USB.
    ///
    /// Returns the number of bytes populated into `dst`.
    pub fn fill_usb_in_packet(&mut self, umps: &[Ump], dst: &mut [u8]) -> Option<usize> {
        if !self.connected {
            return None;
        }
        UsbMidiPacketPacker::pack_umps(umps, dst)
    }
}

impl<const EP_SIZE: usize, const QUEUE_SIZE: usize> Transport
    for Rp2040UsbMidiDriver<EP_SIZE, QUEUE_SIZE>
{
    fn send(&mut self, umps: &[Ump]) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }

        let written = UsbMidiPacketPacker::pack_umps(umps, &mut self.tx_packet_buf)
            .ok_or(TransportError::TxBufferFull)?;
        self.tx_packet_len = written;
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Ump>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        if self.rx_count == 0 {
            return Ok(None);
        }

        let ump = self.rx_queue[self.rx_head];
        self.rx_head = (self.rx_head + 1) % QUEUE_SIZE;
        self.rx_count -= 1;
        Ok(Some(ump))
    }
}

impl<const EP_SIZE: usize, const QUEUE_SIZE: usize> EmbeddedTransport
    for Rp2040UsbMidiDriver<EP_SIZE, QUEUE_SIZE>
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
