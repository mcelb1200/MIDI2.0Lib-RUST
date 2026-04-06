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

        // Fast-path MessageType extraction without branching or enum conversion overhead
        // Grouping matching directly on the MT bounds limits memory lookup overhead
        // and enables the compiler to generate an optimized branch table.
        // We explicitly return None if the stream truncates mid-packet.
        match w1 >> 28 {
            0x0..=0x2 | 0x6..=0x7 => Some(Ump {
                data: [w1, 0, 0, 0],
            }),
            0x3..=0x4 | 0x8..=0xA => Some(Ump {
                data: [w1, self.stream.next()?, 0, 0],
            }),
            0xB..=0xC => Some(Ump {
                data: [w1, self.stream.next()?, self.stream.next()?, 0],
            }),
            0x5 | 0xD..=0xF => Some(Ump {
                data: [
                    w1,
                    self.stream.next()?,
                    self.stream.next()?,
                    self.stream.next()?,
                ],
            }),
            _ => None,
        }
    }
}
