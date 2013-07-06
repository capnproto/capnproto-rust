use std;
use common::*;
use arena::*;
use layout;

pub struct ReaderOptions {
    traversalLimitInWords : u64,
    nestingLimit : uint
}

pub static DEFAULT_READER_OPTIONS : ReaderOptions =
    ReaderOptions { traversalLimitInWords : 8 * 1024 * 1024, nestingLimit : 64 };

pub struct MessageReader<'self> {
    segments : &'self [ &'self [u8]],
    options : ReaderOptions,
//    arena : ReaderArena<'self>
}

type SegmentId = u32;

impl <'self> MessageReader<'self> {

    #[inline(always)]
    pub fn getSegment(&self, id : uint) -> &'self [u8] {
        self.segments[id]
    }

    #[inline(always)]
    pub fn getSegmentReader<'a>(&'a self, id : SegmentId) -> SegmentReader<'a> {
        SegmentReader { messageReader : self, segment : self.getSegment(id as uint) }
    }

    #[inline(always)]
    pub fn getOptions<'a>(&'a self) -> &'a ReaderOptions {
        return &self.options;
    }

    pub fn getRoot<'a>(&'a mut self) -> layout::StructReader<'a> {
        let segment = self.getSegmentReader(0);

        return layout::StructReader::readRoot(0, segment,
                                              self.options.nestingLimit as int);
    }

}

pub enum AllocationStrategy {
    FIXED_SIZE,
    GROW_HEURISTICALLY
}

pub static SUGGESTED_FIRST_SEGMENT_WORDS : uint = 1024;
pub static SUGGESTED_ALLOCATION_STRATEGY : AllocationStrategy = GROW_HEURISTICALLY;

pub struct MallocMessageBuilder {
    nextSize : uint,
    allocationStrategy : AllocationStrategy,
    firstSegment : ~[u8],
    moreSegments : Option<~[~[u8]]>
}


impl MallocMessageBuilder {
    pub fn new(firstSegment : ~[u8], allocationStrategy : AllocationStrategy)
        -> MallocMessageBuilder {
        MallocMessageBuilder {
            nextSize : firstSegment.len(),
            allocationStrategy : allocationStrategy,
            firstSegment : firstSegment,
            moreSegments : None
        }
    }

    pub fn new_default() -> MallocMessageBuilder {
        MallocMessageBuilder {
            nextSize : SUGGESTED_FIRST_SEGMENT_WORDS,
            allocationStrategy : SUGGESTED_ALLOCATION_STRATEGY,
            firstSegment : std::vec::from_elem(SUGGESTED_FIRST_SEGMENT_WORDS * BYTES_PER_WORD,
                                               0),
            moreSegments : None
        }
    }

    pub fn allocateSegment<'a>(&'a self, minimumSize : uint) -> &'a [u8] {
        fail!()
    }

}