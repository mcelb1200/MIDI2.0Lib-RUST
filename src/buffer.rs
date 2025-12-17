use crate::ump::{Ump, MessageType};

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
    pub fn new(iter: I) -> Self {
        Self { iter }
    }
}

impl<I> Iterator for UmpStreamParser<I>
where
    I: Iterator<Item = u32>,
{
    type Item = Ump;

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
