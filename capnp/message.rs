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
use arena::*;
use layout;
use layout::{FromStructBuilder, HasStructSize};

pub struct ReaderOptions {
    traversalLimitInWords : u64,
    nestingLimit : uint
}

pub static DefaultReaderOptions : ReaderOptions =
    ReaderOptions { traversalLimitInWords : 8 * 1024 * 1024, nestingLimit : 64 };


type SegmentId = u32;

pub trait MessageReader {
    fn get_segment<'a>(&'a self, id : uint) -> &'a [Word];
    fn arena<'a>(&'a self) -> &'a ReaderArena;
    fn mut_arena<'a>(&'a mut self) -> &'a mut ReaderArena;
    fn get_options<'a>(&'a self) -> &'a ReaderOptions;

    // XXX lifetime should not be 'static. See rustc issues #12856 #12857.
    fn get_root<T : layout::FromStructReader<'static>>(&self) -> T {
        unsafe {
            let segment : *SegmentReader = &self.arena().segment0;

            let pointer_reader = layout::PointerReader::get_root(
                segment, (*segment).get_start_ptr(), self.get_options().nestingLimit as int);

            let result : T = layout::FromStructReader::new(
                pointer_reader.get_struct(std::ptr::null()));

            result
        }
    }

    fn init_cap_table(&mut self, cap_table : Vec<Option<~ClientHook>>) {
        self.mut_arena().init_cap_table(cap_table);
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
            arena : ReaderArena::new(segments),
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

pub trait MessageBuilder {
    fn mut_arena<'a>(&'a mut self) -> &'a mut BuilderArena;
    fn arena<'a>(&'a self) -> &'a BuilderArena;



    // XXX is there a way to make this private?
    fn get_root_internal<'a>(&mut self) -> AnyPointer::Builder<'a> {
        let rootSegment = &mut self.mut_arena().segment0 as *mut SegmentBuilder;

        if self.arena().segment0.current_size() == 0 {
            match self.mut_arena().segment0.allocate(WORDS_PER_POINTER) {
                None => {fail!("could not allocate root pointer") }
                Some(location) => {
                    assert!(location == self.arena().segment0.get_ptr_unchecked(0),
                            "First allocated word of new segment was not at offset 0");

                    AnyPointer::Builder::new(layout::PointerBuilder::get_root(rootSegment, location))

                }
            }
        } else {
            AnyPointer::Builder::new(
                layout::PointerBuilder::get_root(rootSegment,
                                                 self.arena().segment0.get_ptr_unchecked(0)))
        }

    }

    // XXX lifetime should not be 'static.
    fn init_root<T : FromStructBuilder<'static> + HasStructSize>(&mut self) -> T {
        self.get_root_internal().init_as_struct()
    }

    // XXX lifetime should not be 'static
    fn get_root<T : FromStructBuilder<'static> + HasStructSize>(&mut self) -> T {
        self.get_root_internal().get_as_struct()
    }

    // XXX lifetime should not be 'static
    fn set_root<T : layout::ToStructReader<'static> + layout::ToStructReader<'static>>(&mut self, value : &T) {
        use layout::ToStructReader;
        self.get_root_internal().set_as_struct(value);
    }

    fn get_segments_for_output<T>(&self, cont : |&[&[Word]]| -> T) -> T {
        self.arena().get_segments_for_output(cont)
    }

    fn get_cap_table<'a>(&'a self) -> &'a [Option<~ClientHook>] {
        self.arena().get_cap_table()
    }
}

pub struct MallocMessageBuilder {
    priv arena : ~BuilderArena,
}

impl Drop for MallocMessageBuilder {
    fn drop(&mut self) { }
}

impl MallocMessageBuilder {

    pub fn new(first_segment_size : uint, allocationStrategy : AllocationStrategy) -> MallocMessageBuilder {
        let arena = BuilderArena::new(allocationStrategy, NumWords(first_segment_size));

        MallocMessageBuilder { arena : arena }
    }

    pub fn new_default() -> MallocMessageBuilder {
        MallocMessageBuilder::new(SUGGESTED_FIRST_SEGMENT_WORDS, SUGGESTED_ALLOCATION_STRATEGY)
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
    priv arena : ~BuilderArena,
    priv scratch_space : &'a mut [Word],
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

    pub fn new<'b>(scratch_space : &'b mut [Word], allocationStrategy : AllocationStrategy)
               -> ScratchSpaceMallocMessageBuilder<'b> {
        let arena = BuilderArena::new(allocationStrategy, ZeroedWords(scratch_space));

        ScratchSpaceMallocMessageBuilder { arena : arena, scratch_space : scratch_space }
    }

    pub fn new_default<'b>(scratch_space : &'b mut [Word]) -> ScratchSpaceMallocMessageBuilder<'b> {
        ScratchSpaceMallocMessageBuilder::new(scratch_space, SUGGESTED_ALLOCATION_STRATEGY)
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
