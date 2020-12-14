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

use alloc::vec::Vec;
use core::convert::From;

use crate::any_pointer;
use crate::private::arena::{BuilderArenaImpl, ReaderArenaImpl, ReadLimiterImpl, BuilderArena, ReaderArena};
use crate::private::layout;
use crate::private::units::BYTES_PER_WORD;
use crate::traits::{FromPointerReader, FromPointerBuilder, SetPointerBuilder, Owned};
use crate::{OutputSegments, Result};

/// Options controlling how data is read.
#[derive(Clone, Copy, Debug)]
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
    pub traversal_limit_in_words: u64,

    /// Limits how deeply nested a message structure can be, e.g. structs containing other structs or
    /// lists of structs.
    ///
    /// Like the traversal limit, this limit exists for security reasons. Since it is common to use
    /// recursive code to traverse recursive data structures, an attacker could easily cause a stack
    /// overflow by sending a very-depply-nested (or even cyclic) message, without the message even
    /// being very large. The default limit of 64 is probably low enough to prevent any chance of
    /// stack overflow, yet high enough that it is never a problem in practice.
    pub nesting_limit: i32,
}

pub const DEFAULT_READER_OPTIONS: ReaderOptions =
    ReaderOptions { traversal_limit_in_words: 8 * 1024 * 1024, nesting_limit: 64 };


impl Default for ReaderOptions {
    fn default() -> ReaderOptions {
        DEFAULT_READER_OPTIONS
    }
}

impl ReaderOptions {
    pub fn new() -> ReaderOptions { DEFAULT_READER_OPTIONS }

    pub fn nesting_limit<'a>(&'a mut self, value: i32) -> &'a mut ReaderOptions {
        self.nesting_limit = value;
        self
    }

    pub fn traversal_limit_in_words<'a>(&'a mut self, value: u64) -> &'a mut ReaderOptions {
        self.traversal_limit_in_words = value;
        self
    }
}

/// An object that manages the buffers underlying a Cap'n Proto message reader.
pub trait ReaderSegments {
    /// Gets the segment with index `idx`. Returns `None` if `idx` is out of range.
    ///
    /// The segment must be 8-byte aligned or the "unaligned" feature must
    /// be enabled in the capnp crate. (Otherwise reading the segment will return an error.)
    ///
    /// The returned slice is required to point to memory that remains valid until the ReaderSegments
    /// object is dropped. In safe Rust, it should not be possible to violate this requirement.
    fn get_segment<'a>(&'a self, idx: u32) -> Option<&'a [u8]>;

    /// Gets the number of segments.
    fn len(&self) -> usize {
        for i in 0.. {
            if self.get_segment(i as u32).is_none() {
                return i;
            }
        }
        unreachable!()
    }
}

impl <S> ReaderSegments for &S where S: ReaderSegments {
    fn get_segment<'a>(&'a self, idx: u32) -> Option<&'a [u8]> {
        (**self).get_segment(idx)
    }

    fn len(&self) -> usize {
        (**self).len()
    }
}

/// An array of segments.
pub struct SegmentArray<'a> {
    segments: &'a [&'a [u8]],
}

impl <'a> SegmentArray<'a> {
    pub fn new(segments: &'a [&'a [u8]]) -> SegmentArray<'a> {
        SegmentArray { segments: segments }
    }
}

impl <'b> ReaderSegments for SegmentArray<'b> {
    fn get_segment<'a>(&'a self, id: u32) -> Option<&'a [u8]> {
        self.segments.get(id as usize).map(|slice| *slice)
    }

    fn len(&self) -> usize {
        self.segments.len()
    }
}

impl <'b> ReaderSegments for [&'b [u8]] {
    fn get_segment<'a>(&'a self, id: u32) -> Option<&'a [u8]> {
        self.get(id as usize).map(|slice| *slice)
    }

    fn len(&self) -> usize {
        self.len()
    }
}

/// A container used to read a message.
pub struct Reader<A> where A: ReaderArena {
    arena: A,
}

impl <A> Reader<A> where A: ReaderArena {
    pub fn new<Align>(segments: S, options: ReaderOptions) -> Reader<ReaderArenaImpl<S, ReadLimiterImpl, Align>>
        where Align: crate::private::primitive::Alignment
    {
        Reader {
            arena: ReaderArenaImpl::new::<Align>(segments, options),
        }
    }

    fn get_root_internal<'a>(&'a self) -> Result<any_pointer::Reader<'a, A>> {
        let (segment_start, _seg_len) = self.arena.get_segment(0)?;
        let pointer_reader = layout::PointerReader::get_root(
            &self.arena, 0, segment_start, self.arena.nesting_limit())?;
        Ok(any_pointer::Reader::new(pointer_reader))
    }

    /// Gets the root of the message, interpreting it as the given type.
    pub fn get_root<'a, T: FromPointerReader<'a, A>>(&'a self) -> Result<T> {
        self.get_root_internal()?.get_as()
    }

    pub fn into_segments(self) -> S {
        self.arena.into_segments()
    }

    /// Checks whether the message is [canonical](https://capnproto.org/encoding.html#canonicalization).
    pub fn is_canonical(&self) -> Result<bool> {
        let (segment_start, seg_len) = self.arena.get_segment(0)?;

        if self.arena.get_segment(1).is_ok() {
            // TODO(cleanup, apibump): should there be a nicer way to ask the arena how many
            // segments there are?

            // There is more than one segment, so the message cannot be canonical.
            return Ok(false)
        }

        let pointer_reader = layout::PointerReader::get_root(
            &self.arena, 0, segment_start, self.arena.nesting_limit())?;
        let read_head = ::core::cell::Cell::new(unsafe {segment_start.offset(BYTES_PER_WORD as isize)});
        let root_is_canonical = pointer_reader.is_canonical(&read_head)?;
        let all_words_consumed =
            (read_head.get() as usize - segment_start as usize) / BYTES_PER_WORD == seg_len as usize;
        Ok(root_is_canonical && all_words_consumed)
    }

    /// Gets the [canonical](https://capnproto.org/encoding.html#canonicalization) form
    /// of this message. Works by copying the message twice. For a canonicalization
    /// method that only requires one copy, see `message::Builder::set_root_canonical()`.
    pub fn canonicalize(&self) -> Result<Vec<crate::Word>> {
        let root = self.get_root_internal()?;
        let size = root.target_size()?.word_count + 1;
        let mut message = Builder::new(HeapAllocator::new().first_segment_words(size as u32));
        message.set_root_canonical(root)?;
        let output_segments = message.get_segments_for_output();
        assert_eq!(1, output_segments.len());
        let output = output_segments[0];
        assert!((output.len() / BYTES_PER_WORD) as u64 <= size);
        let mut result = crate::Word::allocate_zeroed_vec(output.len() / BYTES_PER_WORD);
        crate::Word::words_to_bytes_mut(&mut result[..]).copy_from_slice(output);
        Ok(result)
    }

//    pub fn into_typed<T: Owned>(self) -> TypedReader<S, T> {
//        TypedReader::new(self)
//    }
}
/*
/// A message reader whose value is known to be of type `T`.
pub struct TypedReader<S, T>
    where S: ReaderSegments,
          T: Owned {
    marker: ::core::marker::PhantomData<T>,
    message: Reader<S>,
}

impl <S, T> TypedReader<S, T>
    where S: ReaderSegments,
          T : Owned {

    pub fn new(message: Reader<S>) -> Self {
        TypedReader {
            marker: ::core::marker::PhantomData,
            message: message,
        }
    }

    pub fn get<'a> (&'a self) -> Result<<T as Owned>::Reader<'a, ReaderArenaImpl<S, ReadLimiterImpl, crate::private::primitive::Unaligned>>> {
        self.message.get_root()
    }

    pub fn into_inner(self) -> Reader<S> {
        self.message
    }
}

impl <S, T> From<Reader<S>> for TypedReader<S, T>
    where S: ReaderSegments,
          T: Owned {

    fn from(message: Reader<S>) -> TypedReader<S, T> {
        TypedReader::new(message)
    }
}

impl <A, T> From<Builder<A>> for TypedReader<Builder<A>, T>
    where A: Allocator,
          T: Owned {

    fn from(message: Builder<A>) -> TypedReader<Builder<A>, T> {
        let reader = message.into_reader();
        reader.into_typed()
    }
}
*/
/// An object that allocates memory for a Cap'n Proto message as it is being built.
pub unsafe trait Allocator {
    /// Allocates zeroed memory for a new segment, returning a pointer to the start of the segment
    /// and a u32 indicating the length of the segment in words. The allocated segment must be
    /// at least `minimum_size` words long (`minimum_size * 8` bytes long). Allocator implementations
    /// commonly allocate much more than the minimum, to reduce the total number of segments needed.
    /// A reasonable strategy is to allocate the maximum of `minimum_size` and twice the size of the
    /// previous segment.
    ///
    /// UNSAFETY ALERT: Implementors must ensure all of the following:
    ///     1. the returned memory is initialized to all zeroes,
    ///     2. the returned memory is valid until deallocate_segment() is called on it,
    ///     3. the memory doesn't overlap with other allocated memory,
    ///     4. the memory is 8-byte aligned (or the "unaligned" feature is enabled for the capnp crate).
    fn allocate_segment(&mut self, minimum_size: u32) -> (*mut u8, u32);

    /// Indicates that a segment, previously allocated via allocate_segment(), is no longer in use.
    /// `word_size` is the length of the segment in words, as returned from `allocate_segment()`.
    /// `words_used` is always less than or equal to `word_size`, and indicates how many
    /// words (contiguous from the start of the segment) were possibly written with non-zero values.
    fn deallocate_segment(&mut self, ptr: *mut u8, word_size: u32, words_used: u32);
}

/// A container used to build a message.
pub struct Builder<A> where A: Allocator {
    arena: BuilderArenaImpl<A>,
}

unsafe impl <A> Send for Builder<A> where A: Send + Allocator {}

fn _assert_kinds() {
    fn _assert_send<T: Send>() {}
    fn _assert_reader<S: ReaderSegments + Send>() {
        _assert_send::<Reader<S>>();
    }
    fn _assert_builder<A: Allocator + Send>() {
        _assert_send::<Builder<A>>();
    }
}

impl <A> Builder<A> where A: Allocator {
    pub fn new(allocator: A) -> Self {
        Builder {
            arena: BuilderArenaImpl::new(allocator),
        }
    }

    fn get_root_internal<'a>(&'a mut self) -> any_pointer::Builder<'a, BuilderArenaImpl<A>> {
        if self.arena.len() == 0 {
            self.arena.allocate_segment(1).expect("allocate root pointer");
            self.arena.allocate(0, 1).expect("allocate root pointer");
        }
        let (seg_start, _seg_len) = self.arena.get_segment_mut(0);
        let location: *mut u8 = seg_start;
        let Builder { ref mut arena } = *self;

        any_pointer::Builder::new(
            layout::PointerBuilder::get_root(arena, 0, location))
    }

    /// Initializes the root as a value of the given type.
    pub fn init_root<'a, T: FromPointerBuilder<'a, BuilderArenaImpl<A>>>(&'a mut self) -> T {
        let root = self.get_root_internal();
        root.init_as()
    }

    /// Gets the root, interpreting it as the given type.
    pub fn get_root<'a, T: FromPointerBuilder<'a, BuilderArenaImpl<A>>>(&'a mut self) -> Result<T> {
        let root = self.get_root_internal();
        root.get_as()
    }

    pub fn get_root_as_reader<'a, T: FromPointerReader<'a, BuilderArenaImpl<A>>>(&'a self) -> Result<T> {
        if self.arena.len() == 0 {
            any_pointer::Reader::new(layout::PointerReader::new_default()).get_as()
        } else {
            let (segment_start, _segment_len) = self.arena.get_segment(0)?;
            let pointer_reader = layout::PointerReader::get_root(
                &self.arena, 0, segment_start, 0x7fffffff)?;
            let root = any_pointer::Reader::new(pointer_reader);
            root.get_as()
        }
    }

    /// Sets the root to a deep copy of the given value.
    pub fn set_root<From: SetPointerBuilder>(&mut self, value: From) -> Result<()> {
        let root = self.get_root_internal();
        root.set_as(value)
    }

    /// Sets the root to a canonicalized version of `value`. If this was the first action taken
    /// on this `Builder`, then a subsequent call to `get_segments_for_output()` should return
    /// a single segment, containing the full canonicalized message.
    pub fn set_root_canonical<From: SetPointerBuilder>(&mut self, value: From) -> Result<()> {
        if self.arena.len() == 0 {
            self.arena.allocate_segment(1).expect("allocate root pointer");
            self.arena.allocate(0, 1).expect("allocate root pointer");
        }
        let (seg_start, _seg_len) = self.arena.get_segment_mut(0);
        let pointer = layout::PointerBuilder::get_root(&mut self.arena, 0, seg_start);
        SetPointerBuilder::set_pointer_builder(pointer, value, true)?;
        assert_eq!(self.get_segments_for_output().len(), 1);
        Ok(())
    }

    pub fn get_segments_for_output<'a>(&'a self) -> OutputSegments<'a> {
        self.arena.get_segments_for_output()
    }
/*
    pub fn into_reader(self) -> Reader<Builder<A>> {
        Reader::new(self, ReaderOptions {
            traversal_limit_in_words: u64::max_value(),
            nesting_limit: i32::max_value()
        })
    }
*/
    pub fn into_allocator(self) -> A {
        self.arena.into_allocator()
    }
}

impl <A> ReaderSegments for Builder<A> where A: Allocator {
    fn get_segment<'a>(&'a self, id: u32) -> Option<&'a [u8]> {
        self.get_segments_for_output().get(id as usize).map(|x| *x)
    }

    fn len(&self) -> usize {
        self.get_segments_for_output().len()
    }
}

/// Standard segment allocator. Allocates each segment via `alloc::alloc::alloc_zeroed()`.
#[derive(Debug)]
pub struct HeapAllocator {
    // Minimum number of words in the next allocation.
    next_size: u32,

    // How to update next_size after an allocation.
    allocation_strategy: AllocationStrategy,
}

#[derive(Clone, Copy, Debug)]
pub enum AllocationStrategy {
    /// Allocates the same number of words for each segment, to the extent possible.
    /// This strategy is primarily useful for testing cross-segment pointers.
    FixedSize,

    /// Increases segment size by a multiplicative factor for each subsequent segment.
    GrowHeuristically,
}

pub const SUGGESTED_FIRST_SEGMENT_WORDS: u32 = 1024;
pub const SUGGESTED_ALLOCATION_STRATEGY: AllocationStrategy = AllocationStrategy::GrowHeuristically;

impl HeapAllocator {
    pub fn new() -> HeapAllocator {
        HeapAllocator { next_size: SUGGESTED_FIRST_SEGMENT_WORDS,
                        allocation_strategy: SUGGESTED_ALLOCATION_STRATEGY }
    }

    /// Sets the size of the initial segment in words, where 1 word = 8 bytes.
    pub fn first_segment_words(mut self, value: u32) -> HeapAllocator {
        self.next_size = value;
        self
    }

    /// Sets the allocation strategy for segments after the first one.
    pub fn allocation_strategy(mut self, value : AllocationStrategy) -> HeapAllocator {
        self.allocation_strategy = value;
        self
    }
}

unsafe impl Allocator for HeapAllocator {
    fn allocate_segment(&mut self, minimum_size: u32) -> (*mut u8, u32) {
        let size = core::cmp::max(minimum_size, self.next_size);
        let ptr = unsafe {
            alloc::alloc::alloc_zeroed(alloc::alloc::Layout::from_size_align(size as usize * BYTES_PER_WORD, 8).unwrap())
        };
        match self.allocation_strategy {
            AllocationStrategy::GrowHeuristically => { self.next_size += size; }
            AllocationStrategy::FixedSize => { }
        }
        (ptr, size as u32)
    }

    fn deallocate_segment(&mut self, ptr: *mut u8, word_size: u32, _words_used: u32) {
        unsafe {
            alloc::alloc::dealloc(ptr,
                                  alloc::alloc::Layout::from_size_align(word_size as usize * BYTES_PER_WORD, 8).unwrap());
        }
        self.next_size = SUGGESTED_FIRST_SEGMENT_WORDS;
    }
}

impl Builder<HeapAllocator> {
    pub fn new_default() -> Builder<HeapAllocator> {
        Builder::new(HeapAllocator::new())
    }
}

/// An Allocator whose first segment is a backed by a user-provided buffer.
///
/// Recall that an `Allocator` implementation must ensure that allocated segments are
/// initially *zeroed*. `ScratchSpaceHeapAllocator` ensures that is the case by zeroing
/// the entire buffer upon initial construction, and then zeroing any *potentially used*
/// part of the buffer upon `deallocate_segment()`.
///
/// You can reuse a `ScratchSpaceHeapAllocator` by calling `message::Builder::into_allocator()`,
/// or by initally passing it to `message::Builder::new()` as a `&mut ScratchSpaceHeapAllocator`.
/// Such reuse can save significant amounts of zeroing.
pub struct ScratchSpaceHeapAllocator<'a> {
    scratch_space: &'a mut [u8],
    scratch_space_allocated: bool,
    allocator: HeapAllocator,
}

impl <'a> ScratchSpaceHeapAllocator<'a> {
    /// Writes zeroes into the entire buffer and constructs a new allocator from it.
    ///
    /// If the buffer is large, this operation could be relatively expensive. If you want to reuse
    /// the same scratch space in a later message, you should reuse the entire
    /// `ScratchSpaceHeapAllocator`, to avoid paying this full cost again.
    pub fn new(scratch_space: &'a mut [u8]) -> ScratchSpaceHeapAllocator<'a> {
        #[cfg(not(feature = "unaligned"))]
        {
            if scratch_space.as_ptr() as usize % BYTES_PER_WORD != 0 {
                panic!("Scratch space must be 8-byte aligned, or you must enable the \"unaligned\" \
                        feature in the capnp crate");
            }
        }

        // We need to ensure that the buffer is zeroed.
        for b in &mut scratch_space[..] {
            *b = 0;
        }
        ScratchSpaceHeapAllocator { scratch_space: scratch_space,
                                    scratch_space_allocated: false,
                                    allocator: HeapAllocator::new()}
    }

    /// Sets the size of the second segment in words, where 1 word = 8 bytes.
    /// (The first segment is the scratch space passed to `ScratchSpaceHeapAllocator::new()`.
    pub fn second_segment_words(self, value: u32) -> ScratchSpaceHeapAllocator<'a> {
        ScratchSpaceHeapAllocator { allocator: self.allocator.first_segment_words(value), ..self }

    }

    /// Sets the allocation strategy for segments after the second one.
    pub fn allocation_strategy(self, value: AllocationStrategy) -> ScratchSpaceHeapAllocator<'a> {
        ScratchSpaceHeapAllocator { allocator: self.allocator.allocation_strategy(value), ..self }
    }
}

unsafe impl <'a> Allocator for ScratchSpaceHeapAllocator<'a> {
    fn allocate_segment(&mut self, minimum_size: u32) -> (*mut u8, u32) {
        if (minimum_size as usize) < (self.scratch_space.len() / BYTES_PER_WORD) && !self.scratch_space_allocated {
            self.scratch_space_allocated = true;
            (self.scratch_space.as_mut_ptr(), (self.scratch_space.len() / BYTES_PER_WORD) as u32)
        } else {
            self.allocator.allocate_segment(minimum_size)
        }
    }

    fn deallocate_segment(&mut self, ptr: *mut u8, word_size: u32, words_used: u32) {
        if ptr == self.scratch_space.as_mut_ptr() {
            // Rezero the slice to allow reuse of the allocator. We only need to write
            // words that we know might contain nonzero values.
            unsafe {
                core::ptr::write_bytes(ptr, 0u8, (words_used as usize) * BYTES_PER_WORD);
            }
            self.scratch_space_allocated = false;
        } else {
            self.allocator.deallocate_segment(ptr, word_size, words_used);
        }
    }
}

unsafe impl <'a, A> Allocator for &'a mut A where A: Allocator {
    fn allocate_segment(&mut self, minimum_size: u32) -> (*mut u8, u32) {
        (*self).allocate_segment(minimum_size)
    }

    fn deallocate_segment(&mut self, ptr: *mut u8, word_size: u32, words_used: u32) {
        (*self).deallocate_segment(ptr, word_size, words_used)
    }
}

