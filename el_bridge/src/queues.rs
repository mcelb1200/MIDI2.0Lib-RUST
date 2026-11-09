//! Lock-free SPSC ringbuffer queues for thread-safe audio and MIDI processing.

use ringbuf::HeapRb;

/// SPSC Producer wrapper.
pub struct QueueProducer<T> {
    producer: ringbuf::HeapProducer<T>,
}

impl<T> QueueProducer<T> {
    /// Attempts to push an item without blocking or locking.
    pub fn try_push(&mut self, item: T) -> Result<(), T> {
        self.producer.push(item)
    }

    /// Returns true if the queue is full.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.producer.is_full()
    }

    /// Returns the free capacity remaining in the queue.
    #[must_use]
    pub fn free_len(&self) -> usize {
        self.producer.free_len()
    }
}

/// SPSC Consumer wrapper.
pub struct QueueConsumer<T> {
    consumer: ringbuf::HeapConsumer<T>,
}

impl<T> QueueConsumer<T> {
    /// Attempts to pop an item without blocking or locking.
    pub fn try_pop(&mut self) -> Option<T> {
        self.consumer.pop()
    }

    /// Returns true if the queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.consumer.is_empty()
    }

    /// Returns the number of items available to read.
    #[must_use]
    pub fn len(&self) -> usize {
        self.consumer.len()
    }
}

/// Creates a pair of lock-free Single-Producer Single-Consumer (SPSC) queue endpoints with the specified capacity.
#[must_use]
pub fn create_spsc_queue<T>(capacity: usize) -> (QueueProducer<T>, QueueConsumer<T>) {
    let rb = HeapRb::<T>::new(capacity);
    let (prod, cons) = rb.split();
    (
        QueueProducer { producer: prod },
        QueueConsumer { consumer: cons },
    )
}
