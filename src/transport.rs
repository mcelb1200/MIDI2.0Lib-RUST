//! MIDI 2.0 Core Transport Traits and Mock Transport
//!
//! Provides hardware- and platform-agnostic abstractions for transmitting and receiving
//! Universal MIDI Packets (UMP) under `no_std` embedded environments.

use crate::ump::Ump;

/// Errors that can occur during transport operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    /// Transmit buffer or hardware FIFO is full.
    TxBufferFull,
    /// Receive buffer or hardware FIFO overflowed.
    RxBufferOverflow,
    /// Device or connection is not open / disconnected.
    NotConnected,
    /// Hardware or communication bus error occurred.
    HardwareError,
    /// Operation timed out.
    Timeout,
}

/// Core synchronous transport abstraction for sending and receiving UMP packets.
///
/// Designed to be zero-allocation, suitable for embedded polling or interrupt-driven handlers.
pub trait Transport {
    /// Transmits a slice of UMP packets across the transport.
    ///
    /// # Errors
    /// Returns `TransportError` if transmission fails or the output buffer is full.
    fn send(&mut self, umps: &[Ump]) -> Result<(), TransportError>;

    /// Attempts to receive a single UMP packet from the transport without blocking.
    ///
    /// # Returns
    /// - `Ok(Some(ump))` if a packet was available.
    /// - `Ok(None)` if no packet is currently available.
    /// - `Err(TransportError)` on communication or overflow error.
    fn recv(&mut self) -> Result<Option<Ump>, TransportError>;

    /// Flushes any pending outbound buffers.
    #[inline]
    fn flush(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
}

/// Core asynchronous transport abstraction for async/await embedded runtimes (e.g. Embassy).
pub trait AsyncTransport {
    /// Asynchronously transmits a slice of UMP packets.
    fn send(
        &mut self,
        umps: &[Ump],
    ) -> impl core::future::Future<Output = Result<(), TransportError>>;

    /// Asynchronously receives a single UMP packet.
    fn recv(&mut self) -> impl core::future::Future<Output = Result<Ump, TransportError>>;
}

/// A zero-allocation, bounded ringbuffer transport for testing and local loopback.
#[derive(Debug)]
pub struct MockTransport<const CAP: usize> {
    buffer: [Ump; CAP],
    head: usize,
    tail: usize,
    count: usize,
    connected: bool,
}

impl<const CAP: usize> Default for MockTransport<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> MockTransport<CAP> {
    /// Creates a new, connected `MockTransport` with capacity `CAP`.
    ///
    /// Compile-time asserts that `CAP > 0` to prevent division or modulo by zero.
    #[must_use]
    pub const fn new() -> Self {
        assert!(CAP > 0, "MockTransport capacity must be greater than 0");
        Self {
            buffer: [Ump { data: [0; 4] }; CAP],
            head: 0,
            tail: 0,
            count: 0,
            connected: true,
        }
    }

    /// Sets the connection state of the mock transport.
    #[inline]
    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
    }

    /// Returns the number of packets currently queued in the ringbuffer.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns `true` if the ringbuffer is empty.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Clears all buffered packets.
    #[inline]
    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.count = 0;
    }
}

impl<const CAP: usize> Transport for MockTransport<CAP> {
    fn send(&mut self, umps: &[Ump]) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        if self.count + umps.len() > CAP {
            return Err(TransportError::TxBufferFull);
        }

        for &ump in umps {
            self.buffer[self.tail] = ump;
            self.tail = (self.tail + 1) % CAP;
            self.count += 1;
        }

        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Ump>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        if self.count == 0 {
            return Ok(None);
        }

        let ump = self.buffer[self.head];
        self.head = (self.head + 1) % CAP;
        self.count -= 1;

        Ok(Some(ump))
    }
}
