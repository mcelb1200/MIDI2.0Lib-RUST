use crate::ump::Ump;

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
    #[must_use]
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
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let w1 = self.iter.next()?;
        // Fast-path MessageType extraction without branching or enum conversion overhead
        // Grouping matching directly on the MT bounds limits memory lookup overhead
        // and enables the compiler to generate an optimized branch table.
        // We explicitly return None if the stream truncates mid-packet.
        // ⚡ Bolt Optimization: Removed unrolled match blocks and replaced them with
        // an explicitly zero-initialized array followed by a tight `for` loop. For mixed-length
        // streams, this eliminates severe branch prediction overhead caused by evaluating the
        // length and jumping to different array initializations, improving parsing speed.
        // Grouping matching directly on the MT bounds limits memory lookup overhead.
        let count = match w1 >> 28 {
            0x0 | 0x1 | 0x2 | 0x6 | 0x7 => 1,
            0x3..=0x4 | 0x8..=0xA => 2,
            0xB..=0xC => 3,
            0x5 | 0xD..=0xF => 4,
            _ => unreachable!(),
        };

        let mut data = [w1, 0, 0, 0];
        // #[allow(clippy::needless_range_loop)] is used here because doing `for item in data.iter_mut().take(count).skip(1)`
        // as clippy suggests adds iterator abstraction overhead inside this hot parser loop.
        #[allow(clippy::needless_range_loop)]
        for i in 1..count {
            data[i] = self.iter.next()?;
        }

        Some(Ump { data })
    }
}
