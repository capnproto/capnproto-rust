use common::*;
use message;

pub struct SegmentReader<'self> {
    messageReader : &'self message::MessageReader<'self>,
    segment : &'self [u8]
}


pub struct SegmentBuilder<'self> {
    messageBuilder : &'self message::MessageBuilder<'self>,
    segment : ~[u8],
    pos : WordCount
}

impl <'self> SegmentBuilder<'self> {
    pub fn allocate(&mut self, amount : WordCount) -> Option<WordCount> {
        if (amount > self.segment.len() - self.pos) {
            return None;
        } else {
            let result = self.pos;
            self.pos += amount;
            return Some(result);
        }
    }
}


