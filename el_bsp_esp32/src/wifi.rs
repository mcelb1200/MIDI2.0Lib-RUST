//! ESP32 WiFi UDP / Network MIDI 2.0 (RTP-MIDI) Adapter
//!
//! Provides zero-allocation UDP socket packetization for Network MIDI 2.0 and AppleMIDI
//! across ESP32 WiFi networking stacks (LwIP / ESP-NETIF).
//! Fully compatible with Xtensa (ESP32, ESP32-S3) and RISC-V (ESP32-C3, ESP32-C6) processors.

use el_midi2::transport::network::{AppleMidiSession, NetworkTransport, SessionRole};
use el_midi2::transport::{EmbeddedTransport, Transport, TransportError};
use el_midi2::ump::Ump;

/// Configuration parameters for an ESP32 Network MIDI 2.0 endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Esp32NetworkConfig<'a> {
    pub local_name: &'a str,
    pub local_ssrc: u32,
    pub control_port: u16,
    pub midi_port: u16,
}

impl<'a> Default for Esp32NetworkConfig<'a> {
    fn default() -> Self {
        Self {
            local_name: "ESP32 MIDI 2.0",
            local_ssrc: 0x12345678,
            control_port: 5004,
            midi_port: 5005,
        }
    }
}

/// ESP32 WiFi Network MIDI 2.0 UDP Adapter.
///
/// Encapsulates AppleMIDI session handling and RTP MIDI 2.0 packetization
/// over UDP datagram buffers without dynamic memory allocations.
pub struct Esp32WifiMidiAdapter<'a, const RX_CAP: usize = 32> {
    pub config: Esp32NetworkConfig<'a>,
    pub transport: NetworkTransport<RX_CAP>,
}

impl<'a, const RX_CAP: usize> Esp32WifiMidiAdapter<'a, RX_CAP> {
    /// Creates a new WiFi Network MIDI adapter in Responder role.
    #[must_use]
    pub const fn new(
        config: Esp32NetworkConfig<'a>,
        role: SessionRole,
        initiator_token: u32,
    ) -> Self {
        let session = AppleMidiSession::new(role, config.local_ssrc, initiator_token);
        Self {
            config,
            transport: NetworkTransport::new(session),
        }
    }

    /// Feeds an incoming UDP control port datagram to the session state machine.
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

impl<'a, const RX_CAP: usize> Transport for Esp32WifiMidiAdapter<'a, RX_CAP> {
    #[inline]
    fn send(&mut self, umps: &[Ump]) -> Result<(), TransportError> {
        self.transport.send(umps)
    }

    #[inline]
    fn recv(&mut self) -> Result<Option<Ump>, TransportError> {
        self.transport.recv()
    }
}

impl<'a, const RX_CAP: usize> EmbeddedTransport for Esp32WifiMidiAdapter<'a, RX_CAP> {
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
