//! DMA-Friendly Ringbuffers & Interrupt-Safe Lock-Free Queues
//!
//! Provides bounded, zero-allocation SPSC (Single Producer Single Consumer) lock-free
//! queues tailored for direct DMA transfer buffers and interrupt handlers.
//!
//! Features:
//! - Atomic head/tail tracking safe for concurrent ISR producer and foreground consumer
//! - Zero-copy slice leasing (`grant_write` / `grant_read`) to pass contiguous slices
//!   directly to/from hardware peripheral DMA FIFOs
//! - Implements the core `Transport` trait for seamless integration into the `el_midi2` runtime.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::transport::{Transport, TransportError};
use crate::ump::Ump;

/// A bounded, interrupt-safe, lock-free SPSC packet queue tailored for DMA transfers.
///
/// `CAP` specifies the buffer capacity in UMP packets. It must be greater than 1.
pub struct PacketQueue<const CAP: usize> {
    buffer: [UnsafeCell<Ump>; CAP],
    head: AtomicUsize, // Read position (owned by consumer)
    tail: AtomicUsize, // Write position (owned by producer)
}

// Safety: PacketQueue is designed for SPSC concurrent access across threads/interrupts.
unsafe impl<const CAP: usize> Sync for PacketQueue<CAP> {}
unsafe impl<const CAP: usize> Send for PacketQueue<CAP> {}

impl<const CAP: usize> Default for PacketQueue<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> PacketQueue<CAP> {
    /// Creates a new `PacketQueue`.
    ///
    /// Compile-time asserts that `CAP > 1` so that the queue can hold at least one packet.
    #[must_use]
    pub const fn new() -> Self {
        assert!(CAP > 1, "PacketQueue capacity must be greater than 1");
        // Initialize UnsafeCell array with zeroed UMPs
        // Const array initialization:
        #[allow(clippy::declare_interior_mutable_const)]
        const EMPTY_CELL: UnsafeCell<Ump> = UnsafeCell::new(Ump { data: [0; 4] });
        Self {
            buffer: [EMPTY_CELL; CAP],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Returns the maximum capacity of the queue.
    #[must_use]
    #[inline]
    pub const fn capacity(&self) -> usize {
        CAP - 1
    }

    /// Returns the number of packets currently available to be read.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        if tail >= head {
            tail - head
        } else {
            CAP + tail - head
        }
    }

    /// Returns `true` if the queue is empty.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the queue is full.
    #[must_use]
    #[inline]
    pub fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    /// Attempts to push a single packet into the queue without blocking.
    ///
    /// Returns `Ok(())` on success, or `Err(TransportError::TxBufferFull)` if full.
    pub fn push(&self, ump: Ump) -> Result<(), TransportError> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        let next_tail = (tail + 1) % CAP;

        if next_tail == head {
            return Err(TransportError::TxBufferFull);
        }

        // Safety: SPSC guarantee ensures only the single producer writes to `buffer[tail]`.
        unsafe {
            *self.buffer[tail].get() = ump;
        }

        self.tail.store(next_tail, Ordering::Release);
        Ok(())
    }

    /// Attempts to pop a single packet from the queue without blocking.
    ///
    /// Returns `Some(ump)` on success, or `None` if the queue is empty.
    pub fn pop(&self) -> Option<Ump> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        if head == tail {
            return None;
        }

        // Safety: SPSC guarantee ensures only the single consumer reads `buffer[head]`.
        let ump = unsafe { *self.buffer[head].get() };
        let next_head = (head + 1) % CAP;
        self.head.store(next_head, Ordering::Release);
        Some(ump)
    }

    /// Leases a contiguous slice of available write space for DMA transfers.
    ///
    /// Returns `(slice_ptr, available_len)` where `slice_ptr` is a pointer to the contiguous
    /// writable memory block inside the ringbuffer.
    ///
    /// After writing up to `available_len` packets, call `commit_write(n)` to update the queue.
    pub fn grant_write(&self) -> (*mut Ump, usize) {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        let available = if head > tail {
            head - tail - 1
        } else if head == 0 {
            CAP - tail - 1
        } else {
            CAP - tail
        };

        // Safety: Raw pointer into the internal contiguous buffer owned by the producer.
        let ptr = self.buffer[tail].get();
        (ptr, available)
    }

    /// Commits `count` packets written via `grant_write`.
    pub fn commit_write(&self, count: usize) {
        let tail = self.tail.load(Ordering::Relaxed);
        let next_tail = (tail + count) % CAP;
        self.tail.store(next_tail, Ordering::Release);
    }

    /// Leases a contiguous slice of available read space for DMA transfers.
    ///
    /// Returns `(slice_ptr, available_len)` where `slice_ptr` is a pointer to the contiguous
    /// readable memory block inside the ringbuffer.
    ///
    /// After reading up to `available_len` packets, call `commit_read(n)` to release the leased space.
    pub fn grant_read(&self) -> (*const Ump, usize) {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        let available = if tail >= head {
            tail - head
        } else {
            CAP - head
        };

        let ptr = self.buffer[head].get() as *const Ump;
        (ptr, available)
    }

    /// Commits `count` packets consumed via `grant_read`.
    pub fn commit_read(&self, count: usize) {
        let head = self.head.load(Ordering::Relaxed);
        let next_head = (head + count) % CAP;
        self.head.store(next_head, Ordering::Release);
    }

    /// Clears all packets from the queue.
    pub fn clear(&self) {
        let head = self.head.load(Ordering::Relaxed);
        self.tail.store(head, Ordering::Release);
    }
}

impl<const CAP: usize> Transport for PacketQueue<CAP> {
    fn send(&mut self, umps: &[Ump]) -> Result<(), TransportError> {
        let needed = umps.len();
        if needed > (self.capacity() - self.len()) {
            return Err(TransportError::TxBufferFull);
        }

        for &ump in umps {
            self.push(ump)?;
        }
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Ump>, TransportError> {
        Ok(self.pop())
    }
}
