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

pub struct MessageReader<'a> {
    segments : &'a [ &'a [Word]],
    options : ReaderOptions,
    arena : ReaderArena
}

type SegmentId = u32;

pub trait MessageReader {
    fn get_segment<'a>(&'a self, id : uint) -> &'a [Word];
    fn arena(&self) -> ReaderArena
    fn get_root<T : layout::FromStructReader<'a>>(&self) -> T {
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

impl <'a> MessageReader<'a> {

    #[inline]
    pub fn get_options<'b>(&'b self) -> &'b ReaderOptions {
        return &self.options;
    }
}

impl <'a> MessageReader<'a> {

    pub fn new<'b>(segments : &'b [&'b [Word]], options : ReaderOptions) -> ~MessageReader<'b> {

        assert!(segments.len() > 0);
        let mut result = ~MessageReader {
            segments : segments,
            arena : ReaderArena {
                segment0 : SegmentReader {
                    arena : Null,
                    ptr : unsafe { segments[0].unsafe_ref(0) },
                    size : segments[0].len()
                },
                more_segments : None
            },
            options : options
        };

        let arena_ptr = ReaderArenaPtr (std::ptr::to_unsafe_ptr(&result.arena));

        result.arena.segment0.arena = arena_ptr;

        if segments.len() > 1 {
            let mut moreSegmentReaders = ~[];
            for segment in segments.slice_from(1).iter() {
                let segmentReader = SegmentReader {
                    arena : arena_ptr,
                    ptr : unsafe { segment.unsafe_ref(0) },
                    size : segment.len()
                };
                moreSegmentReaders.push(segmentReader);
            }
            result.arena.more_segments = Some(moreSegmentReaders);
        }

        result

    }

    pub fn get_root<T : layout::FromStructReader<'a>>(&self) -> T {
        unsafe {
            let segment : *SegmentReader = std::ptr::to_unsafe_ptr(&self.arena.segment0);

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

pub struct MessageBuilder {
    nextSize : uint,
    allocation_strategy : AllocationStrategy,
    arena : ~BuilderArena,
    own_first_segment : bool,
    first_segment : *mut Word,
    more_segments : Option<~[*mut Word]>
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

        match self.more_segments {
            None => {},
            Some(ref mut segs) => {
                for &segment_ptr in segs.iter() {
                    unsafe { std::libc::free(std::cast::transmute(segment_ptr)); }
                }
            }
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
            message : std::ptr::mut_null(),
            segment0 : SegmentBuilder {
                reader : SegmentReader {
                    ptr : first_segment as * Word,
                    size : num_words,
                    arena : Null },
                id : 0,
                pos : first_segment
            },
            more_segments : None };

        let arena_ptr = std::ptr::to_mut_unsafe_ptr(arena);
        arena.segment0.reader.arena = BuilderArenaPtr(arena_ptr);

        let mut result = ~MessageBuilder {
            nextSize : num_words,
            allocation_strategy : allocationStrategy,
            arena : arena,
            own_first_segment: own_first_segment,
            first_segment : first_segment,
            more_segments : None
        };

        (*result.arena).message = std::ptr::to_mut_unsafe_ptr(result);

        cont(result)
    }

    pub fn new_default<T>(cont : |&mut MessageBuilder| -> T) -> T {
        MessageBuilder::new(NumWords(SUGGESTED_FIRST_SEGMENT_WORDS), SUGGESTED_ALLOCATION_STRATEGY, cont)
    }

    pub fn allocate_segment(&mut self, minimumSize : WordCount) -> (*mut Word, WordCount) {
        let size = std::cmp::max(minimumSize, self.nextSize);
        let new_words : *mut Word = unsafe {
            std::cast::transmute(std::libc::calloc(size as std::libc::size_t,
                                                   BYTES_PER_WORD as std::libc::size_t)) };

        match self.more_segments {
            None => self.more_segments = Some(~[new_words]),
            Some(ref mut segs) => segs.push(new_words)
        }

        match self.allocation_strategy {
            GROW_HEURISTICALLY => { self.nextSize += size; }
            _ => { }
        }
        (new_words, size)
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
