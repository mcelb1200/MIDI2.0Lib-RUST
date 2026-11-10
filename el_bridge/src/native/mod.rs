//! Native OS Virtual Endpoint Transports for Linux ALSA, macOS CoreMIDI 2.0, and Windows 11 MIDI Services.

pub mod alsa;
pub mod coremidi;
pub mod winmidi;

use el_core::ump::Ump;
use std::fmt;

/// Unified errors originating from native platform MIDI 2.0 endpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeError {
    /// Device, driver, or virtual endpoint initialization failed.
    InitializationFailed(String),
    /// Port, client, or endpoint creation failed.
    EndpointCreationFailed(String),
    /// Failed to transmit a Universal MIDI Packet.
    SendError(String),
    /// Failed to receive or read a Universal MIDI Packet.
    ReceiveError(String),
    /// Operation timed out.
    Timeout,
    /// Operating system platform subsystem unavailable (e.g. ALSA `/dev/snd/seq` missing).
    SubsystemUnavailable(String),
    /// An operation is unsupported on this platform or architecture.
    Unsupported(String),
}

impl fmt::Display for NativeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InitializationFailed(msg) => write!(f, "Initialization failed: {}", msg),
            Self::EndpointCreationFailed(msg) => write!(f, "Endpoint creation failed: {}", msg),
            Self::SendError(msg) => write!(f, "Send error: {}", msg),
            Self::ReceiveError(msg) => write!(f, "Receive error: {}", msg),
            Self::Timeout => write!(f, "Operation timed out"),
            Self::SubsystemUnavailable(msg) => write!(f, "Native subsystem unavailable: {}", msg),
            Self::Unsupported(msg) => write!(f, "Operation unsupported: {}", msg),
        }
    }
}

impl std::error::Error for NativeError {}

/// High-level trait for Native Virtual MIDI 2.0 Endpoint Transports.
pub trait NativeEndpointTransport: Send {
    /// Transmits a Universal MIDI Packet through the virtual endpoint.
    fn send_ump(&mut self, ump: &Ump) -> Result<(), NativeError>;

    /// Polls for an incoming Universal MIDI Packet (non-blocking).
    fn poll_ump(&mut self) -> Result<Option<Ump>, NativeError>;

    /// Returns the friendly name of this native virtual endpoint.
    fn name(&self) -> &str;

    /// Checks if the native virtual endpoint is currently connected and active.
    fn is_active(&self) -> bool;
}

/// Native Virtual Endpoint abstraction wrapper.
pub enum NativeVirtualEndpoint {
    /// Linux ALSA native sequencer UMP client.
    Alsa(alsa::AlsaVirtualEndpoint),
    /// macOS CoreMIDI 2.0 virtual endpoint.
    CoreMidi(coremidi::CoreMidiVirtualEndpoint),
    /// Windows 11 MIDI Services / UMP API virtual endpoint.
    WindowsMidi(winmidi::WindowsMidiVirtualEndpoint),
}

impl NativeEndpointTransport for NativeVirtualEndpoint {
    fn send_ump(&mut self, ump: &Ump) -> Result<(), NativeError> {
        match self {
            Self::Alsa(ep) => ep.send_ump(ump),
            Self::CoreMidi(ep) => ep.send_ump(ump),
            Self::WindowsMidi(ep) => ep.send_ump(ump),
        }
    }

    fn poll_ump(&mut self) -> Result<Option<Ump>, NativeError> {
        match self {
            Self::Alsa(ep) => ep.poll_ump(),
            Self::CoreMidi(ep) => ep.poll_ump(),
            Self::WindowsMidi(ep) => ep.poll_ump(),
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Alsa(ep) => ep.name(),
            Self::CoreMidi(ep) => ep.name(),
            Self::WindowsMidi(ep) => ep.name(),
        }
    }

    fn is_active(&self) -> bool {
        match self {
            Self::Alsa(ep) => ep.is_active(),
            Self::CoreMidi(ep) => ep.is_active(),
            Self::WindowsMidi(ep) => ep.is_active(),
        }
    }
}

impl NativeVirtualEndpoint {
    /// Creates a virtual endpoint using the best available native transport for the current OS.
    pub fn create_default(name: &str) -> Result<Self, NativeError> {
        #[cfg(target_os = "linux")]
        {
            alsa::AlsaVirtualEndpoint::new(name).map(Self::Alsa)
        }
        #[cfg(target_os = "macos")]
        {
            coremidi::CoreMidiVirtualEndpoint::new(name).map(Self::CoreMidi)
        }
        #[cfg(target_os = "windows")]
        {
            winmidi::WindowsMidiVirtualEndpoint::new(name).map(Self::WindowsMidi)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Err(NativeError::Unsupported(format!(
                "Native virtual endpoint is unsupported on {}",
                std::env::consts::OS
            )))
        }
    }
}
