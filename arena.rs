use common::*;
use message;

pub struct SegmentReader<'self> {
    messageReader : &'self message::MessageReader<'self>,
    segment : &'self [u8]
}


pub struct SegmentBuilder<'self> {
    messageBuilder : &'self message::MessageBuilder,
    pos : WordCount
}

impl <'self> SegmentBuilder<'self> {
    pub fn allocate(&mut self, amount : WordCount) -> WordCount {
        self.pos += amount;
        fail!("unimplemented")
    }
}


