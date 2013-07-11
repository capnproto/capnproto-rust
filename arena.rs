use std;
use common::*;
use message;

pub type SegmentId = u32;

pub struct SegmentReader<'self> {
    messageReader : &'self message::MessageReader<'self>,
    segment : &'self [u8]
}


pub struct SegmentBuilder {
    messageBuilder : @mut message::MessageBuilder,
    segment : ~[u8],
    id : SegmentId,
    pos : WordCount
}

impl SegmentBuilder {

    pub fn new(messageBuilder : @mut message::MessageBuilder,
               size : ByteCount) -> SegmentBuilder {
        SegmentBuilder {
            messageBuilder : messageBuilder,
            segment : std::vec::from_elem(size, 0),
            id : messageBuilder.segments.len() as SegmentId,
            pos : 0
        }
    }


    pub fn allocate(&mut self, amount : WordCount) -> Option<WordCount> {
        if (amount > self.segment.len() - self.pos) {
            return None;
        } else {
            let result = self.pos;
            self.pos += amount;
            return Some(result);
        }
    }

    pub fn available(&self) -> WordCount {
        self.segment.len() * BYTES_PER_WORD - self.pos
    }

    #[inline(always)]
    pub fn memset(&mut self, ptr: uint, c: u8, count: uint) {
        unsafe {
            let p = self.segment.unsafe_mut_ref(ptr);
            std::ptr::set_memory(p, c, count)
        }
    }
}


pub struct BuilderArena {
    message : @mut message::MessageBuilder
}
