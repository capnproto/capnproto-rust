// Copyright (c) 2013-2015 Sandstorm Development Group, Inc. and contributors
// Licensed under the MIT License:
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

//! Untyped root container for a Cap'n Proto value.

use any_pointer;
use private::capability::ClientHook;
use private::units::*;
use private::arena::{BuilderArena, ReaderArena, SegmentBuilder, SegmentReader, NumWords, ZeroedWords};
use private::layout;
use traits::{FromPointerReader, FromPointerBuilder, SetPointerBuilder};
use {OutputSegments, Result, Word};

/// Options controlling how data is read.
#[derive(Clone, Copy)]
pub struct ReaderOptions {

    /// Limits how many total words of data are allowed to be traversed. Traversal is counted when
    /// a new struct or list builder is obtained, e.g. from a get() accessor. This means that calling
    /// the getter for the same sub-struct multiple times will cause it to be double-counted. Once
    /// the traversal limit is reached, an error will be reported.
    ///
    /// This limit exists for security reasons. It is possible for an attacker to construct a message
    /// in which multiple pointers point at the same location. This is technically invalid, but hard
    /// to detect. Using such a message, an attacker could cause a message which is small on the wire
    /// to appear much larger when actually traversed, possibly exhausting server resources leading to
    /// denial-of-service.
    ///
    /// It makes sense to set a traversal limit that is much larger than the underlying message.
    /// Together with sensible coding practices (e.g. trying to avoid calling sub-object getters
    /// multiple times, which is expensive anyway), this should provide adequate protection without
    /// inconvenience.
    pub traversal_limit_in_words : u64,

    /// Limits how deeply nested a message structure can be, e.g. structs containing other structs or
    /// lists of structs.
    ///
    /// Like the traversal limit, this limit exists for security reasons. Since it is common to use
    /// recursive code to traverse recursive data structures, an attacker could easily cause a stack
    /// overflow by sending a very-depply-nested (or even cyclic) message, without the message even
    /// being very large. The default limit of 64 is probably low enough to prevent any chance of
    /// stack overflow, yet high enough that it is never a problem in practice.
    pub nesting_limit : i32,
}

pub const DEFAULT_READER_OPTIONS : ReaderOptions =
    ReaderOptions { traversal_limit_in_words : 8 * 1024 * 1024, nesting_limit : 64 };

impl ReaderOptions {
    pub fn new() -> ReaderOptions { DEFAULT_READER_OPTIONS }

    pub fn nesting_limit<'a>(&'a mut self, value : i32) -> &'a mut ReaderOptions {
        self.nesting_limit = value;
        return self;
    }

    pub fn traversal_limit_in_words<'a>(&'a mut self, value : u64) -> &'a mut ReaderOptions {
        self.traversal_limit_in_words = value;
        return self;
    }
}


type SegmentId = u32;

/// An abstract container used to read a message.
pub trait MessageReader {
    fn get_segment(&self, id : usize) -> &[Word];
    fn arena(&self) -> &ReaderArena;
    fn arena_mut(&mut self) -> &mut ReaderArena;
    fn get_options(&self) -> &ReaderOptions;

    fn get_root_internal(&self) -> Result<any_pointer::Reader> {
        unsafe {
            let segment : *const SegmentReader = &self.arena().segment0;

            let pointer_reader = try!(layout::PointerReader::get_root(
                segment, (*segment).get_start_ptr(), self.get_options().nesting_limit));

            Ok(any_pointer::Reader::new(pointer_reader))
        }
    }

    /// Gets the root of the message, interpreting it as the given type.
    fn get_root<'a, T : FromPointerReader<'a>>(&'a self) -> Result<T> {
        try!(self.get_root_internal()).get_as()
    }

    fn init_cap_table(&mut self, cap_table : Vec<Option<Box<ClientHook+Send>>>) {
        self.arena_mut().init_cap_table(cap_table);
    }
}

pub struct SegmentArrayMessageReader<'a> {
    segments : &'a [ &'a [Word]],
    options : ReaderOptions,
    arena : Box<ReaderArena>
}


impl <'a> MessageReader for SegmentArrayMessageReader<'a> {
    fn get_segment<'b>(&'b self, id : usize) -> &'b [Word] {
        self.segments[id]
    }

    fn arena<'b>(&'b self) -> &'b ReaderArena { &*self.arena }
    fn arena_mut<'b>(&'b mut self) -> &'b mut ReaderArena { &mut *self.arena }

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

#[derive(Clone, Copy)]
pub enum AllocationStrategy {
    FixedSize,
    GrowHeuristically
}

pub const SUGGESTED_FIRST_SEGMENT_WORDS : u32 = 1024;
pub const SUGGESTED_ALLOCATION_STRATEGY : AllocationStrategy = AllocationStrategy::GrowHeuristically;

#[derive(Clone, Copy)]
pub struct BuilderOptions {
    pub first_segment_words : u32,
    pub allocation_strategy : AllocationStrategy,
}

impl BuilderOptions {
    pub fn new() -> BuilderOptions {
        BuilderOptions {first_segment_words : SUGGESTED_FIRST_SEGMENT_WORDS,
                        allocation_strategy : AllocationStrategy::GrowHeuristically}
    }

    pub fn first_segment_words<'a>(&'a mut self, value : u32) -> &'a mut BuilderOptions {
        self.first_segment_words = value;
        return self;
    }

    pub fn allocation_strategy<'a>(&'a mut self, value : AllocationStrategy) -> &'a mut BuilderOptions {
        self.allocation_strategy = value;
        return self;
    }
}

/// An abstract container used to build a message.
pub trait MessageBuilder {
    fn arena_mut(&mut self) -> &mut BuilderArena;
    fn arena(&self) -> &BuilderArena;


    // XXX is there a way to make this private?
    fn get_root_internal<'a>(&mut self) -> any_pointer::Builder<'a> {
        let root_segment : *mut SegmentBuilder = &mut self.arena_mut().segment0;

        if self.arena().segment0.current_size() == 0 {
            match self.arena_mut().segment0.allocate(WORDS_PER_POINTER as u32) {
                None => {panic!("could not allocate root pointer") }
                Some(location) => {
                    assert!(location == self.arena().segment0.get_ptr_unchecked(0),
                            "First allocated word of new segment was not at offset 0");

                    any_pointer::Builder::new(layout::PointerBuilder::get_root(root_segment, location))

                }
            }
        } else {
            any_pointer::Builder::new(
                layout::PointerBuilder::get_root(root_segment,
                                                 self.arena().segment0.get_ptr_unchecked(0)))
        }

    }

    /// Initializes the root as a value of the given type.
    fn init_root<'a, T : FromPointerBuilder<'a>>(&'a mut self) -> T {
        self.get_root_internal().init_as()
    }

    /// Gets the root, interpreting it as the given type.
    fn get_root<'a, T : FromPointerBuilder<'a>>(&'a mut self) -> Result<T> {
        self.get_root_internal().get_as()
    }

    /// Sets the root to a deep copy of the given value.
    fn set_root<To, From : SetPointerBuilder<To>>(&mut self, value : From) -> Result<()> {
        self.get_root_internal().set_as(value)
    }

     fn get_segments_for_output<'a>(&'a self) -> OutputSegments<'a> {
        self.arena().get_segments_for_output()
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

unsafe impl Send for MallocMessageBuilder {}

impl MallocMessageBuilder {

    pub fn new(options : BuilderOptions) -> MallocMessageBuilder {
        let arena = BuilderArena::new(options.allocation_strategy,
                                      NumWords(options.first_segment_words));

        MallocMessageBuilder { arena : arena }
    }

    pub fn new_default() -> MallocMessageBuilder {
        MallocMessageBuilder::new(BuilderOptions::new())
    }

}

impl MessageBuilder for MallocMessageBuilder {
    fn arena_mut(&mut self) -> &mut BuilderArena {
        &mut *self.arena
    }
    fn arena(&self) -> &BuilderArena {
        & *self.arena
    }
}

pub struct ScratchSpaceMallocMessageBuilder<'a> {
    arena : Box<BuilderArena>,
    scratch_space : &'a mut [Word],
}

impl <'a> Drop for ScratchSpaceMallocMessageBuilder<'a> {
    fn drop(&mut self) {
        let ptr = self.scratch_space.as_mut_ptr();
        let segments = self.get_segments_for_output();
        unsafe {
            ::std::ptr::write_bytes(ptr, 0u8, segments[0].len());
        }
    }
}


impl <'a> ScratchSpaceMallocMessageBuilder<'a> {

    pub fn new<'b>(scratch_space : &'b mut [Word], options : BuilderOptions)
               -> ScratchSpaceMallocMessageBuilder<'b> {
        let arena = BuilderArena::new(options.allocation_strategy, ZeroedWords(scratch_space));

        ScratchSpaceMallocMessageBuilder { arena : arena, scratch_space : scratch_space }
    }

    pub fn new_default<'b>(scratch_space : &'b mut [Word]) -> ScratchSpaceMallocMessageBuilder<'b> {
        ScratchSpaceMallocMessageBuilder::new(scratch_space, BuilderOptions::new())
    }

}

impl <'b> MessageBuilder for ScratchSpaceMallocMessageBuilder<'b> {
    fn arena_mut(&mut self) -> &mut BuilderArena {
        &mut *self.arena
    }
    fn arena(&self) -> &BuilderArena {
        & *self.arena
    }
}

