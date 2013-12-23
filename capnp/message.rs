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
    arena : ReaderArena<'a>
}

type SegmentId = u32;

impl <'a> MessageReader<'a> {

    #[inline]
    pub fn get_options<'b>(&'b self) -> &'b ReaderOptions {
        return &self.options;
    }
}

impl <'a> MessageReader<'a> {
    pub fn get_root<T : layout::FromStructReader<'a>>(&self) -> T {
        unsafe {
            let segment : *SegmentReader<'a> = std::ptr::to_unsafe_ptr(&self.arena.segment0);

            let pointer_reader = layout::PointerReader::get_root::<'a>(
                segment, (*segment).get_start_ptr(), self.options.nestingLimit as int);

            let result : T = layout::FromStructReader::from_struct_reader(
                pointer_reader.get_struct::<'a>(std::ptr::null()));

            result
        }
    }
}

pub enum AllocationStrategy {
    FIXED_SIZE,
    GROW_HEURISTICALLY
}

pub static SUGGESTED_FIRST_SEGMENT_WORDS : uint = 1024;
pub static SUGGESTED_ALLOCATION_STRATEGY : AllocationStrategy = GROW_HEURISTICALLY;

pub struct MessageBuilder<'a> {
    nextSize : uint,
    allocation_strategy : AllocationStrategy,
    arena : ~BuilderArena<'a>,
    segments : ~[~[Word]]
}

impl <'a>MessageBuilder<'a> {

    // TODO: maybe when Rust issue #5121 is fixed we can safely get away with not passing
    //  a closure here.
    pub fn new<T>(firstSegmentWords : uint,
                  allocationStrategy : AllocationStrategy,
                  cont : |&mut MessageBuilder| -> T) -> T {

        let mut segments = ~[];
        segments.push(allocate_zeroed_words(firstSegmentWords));
        let mut arena = ~BuilderArena {
            message : std::ptr::mut_null(),
            segment0 : SegmentBuilder {
                reader : SegmentReader {
                    ptr : unsafe { segments[0].unsafe_ref(0) },
                    size : segments[0].len(),
                    arena : Null },
                id : 0,
                pos : unsafe { segments[0].unsafe_mut_ref(0) }
            },
            more_segments : None };

        let arena_ptr = std::ptr::to_mut_unsafe_ptr(arena);
        arena.segment0.reader.arena = BuilderArenaPtr(arena_ptr);

        let mut result = ~MessageBuilder {
            nextSize : firstSegmentWords,
            allocation_strategy : allocationStrategy,
            segments : segments,
            arena : arena
        };

        cont(result)
    }

    pub fn new_default<T>(cont : |&mut MessageBuilder| -> T) -> T {
        MessageBuilder::new(SUGGESTED_FIRST_SEGMENT_WORDS, SUGGESTED_ALLOCATION_STRATEGY, cont)
    }

    pub fn allocate_segment(&mut self, minimumSize : WordCount) -> (*mut SegmentBuilder<'a>, *mut Word) {
        let size = std::cmp::max(minimumSize, self.nextSize);
        let mut id = self.segments.len() as u32;
        let mut new_words = allocate_zeroed_words(size);
        let ptr = unsafe { new_words.unsafe_mut_ref(0) };
        self.segments.push(new_words);
        let mut new_builder = ~SegmentBuilder::new(std::ptr::to_mut_unsafe_ptr(self.arena), id, ptr, size);
        let result_ptr = std::ptr::to_mut_unsafe_ptr(&mut new_builder);
        match self.arena.more_segments {
            None =>
                self.arena.more_segments = Some(~[new_builder]),
            Some(ref mut msegs) => {
                msegs.push(new_builder);
            }
        }

        match self.allocation_strategy {
            GROW_HEURISTICALLY => { self.nextSize += size; }
            _ => { }
        }
        fail!()
        //result_ptr
    }

    pub fn get_segments_for_output<T>(&self, cont : |&[&[Word]]| -> T) -> T {
        self.arena.get_segments_for_output(cont)
    }
}

impl <'a>MessageBuilder<'a> {
    // Note: This type signature ought to prevent a MessageBuilder
    // from being initted twice simultaneously. It currently does not
    // fulfill that goal, perhaps due to Rust issue #5121.
    pub fn init_root<T : layout::HasStructSize + layout::FromStructBuilder<'a>>(&'a mut self) -> T {
        // Rolled in this stuff form getRootSegment.
        let rootSegment = unsafe { std::ptr::to_mut_unsafe_ptr(&mut self.arena.segment0) };

        match self.arena.segment0.allocate(WORDS_PER_POINTER) {
            None => {fail!("could not allocate root pointer") }
            Some(location) => {
                //assert!(location == 0,
                //        "First allocated word of new segment was not at offset 0");

                let pb = layout::PointerBuilder::get_root(rootSegment, location);

                return layout::FromStructBuilder::from_struct_builder(
                    pb.init_struct(layout::HasStructSize::struct_size(None::<T>)));
            }
        }
    }

}
