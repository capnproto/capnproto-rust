/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

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

    #[inline]
    pub fn getSegment(&self, id : uint) -> &'self [u8] {
        self.segments[id]
    }

    #[inline]
    pub fn getSegmentReader<'a>(&'a self, id : SegmentId) -> SegmentReader<'a> {
        SegmentReader { messageReader : self, segment : self.getSegment(id as uint) }
    }

    #[inline]
    pub fn getOptions<'a>(&'a self) -> &'a ReaderOptions {
        return &self.options;
    }

    pub fn getRoot<'a>(&'a self) -> layout::StructReader<'a> {
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

pub struct MessageBuilder {
    nextSize : uint,
    allocationStrategy : AllocationStrategy,
    segments : ~[@mut SegmentBuilder]
}


impl MessageBuilder {

    pub fn new(firstSegmentWords : uint, allocationStrategy : AllocationStrategy)
        -> @mut MessageBuilder {
        let result = @mut MessageBuilder {
            nextSize : firstSegmentWords,
            allocationStrategy : allocationStrategy,
            segments : ~[]
        };
        let builder =
            @mut SegmentBuilder::new(result, firstSegmentWords * BYTES_PER_WORD);
        result.segments.push(builder);

        result
    }

    pub fn new_default() -> @mut MessageBuilder {
        MessageBuilder::new(SUGGESTED_FIRST_SEGMENT_WORDS, SUGGESTED_ALLOCATION_STRATEGY)
    }

    pub fn allocateSegment(@mut self, minimumSize : uint) -> @mut SegmentBuilder {
        let size = std::cmp::max(minimumSize, self.nextSize);
        let result  = @mut SegmentBuilder::new(self, size);
        self.segments.push(result);

        match self.allocationStrategy {
            GROW_HEURISTICALLY => { self.nextSize += size; }
            _ => { }
        }

        result
    }

    pub fn getSegmentWithAvailable(@mut self, minimumAvailable : WordCount)
        -> @mut SegmentBuilder {
        if (self.segments.last().available() >= minimumAvailable) {

            return self.segments[self.segments.len()];

        } else {

            return self.allocateSegment(minimumAvailable);

        }
    }

    pub fn initRoot<T : layout::HasStructSize + layout::FromStructBuilder>(&self) -> T {

        // Rolled in this stuff form getRootSegment.
        let rootSegment = self.segments[0];
        match rootSegment.allocate(WORDS_PER_POINTER) {
            None => {fail!("could not allocate root pointer") }
            Some(location) => {
                assert!(location == 0,
                        "First allocated word of new segment was not at offset 0");

                let sb = layout::StructBuilder::initRoot(rootSegment, location,
                                                         layout::HasStructSize::structSize::<T>());
                return layout::FromStructBuilder::fromStructBuilder::<T>(sb);
            }
        }

    }
}
