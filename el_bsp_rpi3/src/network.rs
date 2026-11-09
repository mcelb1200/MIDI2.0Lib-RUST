//! Raspberry Pi 3B+ LAN7515 Ethernet UDP Network MIDI 2.0 (AppleMIDI / RTP-MIDI) Adapter
//!
//! Provides zero-allocation UDP socket packetization for Network MIDI 2.0 and AppleMIDI
//! across the Raspberry Pi 3B+ on-board Microchip LAN7515 Ethernet controller.
//! Integrates with the BCM2837 1 MHz System Timer for 100-microsecond clock synchronization.

use crate::timer::Rpi3SystemTimer;
use el_midi2::transport::network::{AppleMidiSession, NetworkTransport, SessionRole};
use el_midi2::transport::{EmbeddedTransport, Transport, TransportError};
use el_midi2::ump::Ump;

/// Configuration parameters for an RPi3 Network MIDI 2.0 endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rpi3EthernetConfig<'a> {
    pub local_name: &'a str,
    pub local_ssrc: u32,
    pub control_port: u16,
    pub midi_port: u16,
    pub mac_address: [u8; 6],
}

impl<'a> Default for Rpi3EthernetConfig<'a> {
    fn default() -> Self {
        Self {
            local_name: "Raspberry Pi 3B+ MIDI 2.0",
            local_ssrc: 0xB827EB01, // B8:27:EB is Raspberry Pi Foundation OUI
            control_port: 5004,
            midi_port: 5005,
            mac_address: [0xB8, 0x27, 0xEB, 0x12, 0x34, 0x56],
        }
    }
}

/// Raspberry Pi 3B+ LAN7515 Ethernet Network MIDI 2.0 Adapter.
///
/// Encapsulates AppleMIDI session handling and RTP MIDI 2.0 packetization
/// over UDP datagram buffers without dynamic memory allocations.
pub struct Rpi3EthernetMidiAdapter<'a, const RX_CAP: usize = 32> {
    pub config: Rpi3EthernetConfig<'a>,
    pub transport: NetworkTransport<RX_CAP>,
    pub timer: Rpi3SystemTimer,
}

impl<'a, const RX_CAP: usize> Rpi3EthernetMidiAdapter<'a, RX_CAP> {
    /// Creates a new Ethernet Network MIDI adapter.
    #[must_use]
    pub const fn new(
        config: Rpi3EthernetConfig<'a>,
        role: SessionRole,
        initiator_token: u32,
    ) -> Self {
        let session = AppleMidiSession::new(role, config.local_ssrc, initiator_token);
        Self {
            config,
            transport: NetworkTransport::new(session),
            timer: Rpi3SystemTimer::new(),
        }
    }

    /// Feeds an incoming UDP control port datagram to the AppleMIDI session state machine.
    ///
    /// If a response packet is required, it is formatted into `reply_buf`.
    pub fn handle_control_datagram(
        &mut self,
        datagram: &[u8],
        reply_buf: &mut [u8],
    ) -> Result<Option<usize>, TransportError> {
        self.transport
            .session
            .process_message(datagram, Some(self.config.local_name), reply_buf)
    }

    /// Feeds an incoming UDP MIDI data port datagram.
    ///
    /// Ingests UMPs into the internal transport queue.
    pub fn handle_midi_datagram(&mut self, datagram: &[u8]) -> Result<usize, TransportError> {
        self.transport.ingest_datagram(datagram)
    }

    /// Prepares an outgoing UDP MIDI datagram containing the given UMP slice.
    pub fn prepare_midi_datagram(
        &mut self,
        umps: &[Ump],
        timestamp: u32,
        dst: &mut [u8],
    ) -> Option<usize> {
        self.transport.pack_outbound_datagram(umps, timestamp, dst)
    }
}

impl<'a, const RX_CAP: usize> Transport for Rpi3EthernetMidiAdapter<'a, RX_CAP> {
    #[inline]
    fn send(&mut self, umps: &[Ump]) -> Result<(), TransportError> {
        self.transport.send(umps)
    }

    #[inline]
    fn recv(&mut self) -> Result<Option<Ump>, TransportError> {
        self.transport.recv()
    }
}

impl<'a, const RX_CAP: usize> EmbeddedTransport for Rpi3EthernetMidiAdapter<'a, RX_CAP> {
    type Error = TransportError;

    #[inline]
    fn read_ump(&mut self) -> Result<Option<Ump>, Self::Error> {
        self.transport.read_ump()
    }

    #[inline]
    fn write_ump(&mut self, ump: &Ump) -> Result<(), Self::Error> {
        self.transport.write_ump(ump)
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        EmbeddedTransport::flush(&mut self.transport)
    }
}
