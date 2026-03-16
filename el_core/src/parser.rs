use crate::ump::Ump;

/// Zero-allocation Iterator-based parser that consumes a stream of u32 words
/// and yields correctly aligned `Ump` packets.
pub struct UmpStreamParser<'a> {
    stream: core::slice::Iter<'a, u32>,
}

impl<'a> UmpStreamParser<'a> {
    #[must_use]
    pub fn new(data: &'a [u32]) -> Self {
        Self {
            stream: data.iter(),
        }
    }
}

impl<'a> Iterator for UmpStreamParser<'a> {
    type Item = Ump;

    fn next(&mut self) -> Option<Self::Item> {
        let w1 = *self.stream.next()?;

        // Fast-path MessageType extraction without branching or enum conversion overhead
        // Grouping matching directly on the MT bounds limits memory lookup overhead
        // and enables the compiler to generate an optimized branch table.
        // We explicitly return None if the stream truncates mid-packet.
        match (w1 >> 28) & 0xF {
            0x0 | 0x1 | 0x2 | 0x6 | 0x7 => Some(Ump {
                data: [w1, 0, 0, 0],
            }),
            0x3 | 0x4 | 0x8 | 0x9 | 0xA => {
                let w2 = *self.stream.next()?;
                Some(Ump {
                    data: [w1, w2, 0, 0],
                })
            }
            0xB | 0xC => {
                let w2 = *self.stream.next()?;
                let w3 = *self.stream.next()?;
                Some(Ump {
                    data: [w1, w2, w3, 0],
                })
            }
            0x5 | 0xD | 0xE | 0xF => {
                let w2 = *self.stream.next()?;
                let w3 = *self.stream.next()?;
                let w4 = *self.stream.next()?;
                Some(Ump {
                    data: [w1, w2, w3, w4],
                })
            }
            _ => None, // Unreachable due to 4-bit bitmask constraint
        }
    }
}
