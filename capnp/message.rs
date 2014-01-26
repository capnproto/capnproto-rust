/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
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


type SegmentId = u32;

pub trait MessageReader {
    fn get_segment<'a>(&'a self, id : uint) -> &'a [Word];
    fn arena<'a>(&'a self) -> &'a ReaderArena;
    fn get_options<'a>(&'a self) -> &'a ReaderOptions;
    fn get_root<'a, T : layout::FromStructReader<'a>>(&'a self) -> T {
        unsafe {
            let segment : *SegmentReader = std::ptr::to_unsafe_ptr(&self.arena().segment0);

            let pointer_reader = layout::PointerReader::get_root::<'a>(
                segment, (*segment).get_start_ptr(), self.get_options().nestingLimit as int);

            let result : T = layout::FromStructReader::from_struct_reader(
                pointer_reader.get_struct::<'a>(std::ptr::null()));

            result
        }

    }
}

pub struct SegmentArrayMessageReader<'a> {
    priv segments : &'a [ &'a [Word]],
    priv options : ReaderOptions,
    priv arena : ~ReaderArena
}


impl <'a> MessageReader for SegmentArrayMessageReader<'a> {
    fn get_segment<'b>(&'b self, id : uint) -> &'b [Word] {
        self.segments[id]
    }

    fn arena<'b>(&'b self) -> &'b ReaderArena {
        &*self.arena
    }

    fn get_options<'b>(&'b self) -> &'b ReaderOptions {
        return &self.options;
    }
}

impl <'a> SegmentArrayMessageReader<'a> {

    pub fn new<'b>(segments : &'b [&'b [Word]], options : ReaderOptions) -> SegmentArrayMessageReader<'b> {
        assert!(segments.len() > 0);
        SegmentArrayMessageReader {
            segments : segments,
            arena : ReaderArena::new(segments),
            options : options
        }
    }
}

pub enum AllocationStrategy {
    FIXED_SIZE,
    GROW_HEURISTICALLY
}

pub static SUGGESTED_FIRST_SEGMENT_WORDS : uint = 1024;
pub static SUGGESTED_ALLOCATION_STRATEGY : AllocationStrategy = GROW_HEURISTICALLY;

pub struct MessageBuilder {
    arena : ~BuilderArena,
    own_first_segment : bool,
    first_segment : *mut Word,
}

impl Drop for MessageBuilder {
    fn drop(&mut self) {
        if self.own_first_segment {
            unsafe { std::libc::free(std::cast::transmute(self.first_segment)) }
        } else {
            self.get_segments_for_output(|segments| {
                    unsafe {
                        std::ptr::zero_memory(self.first_segment, segments[0].len());
                    }
                });
        }
    }
}

pub enum FirstSegment<'a> {
    NumWords(uint),
    ZeroedWords(&'a mut [Word])
}

impl MessageBuilder {

    // TODO: maybe when Rust issue #5121 is fixed we can safely get away with not passing
    //  a closure here.
    pub fn new<'a, T>(first_segment_arg : FirstSegment<'a>,
                      allocationStrategy : AllocationStrategy,
                      cont : |&mut MessageBuilder| -> T) -> T {

        let (first_segment, num_words, own_first_segment) : (*mut Word, uint, bool) = unsafe {
            match first_segment_arg {
                NumWords(n) =>
                    (std::cast::transmute(
                        std::libc::calloc(n as std::libc::size_t,
                                          BYTES_PER_WORD as std::libc::size_t)),
                     n, true),
                ZeroedWords(w) => (w.as_mut_ptr(), w.len(), false)
            }};

        let mut arena = ~BuilderArena::<'a> {
            segment0 : SegmentBuilder {
                reader : SegmentReader {
                    ptr : first_segment as * Word,
                    size : num_words,
                    arena : Null },
                id : 0,
                pos : first_segment
            },
            more_segments : None,
            allocation_strategy : allocationStrategy,
            owned_memory : None,
            nextSize : num_words,
        };

        let arena_ptr = std::ptr::to_mut_unsafe_ptr(arena);
        arena.segment0.reader.arena = BuilderArenaPtr(arena_ptr);

        let mut result = ~MessageBuilder {
            arena : arena,
            own_first_segment: own_first_segment,
            first_segment : first_segment,
        };

        cont(result)
    }

    pub fn new_default<T>(cont : |&mut MessageBuilder| -> T) -> T {
        MessageBuilder::new(NumWords(SUGGESTED_FIRST_SEGMENT_WORDS), SUGGESTED_ALLOCATION_STRATEGY, cont)
    }

    pub fn get_segments_for_output<T>(&self, cont : |&[&[Word]]| -> T) -> T {
        self.arena.get_segments_for_output(cont)
    }
}

impl MessageBuilder {
    // Note: This type signature ought to prevent a MessageBuilder
    // from being initted twice simultaneously. It currently does not
    // fulfill that goal, perhaps due to Rust issue #5121.
    pub fn init_root<'a, T : layout::HasStructSize + layout::FromStructBuilder<'a>>(&mut self) -> T {
        // Rolled in this stuff form getRootSegment.
        let rootSegment = std::ptr::to_mut_unsafe_ptr(&mut self.arena.segment0);

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
