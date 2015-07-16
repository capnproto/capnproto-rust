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
use private::arena::{BuilderArena, ReaderArena, SegmentBuilder, SegmentReader};
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

pub unsafe trait ReaderSegments {
    // UNSAFETY ALERT: The callee is responsible for ensuring that the returned memory is valid
    // for the lifetime of the object. This could be much longer than `'a`.
    fn get_segment<'a>(&'a self, id: u32) -> Option<&'a [Word]>;
}

pub struct SegmentArray<'a> {
    segments: &'a [&'a [Word]],
}

impl <'a> SegmentArray<'a> {
    pub fn new(segments: &'a [&'a [Word]]) -> SegmentArray<'a> {
        SegmentArray { segments: segments }
    }
}

unsafe impl <'b> ReaderSegments for SegmentArray<'b> {
    fn get_segment<'a>(&'a self, id: u32) -> Option<&'a [Word]> {
        if id < self.segments.len() as u32 {
            Some(self.segments[id as usize])
        } else {
            None
        }
    }
}

/// An abstract container used to read a message.
pub struct Reader<S> where S: ReaderSegments {
    arena: Box<ReaderArena>,
    segments: Box<S>,
    options: ReaderOptions,
}

unsafe impl <S> Send for Reader<S> where S: Send + ReaderSegments {}

impl <S> Reader<S> where S: ReaderSegments {
    pub fn new(segments: S, options: ReaderOptions) -> Reader<S> {
        let boxed_segments = Box::new(segments);
        let boxed_segments_ref = {
            let r :&S = &*boxed_segments;
            unsafe { ::std::mem::transmute(r as &ReaderSegments) }
        };
        let arena = ReaderArena::new(boxed_segments_ref, options);
        Reader { arena: arena, segments: boxed_segments, options: options }
    }

    fn get_root_internal(&self) -> Result<any_pointer::Reader> {
        unsafe {
            let segment : *const SegmentReader = &self.arena.segment0;

            let pointer_reader = try!(layout::PointerReader::get_root(
                segment, (*segment).get_start_ptr(), self.options.nesting_limit));

            Ok(any_pointer::Reader::new(pointer_reader))
        }
    }

    /// Gets the root of the message, interpreting it as the given type.
    pub fn get_root<'a, T : FromPointerReader<'a>>(&'a self) -> Result<T> {
        try!(self.get_root_internal()).get_as()
    }

    pub fn init_cap_table(&mut self, cap_table : Vec<Option<Box<ClientHook+Send>>>) {
        self.arena.init_cap_table(cap_table);
    }

    pub fn into_segments(self) -> S {
        *self.segments
    }
}

pub unsafe trait Allocator {
    // UNSAFETY ALERT: The callee is responsible for ensuring that the returned memory is valid
    // for the lifetime of the object.
    fn allocate_segment(&mut self, miniumum_size: u32) -> (*mut Word, u32);
    fn pre_drop(&mut self, _segment0_currently_allocated: u32) {}
}

/// A container used to build a message.
pub struct Builder<S> where S: Allocator {
    arena: Box<BuilderArena>,
    allocator: Box<S>,
}

unsafe impl <A> Send for Builder<A> where A: Send + Allocator {}

impl <S> Builder<S> where S: Allocator {
    pub fn new(allocator: S) -> Builder<S> {
        let mut boxed_allocator = Box::new(allocator);
        let boxed_allocator_ref = {
            let r: &mut S = &mut *boxed_allocator;
            unsafe { ::std::mem::transmute(r as &mut Allocator) }
        };
        let arena = BuilderArena::new(boxed_allocator_ref);
        Builder { arena: arena, allocator: boxed_allocator }
    }

    fn get_root_internal<'a>(&mut self) -> any_pointer::Builder<'a> {
        let root_segment: *mut SegmentBuilder = &mut self.arena.segment0;

        if self.arena.segment0.current_size() == 0 {
            match self.arena.segment0.allocate(WORDS_PER_POINTER as u32) {
                None => {panic!("could not allocate root pointer") }
                Some(location) => {
                    assert!(location == self.arena.segment0.get_ptr_unchecked(0),
                            "First allocated word of new segment was not at offset 0");

                    any_pointer::Builder::new(layout::PointerBuilder::get_root(root_segment, location))

                }
            }
        } else {
            any_pointer::Builder::new(
                layout::PointerBuilder::get_root(root_segment,
                                                 self.arena.segment0.get_ptr_unchecked(0)))
        }

    }

    /// Initializes the root as a value of the given type.
    pub fn init_root<'a, T : FromPointerBuilder<'a>>(&'a mut self) -> T {
        self.get_root_internal().init_as()
    }

    /// Gets the root, interpreting it as the given type.
    pub fn get_root<'a, T : FromPointerBuilder<'a>>(&'a mut self) -> Result<T> {
        self.get_root_internal().get_as()
    }

    /// Sets the root to a deep copy of the given value.
    pub fn set_root<To, From : SetPointerBuilder<To>>(&mut self, value : From) -> Result<()> {
        self.get_root_internal().set_as(value)
    }

    pub fn get_segments_for_output<'a>(&'a self) -> OutputSegments<'a> {
        self.arena.get_segments_for_output()
    }

    pub fn get_cap_table<'a>(&'a self) -> &'a [Option<Box<ClientHook+Send>>] {
        self.arena.get_cap_table()
    }
}

impl <S> Drop for Builder<S> where S: Allocator {
    fn drop(&mut self) {
        self.allocator.pre_drop(self.arena.segment0.current_size());
    }
}

pub struct HeapAllocator {
    owned_memory : Vec<Vec<Word>>,
    next_size: u32,
    allocation_strategy: AllocationStrategy,
}

#[derive(Clone, Copy)]
pub enum AllocationStrategy {
    FixedSize,
    GrowHeuristically
}

pub const SUGGESTED_FIRST_SEGMENT_WORDS : u32 = 1024;
pub const SUGGESTED_ALLOCATION_STRATEGY : AllocationStrategy = AllocationStrategy::GrowHeuristically;

impl HeapAllocator {
    pub fn new() -> HeapAllocator {
        HeapAllocator { owned_memory: Vec::new(),
                        next_size: SUGGESTED_FIRST_SEGMENT_WORDS,
                        allocation_strategy: SUGGESTED_ALLOCATION_STRATEGY }
    }

    pub fn first_segment_words(mut self, value: u32) -> HeapAllocator {
        self.next_size = value;
        return self;
    }

    pub fn allocation_strategy(mut self, value : AllocationStrategy) -> HeapAllocator {
        self.allocation_strategy = value;
        return self;
    }
}

unsafe impl Allocator for HeapAllocator {
    fn allocate_segment(&mut self, minimum_size: u32) -> (*mut Word, u32) {
        let size = ::std::cmp::max(minimum_size, self.next_size);
        let mut new_words = Word::allocate_zeroed_vec(size as usize);
        let ptr = new_words.as_mut_ptr();
        self.owned_memory.push(new_words);

        match self.allocation_strategy {
            AllocationStrategy::GrowHeuristically => { self.next_size += size; }
            _ => { }
        }
        (ptr, size as u32)
    }
}

impl Builder<HeapAllocator> {
    pub fn new_default() -> Builder<HeapAllocator> {
        Builder::new(HeapAllocator::new())
    }
}

pub struct ScratchSpace<'a> {
    slice: &'a mut [Word],
    in_use: bool,
}

impl <'a> ScratchSpace<'a> {
    pub fn new(slice: &'a mut [Word]) -> ScratchSpace<'a> {
        ScratchSpace { slice: slice, in_use: false }
    }
}

pub struct ScratchSpaceHeapAllocator<'a, 'b: 'a> {
    scratch_space: &'a mut ScratchSpace<'b>,
    allocator: HeapAllocator,
}

impl <'a, 'b: 'a> ScratchSpaceHeapAllocator<'a, 'b> {
    pub fn new(scratch_space: &'a mut ScratchSpace<'b>) -> ScratchSpaceHeapAllocator<'a, 'b> {
        ScratchSpaceHeapAllocator { scratch_space: scratch_space,
                                    allocator: HeapAllocator::new()}
    }

    pub fn second_segment_words(mut self, value: u32) -> ScratchSpaceHeapAllocator<'a, 'b> {
        ScratchSpaceHeapAllocator { scratch_space: self.scratch_space,
                                    allocator: self.allocator.first_segment_words(value) }

    }

    pub fn allocation_strategy(mut self, value: AllocationStrategy) -> ScratchSpaceHeapAllocator<'a, 'b> {
        ScratchSpaceHeapAllocator { scratch_space: self.scratch_space,
                                    allocator: self.allocator.allocation_strategy(value) }
    }

}

unsafe impl <'a, 'b: 'a> Allocator for ScratchSpaceHeapAllocator<'a, 'b> {
    fn allocate_segment(&mut self, minimum_size: u32) -> (*mut Word, u32) {
        if !self.scratch_space.in_use {
            self.scratch_space.in_use = true;
            (self.scratch_space.slice.as_mut_ptr(), self.scratch_space.slice.len() as u32)
        } else {
            self.allocator.allocate_segment(minimum_size)
        }
    }

    fn pre_drop(&mut self, segment0_currently_allocated: u32) {
        let ptr = self.scratch_space.slice.as_mut_ptr();
        unsafe {
            ::std::ptr::write_bytes(ptr, 0u8, segment0_currently_allocated as usize);
        }
        self.scratch_space.in_use = false;
    }
}
