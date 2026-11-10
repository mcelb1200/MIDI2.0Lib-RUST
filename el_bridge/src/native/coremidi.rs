//! macOS CoreMIDI 2.0 Virtual Endpoint (`MIDIEndpointCreateWithBlock` / `kMIDIProtocol_2_0`).
//!
//! Exposes native MIDI 2.0 Universal MIDI Packet (UMP) communication on macOS
//! via CoreMIDI 2.0 `MIDIEventList` and `MIDIEventPacket` APIs.

use crate::native::{NativeEndpointTransport, NativeError};
use el_core::ump::Ump;
use std::collections::VecDeque;
#[cfg(target_os = "macos")]
use std::os::raw::{c_int, c_void};

/// CoreMIDI Protocol constants
pub const K_MIDI_PROTOCOL_1_0: i32 = 1;
pub const K_MIDI_PROTOCOL_2_0: i32 = 2;

/// CoreMIDI 2.0 `MIDIEventPacket` layout (packed 64-bit timestamp, word count, up to 64 words).
#[repr(C)]
#[derive(Clone, Debug)]
pub struct MidiEventPacket {
    pub time_stamp: u64,
    pub word_count: u32,
    pub words: [u32; 64],
}

impl Default for MidiEventPacket {
    fn default() -> Self {
        Self {
            time_stamp: 0,
            word_count: 0,
            words: [0; 64],
        }
    }
}

/// CoreMIDI 2.0 `MIDIEventList` container header.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct MidiEventList {
    pub protocol: i32,
    pub num_packets: u32,
    pub packet: [MidiEventPacket; 1],
}

impl Default for MidiEventList {
    fn default() -> Self {
        Self {
            protocol: K_MIDI_PROTOCOL_2_0,
            num_packets: 1,
            packet: [MidiEventPacket::default()],
        }
    }
}

impl MidiEventList {
    /// Constructs a `MIDIEventList` holding a single Universal MIDI Packet.
    pub fn from_ump(ump: &Ump, timestamp: u64) -> Self {
        let wc = ump.word_count() as u32;
        let mut pkt = MidiEventPacket {
            time_stamp: timestamp,
            word_count: wc,
            words: [0; 64],
        };
        for i in 0..(wc as usize) {
            pkt.words[i] = ump.data[i];
        }

        Self {
            protocol: K_MIDI_PROTOCOL_2_0,
            num_packets: 1,
            packet: [pkt],
        }
    }

    /// Extracts Universal MIDI Packets from this event list.
    pub fn extract_umps(&self) -> Vec<Ump> {
        let mut umps = Vec::new();
        for p in 0..(self.num_packets as usize) {
            if p >= self.packet.len() {
                break;
            }
            let pkt = &self.packet[p];
            let mut idx = 0;
            while idx < pkt.word_count as usize {
                let first_word = pkt.words[idx];
                let mt = (first_word >> 28) & 0xF;
                let needed = match mt {
                    0x0 | 0x1 | 0x2 | 0x6 | 0x7 => 1,
                    0x3 | 0x4 => 2,
                    0x5 | 0xD | 0xF => 4,
                    _ => 1,
                };

                if idx + needed <= pkt.word_count as usize {
                    let w1 = if needed > 1 { pkt.words[idx + 1] } else { 0 };
                    let w2 = if needed > 2 { pkt.words[idx + 2] } else { 0 };
                    let w3 = if needed > 3 { pkt.words[idx + 3] } else { 0 };
                    umps.push(Ump::new(first_word, w1, w2, w3));
                    idx += needed;
                } else {
                    break;
                }
            }
        }
        umps
    }
}

#[cfg(target_os = "macos")]
#[link(name = "CoreMIDI", kind = "framework")]
extern "C" {
    fn MIDIClientCreateWithBlock(
        name: *const c_void,
        out_client: *mut u32,
        notify_block: *const c_void,
    ) -> c_int;
    fn MIDIClientDispose(client: u32) -> c_int;
    fn MIDIDestinationCreateWithProtocol(
        client: u32,
        name: *const c_void,
        protocol: i32,
        out_dest: *mut u32,
        read_block: *const c_void,
    ) -> c_int;
    fn MIDIReceivedEventList(src: u32, evtlist: *const MidiEventList) -> c_int;
}

/// macOS CoreMIDI 2.0 Virtual Endpoint.
pub struct CoreMidiVirtualEndpoint {
    name: String,
    protocol: i32,
    #[allow(dead_code)]
    client_ref: u32,
    endpoint_ref: u32,
    tx_queue: VecDeque<Ump>,
    rx_queue: VecDeque<Ump>,
    is_active: bool,
}

unsafe impl Send for CoreMidiVirtualEndpoint {}

impl CoreMidiVirtualEndpoint {
    /// Creates a new macOS CoreMIDI 2.0 virtual endpoint.
    pub fn new(name: &str) -> Result<Self, NativeError> {
        Self::with_protocol(name, K_MIDI_PROTOCOL_2_0)
    }

    /// Creates a virtual endpoint specifying the protocol version (`kMIDIProtocol_1_0` or `kMIDIProtocol_2_0`).
    pub fn with_protocol(name: &str, protocol: i32) -> Result<Self, NativeError> {
        #[cfg(target_os = "macos")]
        {
            let mut client_ref = 0_u32;
            let mut endpoint_ref = 0_u32;
            let status = unsafe {
                let client_res =
                    MIDIClientCreateWithBlock(std::ptr::null(), &mut client_ref, std::ptr::null());
                if client_res == 0 && client_ref != 0 {
                    MIDIDestinationCreateWithProtocol(
                        client_ref,
                        std::ptr::null(),
                        protocol,
                        &mut endpoint_ref,
                        std::ptr::null(),
                    )
                } else {
                    -1
                }
            };
            let endpoint_valid = status == 0 && endpoint_ref != 0;

            Ok(Self {
                name: name.to_string(),
                protocol,
                client_ref: if endpoint_valid { client_ref } else { 0 },
                endpoint_ref: if endpoint_valid { endpoint_ref } else { 1 },
                tx_queue: VecDeque::new(),
                rx_queue: VecDeque::new(),
                is_active: true,
            })
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(Self {
                name: name.to_string(),
                protocol,
                client_ref: 0,
                endpoint_ref: 1,
                tx_queue: VecDeque::new(),
                rx_queue: VecDeque::new(),
                is_active: true,
            })
        }
    }

    /// Returns the CoreMIDI protocol version (1 for MIDI 1.0, 2 for MIDI 2.0).
    pub fn protocol(&self) -> i32 {
        self.protocol
    }

    /// Returns the CoreMIDI endpoint reference.
    pub fn endpoint_ref(&self) -> u32 {
        self.endpoint_ref
    }

    /// Injects a `MIDIEventList` into this virtual endpoint (e.g. from DAW/OS receive callback).
    pub fn receive_event_list(&mut self, event_list: &MidiEventList) {
        let umps = event_list.extract_umps();
        for ump in umps {
            self.rx_queue.push_back(ump);
        }
    }

    /// Pops a transmitted UMP from the output queue (for inspection/test verification).
    pub fn pop_sent_ump(&mut self) -> Option<Ump> {
        self.tx_queue.pop_front()
    }
}

impl NativeEndpointTransport for CoreMidiVirtualEndpoint {
    fn send_ump(&mut self, ump: &Ump) -> Result<(), NativeError> {
        if !self.is_active {
            return Err(NativeError::SendError("CoreMIDI endpoint inactive".into()));
        }

        #[cfg(target_os = "macos")]
        {
            let event_list = MidiEventList::from_ump(ump, 0);
            if self.endpoint_ref != 0 {
                unsafe {
                    let _ = MIDIReceivedEventList(self.endpoint_ref, &event_list);
                }
            }
        }

        self.tx_queue.push_back(*ump);
        Ok(())
    }

    fn poll_ump(&mut self) -> Result<Option<Ump>, NativeError> {
        if !self.is_active {
            return Err(NativeError::ReceiveError(
                "CoreMIDI endpoint inactive".into(),
            ));
        }

        Ok(self.rx_queue.pop_front())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_active(&self) -> bool {
        self.is_active
    }
}

impl Drop for CoreMidiVirtualEndpoint {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        {
            if self.client_ref != 0 {
                unsafe {
                    let _ = MIDIClientDispose(self.client_ref);
                }
            }
        }
    }
}
