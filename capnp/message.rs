/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use std::vec::Vec;
use any::AnyPointer;
use capability::ClientHook;
use common::*;
use arena::{BuilderArena, ReaderArena, SegmentBuilder, SegmentReader, NumWords, ZeroedWords};
use layout;
use layout::{FromStructBuilder, HasStructSize};

pub struct ReaderOptions {
    pub traversal_limit_in_words : u64,
    pub nesting_limit : int,

    // If true, malformed messages trigger task failure.
    // If false, malformed messages fall back to default values.
    pub fail_fast : bool,
}

pub static DefaultReaderOptions : ReaderOptions =
    ReaderOptions { traversal_limit_in_words : 8 * 1024 * 1024, nesting_limit : 64,
                    fail_fast : true };

impl ReaderOptions {
    pub fn new() -> ReaderOptions { DefaultReaderOptions }

    pub fn nesting_limit<'a>(&'a mut self, value : int) -> &'a mut ReaderOptions {
        self.nesting_limit = value;
        return self;
    }

    pub fn traversal_limit_in_words<'a>(&'a mut self, value : u64) -> &'a mut ReaderOptions {
        self.traversal_limit_in_words = value;
        return self;
    }

    pub fn fail_fast<'a>(&'a mut self, value : bool) -> &'a mut ReaderOptions {
        self.fail_fast = value;
        return self;
    }
}


type SegmentId = u32;

pub trait MessageReader {
    fn get_segment<'a>(&'a self, id : uint) -> &'a [Word];
    fn arena<'a>(&'a self) -> &'a ReaderArena;
    fn mut_arena<'a>(&'a mut self) -> &'a mut ReaderArena;
    fn get_options<'a>(&'a self) -> &'a ReaderOptions;

    fn get_root_internal<'a> (&'a self) -> AnyPointer::Reader<'a> {
        unsafe {
            let segment : *const SegmentReader = &self.arena().segment0;

            let pointer_reader = layout::PointerReader::get_root(
                segment, (*segment).get_start_ptr(), self.get_options().nesting_limit as int);

            AnyPointer::Reader::new(pointer_reader)
        }
    }

    fn get_root<'a, T : layout::FromStructReader<'a>>(&'a self) -> T {
        self.get_root_internal().get_as_struct()
    }

    fn init_cap_table(&mut self, cap_table : Vec<Option<Box<ClientHook+Send>>>) {
        self.mut_arena().init_cap_table(cap_table);
    }
}

pub struct SegmentArrayMessageReader<'a> {
    segments : &'a [ &'a [Word]],
    options : ReaderOptions,
    arena : Box<ReaderArena>
}


impl <'a> MessageReader for SegmentArrayMessageReader<'a> {
    fn get_segment<'b>(&'b self, id : uint) -> &'b [Word] {
        self.segments[id]
    }

    fn arena<'b>(&'b self) -> &'b ReaderArena { &*self.arena }
    fn mut_arena<'b>(&'b mut self) -> &'b mut ReaderArena { &mut *self.arena }

    fn get_options<'b>(&'b self) -> &'b ReaderOptions {
        return &self.options;
    }
}

impl <'a> SegmentArrayMessageReader<'a> {

    pub fn new<'b>(segments : &'b [&'b [Word]], options : ReaderOptions) -> SegmentArrayMessageReader<'b> {
        assert!(segments.len() > 0);
        SegmentArrayMessageReader {
            segments : segments,
            arena : ReaderArena::new(segments, options),
            options : options
        }
    }
}

pub enum AllocationStrategy {
    FixedSize,
    GrowHeuristically
}

pub static SUGGESTED_FIRST_SEGMENT_WORDS : uint = 1024;
pub static SUGGESTED_ALLOCATION_STRATEGY : AllocationStrategy = GrowHeuristically;

pub struct BuilderOptions {
    pub first_segment_words : uint,
    pub allocation_strategy : AllocationStrategy,

    // If true, malformed messages trigger task failure.
    // If false, malformed messages fall back to default values.
    pub fail_fast : bool,
}

impl BuilderOptions {
    pub fn new() -> BuilderOptions {
        BuilderOptions {first_segment_words : SUGGESTED_FIRST_SEGMENT_WORDS,
                        allocation_strategy : GrowHeuristically,
                        fail_fast : true }
    }

    pub fn first_segment_words<'a>(&'a mut self, value : uint) -> &'a mut BuilderOptions {
        self.first_segment_words = value;
        return self;
    }

    pub fn allocation_strategy<'a>(&'a mut self, value : AllocationStrategy) -> &'a mut BuilderOptions {
        self.allocation_strategy = value;
        return self;
    }

    pub fn fail_fast<'a>(&'a mut self, value : bool) -> &'a mut BuilderOptions {
        self.fail_fast = value;
        return self;
    }
}


pub trait MessageBuilder {
    fn mut_arena<'a>(&'a mut self) -> &'a mut BuilderArena;
    fn arena<'a>(&'a self) -> &'a BuilderArena;


    // XXX is there a way to make this private?
    fn get_root_internal<'a>(&mut self) -> AnyPointer::Builder<'a> {
        let root_segment = &mut self.mut_arena().segment0 as *mut SegmentBuilder;

        if self.arena().segment0.current_size() == 0 {
            match self.mut_arena().segment0.allocate(WORDS_PER_POINTER) {
                None => {fail!("could not allocate root pointer") }
                Some(location) => {
                    assert!(location == self.arena().segment0.get_ptr_unchecked(0),
                            "First allocated word of new segment was not at offset 0");

                    AnyPointer::Builder::new(layout::PointerBuilder::get_root(root_segment, location))

                }
            }
        } else {
            AnyPointer::Builder::new(
                layout::PointerBuilder::get_root(root_segment,
                                                 self.arena().segment0.get_ptr_unchecked(0)))
        }

    }

    fn init_root<'a, T : FromStructBuilder<'a> + HasStructSize>(&'a mut self) -> T {
        self.get_root_internal().init_as_struct()
    }

    fn get_root<'a, T : FromStructBuilder<'a> + HasStructSize>(&'a mut self) -> T {
        self.get_root_internal().get_as_struct()
    }

    fn set_root<'a, T : layout::ToStructReader<'a>>(&'a mut self, value : &T) {
        self.get_root_internal().set_as_struct(value);
    }

    fn get_segments_for_output<T>(&self, cont : |&[&[Word]]| -> T) -> T {
        self.arena().get_segments_for_output(cont)
    }

    fn get_cap_table<'a>(&'a self) -> &'a [Option<Box<ClientHook+Send>>] {
        self.arena().get_cap_table()
    }
}

pub struct MallocMessageBuilder {
    arena : Box<BuilderArena>,
}

impl Drop for MallocMessageBuilder {
    fn drop(&mut self) { }
}

impl MallocMessageBuilder {

    pub fn new(options : BuilderOptions) -> MallocMessageBuilder {
        let arena = BuilderArena::new(options.allocation_strategy,
                                      NumWords(options.first_segment_words),
                                      options.fail_fast);

        MallocMessageBuilder { arena : arena }
    }

    pub fn new_default() -> MallocMessageBuilder {
        MallocMessageBuilder::new(BuilderOptions::new())
    }

}

impl MessageBuilder for MallocMessageBuilder {
    fn mut_arena<'a>(&'a mut self) -> &'a mut BuilderArena {
        &mut *self.arena
    }
    fn arena<'a>(&'a self) -> &'a BuilderArena {
        & *self.arena
    }
}


pub struct ScratchSpaceMallocMessageBuilder<'a> {
    arena : Box<BuilderArena>,
    scratch_space : &'a mut [Word],
}


// TODO: figure out why rust thinks this is unsafe.
#[unsafe_destructor]
impl <'a> Drop for ScratchSpaceMallocMessageBuilder<'a> {
    fn drop(&mut self) {
        let ptr = self.scratch_space.as_mut_ptr();
        self.get_segments_for_output(|segments| {
                unsafe {
                    std::ptr::zero_memory(ptr, segments[0].len());
                }
            });
    }
}


impl <'a> ScratchSpaceMallocMessageBuilder<'a> {

    pub fn new<'b>(scratch_space : &'b mut [Word], options : BuilderOptions)
               -> ScratchSpaceMallocMessageBuilder<'b> {
        let arena = BuilderArena::new(options.allocation_strategy, ZeroedWords(scratch_space),
                                      options.fail_fast);

        ScratchSpaceMallocMessageBuilder { arena : arena, scratch_space : scratch_space }
    }

    pub fn new_default<'b>(scratch_space : &'b mut [Word]) -> ScratchSpaceMallocMessageBuilder<'b> {
        ScratchSpaceMallocMessageBuilder::new(scratch_space, BuilderOptions::new())
    }

}

impl <'a> MessageBuilder for ScratchSpaceMallocMessageBuilder<'a> {
    fn mut_arena<'a>(&'a mut self) -> &'a mut BuilderArena {
        &mut *self.arena
    }
    fn arena<'a>(&'a self) -> &'a BuilderArena {
        & *self.arena
    }
}
