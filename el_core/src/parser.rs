use crate::ump::Ump;

/// Zero-allocation Iterator-based parser that consumes a stream of u32 words
/// and yields correctly aligned `Ump` packets.
pub struct UmpStreamParser<I>
where
    I: Iterator<Item = u32>,
{
    stream: I,
}

impl<I> UmpStreamParser<I>
where
    I: Iterator<Item = u32>,
{
    #[must_use]
    pub fn new(stream: I) -> Self {
        Self { stream }
    }
}

impl<I> Iterator for UmpStreamParser<I>
where
    I: Iterator<Item = u32>,
{
    type Item = Ump;

    fn next(&mut self) -> Option<Self::Item> {
        let w1 = self.stream.next()?;

        use crate::ump::MessageType;
        let count = MessageType::from_u32(w1).word_count();

        // ⚡ Bolt Optimization: Removed unrolled match blocks and replaced them with
        // an explicitly zero-initialized array followed by a tight `for` loop. For mixed-length
        // streams, this eliminates severe branch prediction overhead caused by evaluating the
        // length and jumping to different array initializations, improving parsing speed.
        // We explicitly return None if the stream truncates mid-packet.
        let mut data = [w1, 0, 0, 0];
        for i in 1..count {
            data[i] = self.stream.next()?;
        }

        Some(Ump { data })
    }
}
