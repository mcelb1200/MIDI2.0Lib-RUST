//! Synopsys DWC2 USB-MIDI 2.0 Driver for Raspberry Pi 3B+
//!
//! Provides USB-MIDI 2.0 endpoint handling, Group Terminal Block (GTB) configuration,
//! and Little-Endian UMP packet framing for the BCM2837 Synopsys DWC2 USB OTG controller.

use crate::mmio::DWC2_USB_BASE;
use el_midi2::transport::{
    EmbeddedTransport, GroupTerminalBlock, GroupTerminalBlockType, GroupTerminalProtocol,
    Transport, TransportError, UsbMidiPacketPacker,
};
use el_midi2::ump::Ump;

/// DWC2 Core Register Offsets
pub mod reg {
    /// Core AHB Configuration (offset 0x08)
    pub const GAHBCFG: usize = 0x08;
    /// Core USB Configuration (offset 0x0C)
    pub const GUSBCFG: usize = 0x0C;
    /// Core Reset Register (offset 0x10)
    pub const GRSTCTL: usize = 0x10;
    /// Core Interrupt Register (offset 0x14)
    pub const GINTSTS: usize = 0x14;
    /// Core Interrupt Mask Register (offset 0x18)
    pub const GINTMSK: usize = 0x18;
    /// Device All Endpoints Interrupt Register (offset 0x818)
    pub const DAINT: usize = 0x818;
    /// Device In Endpoint 0 Control (offset 0x900)
    pub const DIEPCTL0: usize = 0x900;
    /// Device Out Endpoint 0 Control (offset 0xB00)
    pub const DOEPCTL0: usize = 0xB00;
}

/// Raspberry Pi 3B+ USB-MIDI 2.0 Native Device Driver
///
/// Bounded zero-allocation queue for USB Bulk/Interrupt endpoints.
pub struct Rpi3UsbMidiDriver<const EP_SIZE: usize = 64, const QUEUE_SIZE: usize = 16> {
    base_addr: usize,
    pub gtb: GroupTerminalBlock,
    rx_queue: [Ump; QUEUE_SIZE],
    rx_head: usize,
    rx_tail: usize,
    rx_count: usize,
    tx_packet_buf: [u8; EP_SIZE],
    tx_packet_len: usize,
    connected: bool,
}

impl<const EP_SIZE: usize, const QUEUE_SIZE: usize> Rpi3UsbMidiDriver<EP_SIZE, QUEUE_SIZE> {
    /// Creates a new USB-MIDI 2.0 driver with a default bidirectional 16-group GTB.
    #[must_use]
    pub const fn new(block_id: u8, group_first: u8, group_count: u8) -> Self {
        Self::with_base(DWC2_USB_BASE, block_id, group_first, group_count)
    }

    /// Creates an instance with a custom base address.
    #[must_use]
    pub const fn with_base(
        base_addr: usize,
        block_id: u8,
        group_first: u8,
        group_count: u8,
    ) -> Self {
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
            base_addr,
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

    /// Returns the DWC2 USB controller MMIO base address.
    #[must_use]
    #[inline]
    pub const fn base_addr(&self) -> usize {
        self.base_addr
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
    for Rpi3UsbMidiDriver<EP_SIZE, QUEUE_SIZE>
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
    for Rpi3UsbMidiDriver<EP_SIZE, QUEUE_SIZE>
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
