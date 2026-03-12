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
        let mt_val = (w1 >> 28) as usize;

        // Safety: MT is strictly 4 bits (0-15).
        const WORD_COUNTS: [u8; 16] = [
            1, // 0x0 Utility
            1, // 0x1 System
            1, // 0x2 MIDI 1.0 Voice
            2, // 0x3 Data 64-bit
            2, // 0x4 MIDI 2.0 Voice
            4, // 0x5 Data 128-bit
            1, // 0x6 Reserved
            1, // 0x7 Reserved
            2, // 0x8 Reserved
            2, // 0x9 Reserved
            2, // 0xA Reserved
            3, // 0xB Reserved
            3, // 0xC Reserved
            4, // 0xD Reserved
            4, // 0xE Reserved
            4, // 0xF Stream
        ];

        let count = WORD_COUNTS[mt_val];

        // Explicit unrolled match to avoid loop overhead and cleanly handle truncation.
        // We explicitly return None if the stream truncates mid-packet, preventing data corruption.
        match count {
            1 => Some(Ump {
                data: [w1, 0, 0, 0],
            }),
            2 => {
                let w2 = *self.stream.next()?;
                Some(Ump {
                    data: [w1, w2, 0, 0],
                })
            }
            3 => {
                let w2 = *self.stream.next()?;
                let w3 = *self.stream.next()?;
                Some(Ump {
                    data: [w1, w2, w3, 0],
                })
            }
            4 => {
                let w2 = *self.stream.next()?;
                let w3 = *self.stream.next()?;
                let w4 = *self.stream.next()?;
                Some(Ump {
                    data: [w1, w2, w3, w4],
                })
            }
            _ => None, // Unreachable due to array bounds, but required by match
        }
    }
}
