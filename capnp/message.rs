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
    segmentReader0 : SegmentReader<'a>,
    moreSegmentReaders : Option<~[SegmentReader<'a>]>
}

type SegmentId = u32;

impl <'a> MessageReader<'a> {

    #[inline]
    pub unsafe fn get_segment_reader<'b>(&'b self, id : SegmentId) -> *SegmentReader<'b> {
        if (id == 0) {
            return std::ptr::to_unsafe_ptr(&self.segmentReader0);
        } else {
            match self.moreSegmentReaders {
                None => {fail!("no segments!")}
                Some(ref segs) => {
                    segs.unsafe_ref(id as uint - 1)
                }
            }
        }
    }

    #[inline]
    pub fn get_options<'b>(&'b self) -> &'b ReaderOptions {
        return &self.options;
    }
}

impl <'a, 'b> MessageReader<'a> {
    pub fn get_root<T : layout::FromStructReader<'b>>(&'b self) -> T {
        let segment = unsafe { self.get_segment_reader(0) };

        let struct_reader = layout::StructReader::read_root(0, segment,
                                                            self.options.nestingLimit as int);
        let result : T = layout::FromStructReader::from_struct_reader(struct_reader);
        result
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
    allocation_strategy : AllocationStrategy,
    segment_builders : ~[~SegmentBuilder],
    segments : ~[~[Word]]
}

impl MessageBuilder {

    pub fn new(firstSegmentWords : uint, allocationStrategy : AllocationStrategy)
        -> ~MessageBuilder {
        let mut result = ~MessageBuilder {
            nextSize : firstSegmentWords,
            allocation_strategy : allocationStrategy,
            segment_builders : ~[],
            segments : ~[]
        };

        result.segments.push(allocate_zeroed_words(firstSegmentWords));
        let builder =
            ~SegmentBuilder::new(std::ptr::to_mut_unsafe_ptr(result), firstSegmentWords);

        result.segment_builders.push(builder);

        result
    }

    pub fn new_default() -> ~MessageBuilder {
        MessageBuilder::new(SUGGESTED_FIRST_SEGMENT_WORDS, SUGGESTED_ALLOCATION_STRATEGY)
    }

    pub fn allocate_segment(&mut self, minimumSize : WordCount) -> *mut SegmentBuilder {
        let size = std::cmp::max(minimumSize, self.nextSize);
        self.segments.push(allocate_zeroed_words(size));
        self.segment_builders.push(~SegmentBuilder::new(self, size));
        let idx = self.segment_builders.len() - 1;
        let result_ptr = std::ptr::to_mut_unsafe_ptr(self.segment_builders[idx]);

        match self.allocation_strategy {
            GROW_HEURISTICALLY => { self.nextSize += size; }
            _ => { }
        }

        result_ptr
    }

    pub fn get_segment_with_available(&mut self, minimumAvailable : WordCount)
        -> *mut SegmentBuilder {
        if (self.segment_builders.last().available() >= minimumAvailable) {
            return std::ptr::to_mut_unsafe_ptr(self.segment_builders[self.segments.len() - 1]);
        } else {
            return self.allocate_segment(minimumAvailable);
        }
    }


    pub fn init_root<T : layout::HasStructSize + layout::FromStructBuilder>(&mut self) -> T {
        // Rolled in this stuff form getRootSegment.
        let rootSegment = std::ptr::to_mut_unsafe_ptr(self.segment_builders[0]);

        let unused_self : Option<T> = None;

        match self.segment_builders[0].allocate(WORDS_PER_POINTER) {
            None => {fail!("could not allocate root pointer") }
            Some(location) => {
                //assert!(location == 0,
                //        "First allocated word of new segment was not at offset 0");

                let sb = layout::StructBuilder::init_root(
                    rootSegment,
                    unsafe {std::cast::transmute(location)},
                    layout::HasStructSize::struct_size(unused_self));

                return layout::FromStructBuilder::from_struct_builder(sb);
            }
        }
    }

    pub fn as_reader<T>(& self, f : |&MessageReader| -> T) -> T {
        let mut segments : ~[&[Word]] = ~[];

        for ii in range(0, self.segments.len()) {
            segments.push(self.segments[ii].as_slice());
        }

        let mut messageReader =
            MessageReader {segments : segments,
                            segmentReader0 :
                            SegmentReader {  messageReader : std::ptr::null(),
                                             segment: segments[0]
                              },
                            moreSegmentReaders : None,
                            options : DEFAULT_READER_OPTIONS};

        messageReader.segmentReader0.messageReader = std::ptr::to_unsafe_ptr(&messageReader);

        if (segments.len() > 1) {

            let mut moreSegmentReaders = ~[];
            for segment in segments.slice_from(1).iter() {
                let segmentReader =
                    SegmentReader {
                    messageReader : std::ptr::to_unsafe_ptr(&messageReader),
                    segment: *segment
                };
                moreSegmentReaders.push(segmentReader);
            }

            messageReader.moreSegmentReaders = Some(moreSegmentReaders);
        }


        f(&messageReader)
    }

}
