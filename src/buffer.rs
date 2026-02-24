use crate::ump::{Ump, MessageType};

/// A parser for a stream of 32-bit words into Universal MIDI Packets (UMP).
///
/// This struct wraps an iterator of `u32` words and implements `Iterator` to yield `Ump` structs.
/// It automatically handles variable-length UMPs (1 to 4 words).
pub struct UmpStreamParser<I>
where
    I: Iterator<Item = u32>,
{
    iter: I,
}

impl<I> UmpStreamParser<I>
where
    I: Iterator<Item = u32>,
{
    /// Creates a new `UmpStreamParser`.
    ///
    /// # Arguments
    ///
    /// * `iter` - An iterator yielding `u32` words.
    ///
    /// # Returns
    ///
    /// A new `UmpStreamParser` instance.
    pub fn new(iter: I) -> Self {
        Self { iter }
    }
}

impl<I> Iterator for UmpStreamParser<I>
where
    I: Iterator<Item = u32>,
{
    type Item = Ump;

    /// Advances the iterator and returns the next available `Ump`.
    ///
    /// This method reads the first word to determine the Message Type and length.
    /// It then reads the remaining words (if any) to construct the full UMP.
    /// If the underlying iterator runs out of items in the middle of a multi-word UMP, `None` is returned
    /// to prevent returning an incomplete packet.
    ///
    /// # Returns
    ///
    /// * `Some(Ump)` - The next valid UMP.
    /// * `None` - If the stream ends or is truncated.
    fn next(&mut self) -> Option<Self::Item> {
        let w1 = self.iter.next()?;
        let message_type_val = ((w1 >> 28) & 0xF) as u8;
        let mt = MessageType::from(message_type_val);
        let mut ump = Ump::new();
        ump.data[0] = w1;

        let count = mt.word_count();
        if count == 1 {
            return Some(ump);
        }

        for i in 1..count {
            if let Some(w) = self.iter.next() {
                ump.data[i] = w;
            } else {
                return None; // Truncated stream
            }
        }

        Some(ump)
    }
}
