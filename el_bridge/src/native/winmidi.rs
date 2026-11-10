//! Windows 11 MIDI Services / UMP API Virtual Endpoint.
//!
//! Provides native MIDI 2.0 endpoint abstractions for Windows 11 MIDI Services
//! (Windows.Devices.Midi2 / IMidiCommand / MidiSession / MidiEndpointConnection).

use crate::native::{NativeEndpointTransport, NativeError};
use el_core::ump::Ump;
use std::collections::VecDeque;

/// Windows MIDI Services Packet format (64-bit timestamp + 32-bit words).
#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinMidiPacket {
    pub timestamp_100ns: u64,
    pub word_count: u8,
    pub words: [u32; 4],
}

impl WinMidiPacket {
    /// Creates a `WinMidiPacket` from a Universal MIDI Packet and a 100ns timestamp.
    pub fn from_ump(ump: &Ump, timestamp_100ns: u64) -> Self {
        let wc = ump.word_count() as u8;
        let mut words = [0_u32; 4];
        for i in 0..(wc as usize) {
            words[i] = ump.data[i];
        }

        Self {
            timestamp_100ns,
            word_count: wc,
            words,
        }
    }

    /// Converts back to a `Ump`.
    pub fn to_ump(&self) -> Ump {
        Ump::new(self.words[0], self.words[1], self.words[2], self.words[3])
    }
}

/// Windows 11 MIDI Services Virtual Endpoint representation.
pub struct WindowsMidiVirtualEndpoint {
    name: String,
    device_id: String,
    loopback_queue: VecDeque<Ump>,
    is_open: bool,
}

unsafe impl Send for WindowsMidiVirtualEndpoint {}

impl WindowsMidiVirtualEndpoint {
    /// Creates a new Windows 11 MIDI Services Virtual Endpoint.
    pub fn new(name: &str) -> Result<Self, NativeError> {
        let device_id = format!("\\\\?\\SWD#MIDISRV#{}#{{ump}}", name);
        Ok(Self {
            name: name.to_string(),
            device_id,
            loopback_queue: VecDeque::new(),
            is_open: true,
        })
    }

    /// Returns the unique Windows device interface ID.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// Simulates incoming packet arrival from Windows MIDI Services message dispatcher.
    pub fn receive_packet(&mut self, packet: WinMidiPacket) {
        self.loopback_queue.push_back(packet.to_ump());
    }
}

impl NativeEndpointTransport for WindowsMidiVirtualEndpoint {
    fn send_ump(&mut self, ump: &Ump) -> Result<(), NativeError> {
        if !self.is_open {
            return Err(NativeError::SendError(
                "Windows MIDI Services session closed".into(),
            ));
        }

        let packet = WinMidiPacket::from_ump(ump, 0);
        self.loopback_queue.push_back(packet.to_ump());
        Ok(())
    }

    fn poll_ump(&mut self) -> Result<Option<Ump>, NativeError> {
        if !self.is_open {
            return Err(NativeError::ReceiveError(
                "Windows MIDI Services session closed".into(),
            ));
        }

        Ok(self.loopback_queue.pop_front())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_active(&self) -> bool {
        self.is_open
    }
}
