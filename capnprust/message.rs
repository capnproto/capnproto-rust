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
    segmentBuilders : ~[@mut SegmentBuilder],

    segments : ~[~[u8]]
    // It would probably be nicer if this were a vector of @mut[u8]s
    // and the SegmentBuilders also directly held their respective
    // @mut[u8]s. Only problem is, I don't know how to implement
    // `asReader` in that case.
}

impl MessageBuilder {

    pub fn new(firstSegmentWords : uint, allocationStrategy : AllocationStrategy)
        -> @mut MessageBuilder {
        let result = @mut MessageBuilder {
            nextSize : firstSegmentWords,
            allocationStrategy : allocationStrategy,
            segmentBuilders : ~[],
            segments : ~[]
        };

        let builder =
            @mut SegmentBuilder::new(result, firstSegmentWords);

        result.segments.push(allocate_zeroed_bytes(firstSegmentWords * BYTES_PER_WORD));
        result.segmentBuilders.push(builder);

        result
    }

    pub fn new_default() -> @mut MessageBuilder {
        MessageBuilder::new(SUGGESTED_FIRST_SEGMENT_WORDS, SUGGESTED_ALLOCATION_STRATEGY)
    }

    pub fn allocateSegment(@mut self, minimumSize : WordCount) -> *mut SegmentBuilder {
        let size = std::cmp::max(minimumSize, self.nextSize);
        let segment = allocate_zeroed_bytes(size * BYTES_PER_WORD);
        let result  = @mut SegmentBuilder::new(self, size);
        self.segments.push(segment);
        self.segmentBuilders.push(result);

        match self.allocationStrategy {
            GROW_HEURISTICALLY => { self.nextSize += size; }
            _ => { }
        }

        std::ptr::to_mut_unsafe_ptr(result)
    }

    pub fn getSegmentWithAvailable(@mut self, minimumAvailable : WordCount)
        -> *mut SegmentBuilder {
        if (self.segmentBuilders.last().available() >= minimumAvailable) {
            return std::ptr::to_mut_unsafe_ptr(self.segmentBuilders[self.segments.len() - 1]);
        } else {
            return self.allocateSegment(minimumAvailable);
        }
    }


    pub fn initRoot<T : layout::HasStructSize + layout::FromStructBuilder>(@ mut self) -> T {

        // Rolled in this stuff form getRootSegment.
        let rootSegment = self.segmentBuilders[0];

        let unused_self : Option<T> = None;

        match rootSegment.allocate(WORDS_PER_POINTER) {
            None => {fail!("could not allocate root pointer") }
            Some(location) => {
                //assert!(location == 0,
                //        "First allocated word of new segment was not at offset 0");

                let sb = layout::StructBuilder::initRoot(
                    std::ptr::to_mut_unsafe_ptr(rootSegment),
                    unsafe {std::cast::transmute(location)},
                    layout::HasStructSize::structSize(unused_self));

                return layout::FromStructBuilder::fromStructBuilder(sb);
            }
        }

    }

    pub fn asReader<T>(& self, f : &fn(r : MessageReader) -> T) -> T {
        let mut segments : ~[&[u8]] = ~[];

        for ii in range(0, self.segments.len()) {
            segments.push(self.segments[ii].as_slice());
        }

        f(MessageReader {segments : segments, options : DEFAULT_READER_OPTIONS})
    }

    // break the reference cycle
    pub fn release(&mut self) {
        self.segmentBuilders.clear()
    }

}
