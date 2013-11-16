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

pub struct MessageReader<'a> {
    segments : &'a [ &'a [Word]],
    options : ReaderOptions,
    segmentReaders : ~[SegmentReader<'a>]
}

type SegmentId = u32;

impl <'self> MessageReader<'self> {

    #[inline]
    pub fn getSegment(&self, id : uint) -> &'self [Word] {
        self.segments[id]
    }

    #[inline]
    pub unsafe fn getSegmentReader<'a>(&'a self, id : SegmentId) -> *SegmentReader<'a> {
        std::ptr::to_unsafe_ptr(&self.segmentReaders[id])
    }

    #[inline]
    pub fn getOptions<'a>(&'a self) -> &'a ReaderOptions {
        return &self.options;
    }

    pub fn getRoot<'a>(&'a self) -> layout::StructReader<'a> {
        let segment = unsafe { self.getSegmentReader(0) };

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
    segmentBuilders : ~[SegmentBuilder],
    segments : ~[~[Word]]
}

impl MessageBuilder {

    pub fn new(firstSegmentWords : uint, allocationStrategy : AllocationStrategy)
        -> ~MessageBuilder {
        let mut result = ~MessageBuilder {
            nextSize : firstSegmentWords,
            allocationStrategy : allocationStrategy,
            segmentBuilders : ~[],
            segments : ~[]
        };

        result.segments.push(allocate_zeroed_words(firstSegmentWords));
        let builder =
            SegmentBuilder::new(std::ptr::to_mut_unsafe_ptr(result), firstSegmentWords);

        result.segmentBuilders.push(builder);

        result
    }

    pub fn new_default() -> ~MessageBuilder {
        MessageBuilder::new(SUGGESTED_FIRST_SEGMENT_WORDS, SUGGESTED_ALLOCATION_STRATEGY)
    }

    pub fn allocateSegment(&mut self, minimumSize : WordCount) -> *mut SegmentBuilder {
        let size = std::cmp::max(minimumSize, self.nextSize);
        self.segments.push(allocate_zeroed_words(size));
        self.segmentBuilders.push(SegmentBuilder::new(self, size));
        let idx = self.segmentBuilders.len() - 1;
        let result_ptr = unsafe {
            self.segmentBuilders.unsafe_mut_ref(idx) };

        match self.allocationStrategy {
            GROW_HEURISTICALLY => { self.nextSize += size; }
            _ => { }
        }

        result_ptr
    }

    pub fn getSegmentWithAvailable(&mut self, minimumAvailable : WordCount)
        -> *mut SegmentBuilder {
        if (self.segmentBuilders.last().available() >= minimumAvailable) {
            return std::ptr::to_mut_unsafe_ptr(&mut self.segmentBuilders[self.segments.len() - 1]);
        } else {
            return self.allocateSegment(minimumAvailable);
        }
    }


    pub fn initRoot<T : layout::HasStructSize + layout::FromStructBuilder>(&mut self) -> T {
        // Rolled in this stuff form getRootSegment.
        let rootSegment = std::ptr::to_mut_unsafe_ptr(&mut self.segmentBuilders[0]);

        let unused_self : Option<T> = None;

        match self.segmentBuilders[0].allocate(WORDS_PER_POINTER) {
            None => {fail!("could not allocate root pointer") }
            Some(location) => {
                //assert!(location == 0,
                //        "First allocated word of new segment was not at offset 0");

                let sb = layout::StructBuilder::initRoot(
                    rootSegment,
                    unsafe {std::cast::transmute(location)},
                    layout::HasStructSize::structSize(unused_self));

                return layout::FromStructBuilder::fromStructBuilder(sb);
            }
        }
    }

    pub fn asReader<T>(& self, f : &fn(r : &MessageReader) -> T) -> T {
        let mut segments : ~[&[Word]] = ~[];

        for ii in range(0, self.segments.len()) {
            segments.push(self.segments[ii].as_slice());
        }

        let mut messageReader = ~MessageReader {segments : segments,
                                                segmentReaders : ~[],
                                                options : DEFAULT_READER_OPTIONS};

        for segment in segments.iter() {
            let segmentReader =
                SegmentReader {
                    messageReader : std::ptr::to_unsafe_ptr(messageReader),
                    segment: *segment
                };
            messageReader.segmentReaders.push(segmentReader);
        }


        f(messageReader)
    }

}
