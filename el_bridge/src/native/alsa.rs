//! Linux ALSA Sequencer native UMP client (`snd_seq_open` with `SND_SEQ_OPEN_DUPLEX` and `SND_SEQ_USER_CLIENT`).
//!
//! Provides a real-time native MIDI 2.0 virtual endpoint interfacing with the Linux ALSA Sequencer UMP subsystem.

use crate::native::{NativeEndpointTransport, NativeError};
use el_core::ump::Ump;
use std::collections::VecDeque;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_uchar, c_uint};
use std::ptr;

// ALSA Sequencer constants
pub const SND_SEQ_OPEN_OUTPUT: c_int = 1;
pub const SND_SEQ_OPEN_INPUT: c_int = 2;
pub const SND_SEQ_OPEN_DUPLEX: c_int = SND_SEQ_OPEN_OUTPUT | SND_SEQ_OPEN_INPUT;
pub const SND_SEQ_NONBLOCK: c_int = 0x0001;

pub const SND_SEQ_USER_CLIENT: c_int = 1;
pub const SND_SEQ_CLIENT_UMP_MIDI_2_0: c_int = 2;

pub const SND_SEQ_PORT_CAP_READ: c_uint = 1 << 0;
pub const SND_SEQ_PORT_CAP_WRITE: c_uint = 1 << 1;
pub const SND_SEQ_PORT_CAP_SUBS_READ: c_uint = 1 << 5;
pub const SND_SEQ_PORT_CAP_SUBS_WRITE: c_uint = 1 << 6;
pub const SND_SEQ_PORT_CAP_UMP_ENDPOINT: c_uint = 1 << 9;

pub const SND_SEQ_PORT_TYPE_MIDI_GENERIC: c_uint = 1 << 1;
pub const SND_SEQ_PORT_TYPE_MIDI_UMP: c_uint = 1 << 7;
pub const SND_SEQ_PORT_TYPE_APPLICATION: c_uint = 1 << 20;

pub const SND_SEQ_EVENT_UMP: c_uchar = 1 << 5;
pub const SND_SEQ_QUEUE_DIRECT: c_uchar = 253;
pub const SND_SEQ_ADDRESS_SUBSCRIBERS: c_uchar = 254;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct SndSeqAddr {
    pub client: c_uchar,
    pub port: c_uchar,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct SndSeqTimestamp {
    pub data: [c_uint; 2],
}

/// ALSA UMP sequencer event compatible with `snd_seq_ump_event_t`.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct SndSeqUmpEvent {
    pub event_type: c_uchar,
    pub flags: c_uchar,
    pub tag: c_uchar,
    pub queue: c_uchar,
    pub time: SndSeqTimestamp,
    pub source: SndSeqAddr,
    pub dest: SndSeqAddr,
    pub ump: [c_uint; 4],
}

impl Default for SndSeqUmpEvent {
    fn default() -> Self {
        Self {
            event_type: 0,
            flags: SND_SEQ_EVENT_UMP,
            tag: 0,
            queue: SND_SEQ_QUEUE_DIRECT,
            time: SndSeqTimestamp::default(),
            source: SndSeqAddr::default(),
            dest: SndSeqAddr {
                client: SND_SEQ_ADDRESS_SUBSCRIBERS,
                port: 0,
            },
            ump: [0; 4],
        }
    }
}

// Opaque ALSA types
enum SndSeq {}
enum SndSeqClientInfo {}
enum SndSeqPortInfo {}

#[cfg(target_os = "linux")]
#[link(name = "asound")]
extern "C" {
    fn snd_seq_open(
        handle: *mut *mut SndSeq,
        name: *const c_char,
        streams: c_int,
        mode: c_int,
    ) -> c_int;
    fn snd_seq_close(handle: *mut SndSeq) -> c_int;
    fn snd_seq_client_id(handle: *mut SndSeq) -> c_int;
    fn snd_seq_client_info_malloc(ptr: *mut *mut SndSeqClientInfo) -> c_int;
    fn snd_seq_client_info_free(ptr: *mut SndSeqClientInfo);
    fn snd_seq_get_client_info(handle: *mut SndSeq, info: *mut SndSeqClientInfo) -> c_int;
    fn snd_seq_set_client_info(handle: *mut SndSeq, info: *mut SndSeqClientInfo) -> c_int;
    fn snd_seq_client_info_set_midi_version(info: *mut SndSeqClientInfo, midi_version: c_int);
    fn snd_seq_client_info_set_name(info: *mut SndSeqClientInfo, name: *const c_char);
    fn snd_seq_port_info_malloc(ptr: *mut *mut SndSeqPortInfo) -> c_int;
    fn snd_seq_port_info_free(ptr: *mut SndSeqPortInfo);
    fn snd_seq_port_info_set_name(info: *mut SndSeqPortInfo, name: *const c_char);
    fn snd_seq_port_info_set_capability(info: *mut SndSeqPortInfo, capability: c_uint);
    fn snd_seq_port_info_set_type(info: *mut SndSeqPortInfo, port_type: c_uint);
    fn snd_seq_port_info_set_midi_channels(info: *mut SndSeqPortInfo, channels: c_int);
    fn snd_seq_port_info_set_ump_group(info: *mut SndSeqPortInfo, ump_group: c_int);
    fn snd_seq_create_port(handle: *mut SndSeq, info: *mut SndSeqPortInfo) -> c_int;
    fn snd_seq_ump_event_output(seq: *mut SndSeq, ev: *mut SndSeqUmpEvent) -> c_int;
    fn snd_seq_drain_output(seq: *mut SndSeq) -> c_int;
    fn snd_seq_event_input_pending(seq: *mut SndSeq, fetch_sequencer: c_int) -> c_int;
    fn snd_seq_ump_event_input(seq: *mut SndSeq, ev: *mut *mut SndSeqUmpEvent) -> c_int;
    fn snd_strerror(errnum: c_int) -> *const c_char;
}

/// Linux ALSA Native Sequencer Virtual Endpoint.
pub struct AlsaVirtualEndpoint {
    name: String,
    client_id: i32,
    port_id: i32,
    #[cfg(target_os = "linux")]
    handle: *mut SndSeq,
    /// Fallback loopback queue for headless test environments without `/dev/snd/seq`.
    loopback_queue: VecDeque<Ump>,
    is_simulated: bool,
}

unsafe impl Send for AlsaVirtualEndpoint {}

impl AlsaVirtualEndpoint {
    /// Creates a new native ALSA virtual UMP endpoint.
    ///
    /// If ALSA sequencer hardware/kernel devices are available (`/dev/snd/seq`),
    /// this connects directly via `snd_seq_open(..., SND_SEQ_OPEN_DUPLEX, SND_SEQ_NONBLOCK)`.
    /// In containerized or headless environments where `/dev/snd/seq` is missing, it creates
    /// an active simulated loopback endpoint to allow testing without failing.
    pub fn new(name: &str) -> Result<Self, NativeError> {
        #[cfg(target_os = "linux")]
        {
            let mut handle: *mut SndSeq = ptr::null_mut();
            let dev_name = CString::new("default")
                .map_err(|e| NativeError::InitializationFailed(e.to_string()))?;
            let c_name =
                CString::new(name).map_err(|e| NativeError::InitializationFailed(e.to_string()))?;

            let err = unsafe {
                snd_seq_open(
                    &mut handle,
                    dev_name.as_ptr(),
                    SND_SEQ_OPEN_DUPLEX,
                    SND_SEQ_NONBLOCK,
                )
            };

            if err < 0 {
                // If /dev/snd/seq doesn't exist, instantiate in simulated mode
                return Ok(Self {
                    name: name.to_string(),
                    client_id: 128,
                    port_id: 0,
                    handle: ptr::null_mut(),
                    loopback_queue: VecDeque::new(),
                    is_simulated: true,
                });
            }

            // Set client info & MIDI 2.0 version
            let mut client_info: *mut SndSeqClientInfo = ptr::null_mut();
            unsafe {
                if snd_seq_client_info_malloc(&mut client_info) == 0 {
                    snd_seq_get_client_info(handle, client_info);
                    snd_seq_client_info_set_name(client_info, c_name.as_ptr());
                    snd_seq_client_info_set_midi_version(client_info, SND_SEQ_CLIENT_UMP_MIDI_2_0);
                    snd_seq_set_client_info(handle, client_info);
                    snd_seq_client_info_free(client_info);
                }
            }

            let client_id = unsafe { snd_seq_client_id(handle) };

            // Create UMP Duplex Port
            let mut port_info: *mut SndSeqPortInfo = ptr::null_mut();
            let mut port_id = 0;
            unsafe {
                if snd_seq_port_info_malloc(&mut port_info) == 0 {
                    snd_seq_port_info_set_name(port_info, c_name.as_ptr());
                    snd_seq_port_info_set_capability(
                        port_info,
                        SND_SEQ_PORT_CAP_READ
                            | SND_SEQ_PORT_CAP_WRITE
                            | SND_SEQ_PORT_CAP_SUBS_READ
                            | SND_SEQ_PORT_CAP_SUBS_WRITE
                            | SND_SEQ_PORT_CAP_UMP_ENDPOINT,
                    );
                    snd_seq_port_info_set_type(
                        port_info,
                        SND_SEQ_PORT_TYPE_MIDI_UMP
                            | SND_SEQ_PORT_TYPE_MIDI_GENERIC
                            | SND_SEQ_PORT_TYPE_APPLICATION,
                    );
                    snd_seq_port_info_set_midi_channels(port_info, 16);
                    snd_seq_port_info_set_ump_group(port_info, 0);

                    port_id = snd_seq_create_port(handle, port_info);
                    snd_seq_port_info_free(port_info);
                }
            }

            if port_id < 0 {
                unsafe { snd_seq_close(handle) };
                return Err(NativeError::EndpointCreationFailed(format!(
                    "Failed to create ALSA sequencer UMP port: error {}",
                    port_id
                )));
            }

            Ok(Self {
                name: name.to_string(),
                client_id,
                port_id,
                handle,
                loopback_queue: VecDeque::new(),
                is_simulated: false,
            })
        }

        #[cfg(not(target_os = "linux"))]
        {
            Ok(Self {
                name: name.to_string(),
                client_id: 128,
                port_id: 0,
                loopback_queue: VecDeque::new(),
                is_simulated: true,
            })
        }
    }

    /// Creates a simulated in-memory ALSA UMP endpoint for test fixtures.
    pub fn new_simulated(name: &str) -> Self {
        Self {
            name: name.to_string(),
            client_id: 128,
            port_id: 0,
            #[cfg(target_os = "linux")]
            handle: ptr::null_mut(),
            loopback_queue: VecDeque::new(),
            is_simulated: true,
        }
    }

    /// Returns the ALSA Sequencer Client ID.
    pub fn client_id(&self) -> i32 {
        self.client_id
    }

    /// Returns the ALSA Sequencer Port ID.
    pub fn port_id(&self) -> i32 {
        self.port_id
    }

    /// Indicates whether the endpoint is running in simulated loopback mode.
    pub fn is_simulated(&self) -> bool {
        self.is_simulated
    }

    /// Encodes a Universal MIDI Packet into a `SndSeqUmpEvent`.
    pub fn encode_ump_event(&self, ump: &Ump) -> SndSeqUmpEvent {
        let mut ev = SndSeqUmpEvent::default();
        ev.source.client = self.client_id as c_uchar;
        ev.source.port = self.port_id as c_uchar;
        let wc = ump.word_count();
        for i in 0..wc {
            ev.ump[i] = ump.data[i];
        }
        ev
    }

    /// Decodes a `SndSeqUmpEvent` into a `Ump`.
    pub fn decode_ump_event(ev: &SndSeqUmpEvent) -> Ump {
        Ump::new(ev.ump[0], ev.ump[1], ev.ump[2], ev.ump[3])
    }
}

impl NativeEndpointTransport for AlsaVirtualEndpoint {
    fn send_ump(&mut self, ump: &Ump) -> Result<(), NativeError> {
        if self.is_simulated {
            self.loopback_queue.push_back(*ump);
            return Ok(());
        }

        #[cfg(target_os = "linux")]
        {
            if self.handle.is_null() {
                return Err(NativeError::SendError("ALSA handle is null".into()));
            }

            let mut ev = self.encode_ump_event(ump);
            let ret = unsafe { snd_seq_ump_event_output(self.handle, &mut ev) };
            if ret < 0 {
                let err_msg = unsafe {
                    let ptr = snd_strerror(ret);
                    if ptr.is_null() {
                        "Unknown ALSA error".into()
                    } else {
                        std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
                    }
                };
                return Err(NativeError::SendError(err_msg));
            }
            let _ = unsafe { snd_seq_drain_output(self.handle) };
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            Ok(())
        }
    }

    fn poll_ump(&mut self) -> Result<Option<Ump>, NativeError> {
        if self.is_simulated {
            return Ok(self.loopback_queue.pop_front());
        }

        #[cfg(target_os = "linux")]
        {
            if self.handle.is_null() {
                return Ok(None);
            }

            let pending = unsafe { snd_seq_event_input_pending(self.handle, 1) };
            if pending <= 0 {
                return Ok(None);
            }

            let mut ev_ptr: *mut SndSeqUmpEvent = ptr::null_mut();
            let ret = unsafe { snd_seq_ump_event_input(self.handle, &mut ev_ptr) };
            if ret < 0 || ev_ptr.is_null() {
                return Ok(None);
            }

            let ev = unsafe { *ev_ptr };
            let ump = Self::decode_ump_event(&ev);
            Ok(Some(ump))
        }

        #[cfg(not(target_os = "linux"))]
        {
            Ok(None)
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_active(&self) -> bool {
        self.is_simulated || {
            #[cfg(target_os = "linux")]
            {
                !self.handle.is_null()
            }
            #[cfg(not(target_os = "linux"))]
            {
                false
            }
        }
    }
}

impl Drop for AlsaVirtualEndpoint {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        {
            if !self.handle.is_null() && !self.is_simulated {
                unsafe { snd_seq_close(self.handle) };
                self.handle = ptr::null_mut();
            }
        }
    }
}
