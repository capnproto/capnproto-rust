// Copyright (c) 2013-2017 Sandstorm Development Group, Inc. and contributors
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

use core::slice;

use crate::message;
use crate::message::Allocator;
use crate::message::ReaderSegments;
use crate::private::read_limiter::ReadLimiter;
use crate::private::units::*;
use crate::OutputSegments;
use crate::{Error, ErrorKind, Result};

pub type SegmentId = u32;

pub unsafe trait ReaderArena {
    // return pointer to start of segment, and number of words in that segment
    fn get_segment(&self, id: u32) -> Result<(*const u8, u32)>;

    unsafe fn check_offset(
        &self,
        segment_id: u32,
        start: *const u8,
        offset_in_words: i32,
    ) -> Result<*const u8> {
        let (segment_start, segment_len) = self.get_segment(segment_id)?;
        let this_start: usize = segment_start as usize;
        let this_size: usize = segment_len as usize * BYTES_PER_WORD;
        let offset: i64 = i64::from(offset_in_words) * BYTES_PER_WORD as i64;
        let start_idx = start as usize;
        if start_idx < this_start || ((start_idx - this_start) as i64 + offset) as usize > this_size
        {
            Err(Error::from_kind(
                ErrorKind::MessageContainsOutOfBoundsPointer,
            ))
        } else {
            unsafe { Ok(start.offset(offset as isize)) }
        }
    }

    fn contains_interval(&self, segment_id: u32, start: *const u8, size: usize) -> Result<()>;
    fn amplified_read(&self, virtual_amount: u64) -> Result<()>;

    fn nesting_limit(&self) -> i32;

    fn size_in_words(&self) -> usize;

    // TODO(apibump): Consider putting extract_cap(), inject_cap(), drop_cap() here
    //   and on message::Reader. Then we could get rid of Imbue and ImbueMut, and
    //   layout::StructReader, layout::ListReader, etc. could drop their `cap_table` fields.
}

pub struct ReaderArenaImpl<S> {
    segments: S,
    read_limiter: ReadLimiter,
    nesting_limit: i32,
}

#[cfg(feature = "sync_reader")]
fn _assert_sync() {
    fn _assert_sync<T: Sync>() {}
    fn _assert_reader<S: ReaderSegments + Sync>() {
        _assert_sync::<ReaderArenaImpl<S>>();
    }
}

impl<S> ReaderArenaImpl<S>
where
    S: ReaderSegments,
{
    pub fn new(segments: S, options: message::ReaderOptions) -> Self {
        let limiter = ReadLimiter::new(options.traversal_limit_in_words);
        Self {
            segments,
            read_limiter: limiter,
            nesting_limit: options.nesting_limit,
        }
    }

    pub fn into_segments(self) -> S {
        self.segments
    }

    pub(crate) fn get_segments(&self) -> &S {
        &self.segments
    }
}

unsafe impl<S> ReaderArena for ReaderArenaImpl<S>
where
    S: ReaderSegments,
{
    fn get_segment(&self, id: u32) -> Result<(*const u8, u32)> {
        match self.segments.get_segment(id) {
            Some(seg) => {
                #[cfg(not(feature = "unaligned"))]
                {
                    if seg.as_ptr() as usize % BYTES_PER_WORD != 0 {
                        return Err(Error::from_kind(ErrorKind::UnalignedSegment));
                    }
                }

                Ok((seg.as_ptr(), (seg.len() / BYTES_PER_WORD) as u32))
            }
            None => Err(Error::from_kind(ErrorKind::InvalidSegmentId(id))),
        }
    }

    fn contains_interval(&self, id: u32, start: *const u8, size_in_words: usize) -> Result<()> {
        let (segment_start, segment_len) = self.get_segment(id)?;
        let this_start: usize = segment_start as usize;
        let this_size: usize = segment_len as usize * BYTES_PER_WORD;
        let start = start as usize;
        let size = size_in_words * BYTES_PER_WORD;

        if !(start >= this_start && start - this_start + size <= this_size) {
            Err(Error::from_kind(
                ErrorKind::MessageContainsOutOfBoundsPointer,
            ))
        } else {
            self.read_limiter.can_read(size_in_words)
        }
    }

    fn amplified_read(&self, virtual_amount: u64) -> Result<()> {
        self.read_limiter.can_read(virtual_amount as usize)
    }

    fn nesting_limit(&self) -> i32 {
        self.nesting_limit
    }

    fn size_in_words(&self) -> usize {
        let mut result = 0;
        for ii in 0..self.segments.len() {
            if let Some(seg) = self.segments.get_segment(ii as u32) {
                result += seg.len() / BYTES_PER_WORD;
            }
        }
        result
    }
}

pub unsafe trait BuilderArena: ReaderArena {
    fn allocate(&mut self, segment_id: u32, amount: WordCount32) -> Option<u32>;
    fn allocate_anywhere(&mut self, amount: u32) -> (SegmentId, u32);
    fn get_segment_mut(&mut self, id: u32) -> (*mut u8, u32);

    fn as_reader(&self) -> &dyn ReaderArena;
}

/// A wrapper around a memory segment used in building a message.
struct BuilderSegment {
    /// Pointer to the start of the segment.
    ptr: *mut u8,

    /// Total number of words the segment could potentially use. That is, all
    /// bytes from `ptr` to `ptr + (capacity * 8)` may be used in the segment.
    capacity: u32,

    /// Number of words already used in the segment.
    allocated: u32,
}

#[cfg(feature = "alloc")]
type BuilderSegmentArray = alloc::vec::Vec<BuilderSegment>;

#[cfg(not(feature = "alloc"))]
#[derive(Default)]
struct BuilderSegmentArray {
    // In the no-alloc case, we only allow a single segment.
    segment: Option<BuilderSegment>,
}

#[cfg(not(feature = "alloc"))]
impl BuilderSegmentArray {
    fn len(&self) -> usize {
        match self.segment {
            Some(_) => 1,
            None => 0,
        }
    }

    fn push(&mut self, segment: BuilderSegment) {
        if self.segment.is_some() {
            panic!("multiple segments are not supported in no-alloc mode")
        }
        self.segment = Some(segment);
    }
}

#[cfg(not(feature = "alloc"))]
impl core::ops::Index<usize> for BuilderSegmentArray {
    type Output = BuilderSegment;

    fn index(&self, index: usize) -> &Self::Output {
        assert_eq!(index, 0);
        match &self.segment {
            Some(s) => s,
            None => panic!("no segment"),
        }
    }
}

#[cfg(not(feature = "alloc"))]
impl core::ops::IndexMut<usize> for BuilderSegmentArray {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert_eq!(index, 0);
        match &mut self.segment {
            Some(s) => s,
            None => panic!("no segment"),
        }
    }
}

pub struct BuilderArenaImplInner<A>
where
    A: Allocator,
{
    allocator: Option<A>, // None if has already been deallocated.
    segments: BuilderSegmentArray,
}

pub struct BuilderArenaImpl<A>
where
    A: Allocator,
{
    inner: BuilderArenaImplInner<A>,
}

// BuilderArenaImpl has no interior mutability. Adding these impls
// allows message::Builder<A> to be Send and/or Sync when appropriate.
unsafe impl<A> Send for BuilderArenaImpl<A> where A: Send + Allocator {}
unsafe impl<A> Sync for BuilderArenaImpl<A> where A: Sync + Allocator {}

impl<A> BuilderArenaImpl<A>
where
    A: Allocator,
{
    pub fn new(allocator: A) -> Self {
        Self {
            inner: BuilderArenaImplInner {
                allocator: Some(allocator),
                segments: Default::default(),
            },
        }
    }

    /// Allocates a new segment with capacity for at least `minimum_size` words.
    pub fn allocate_segment(&mut self, minimum_size: u32) -> Result<()> {
        self.inner.allocate_segment(minimum_size)
    }

    pub fn get_segments_for_output(&self) -> OutputSegments<'_> {
        let reff = &self.inner;
        if reff.segments.len() == 1 {
            let seg = &reff.segments[0];

            // The user must mutably borrow the `message::Builder` to be able to modify segment memory.
            // No such borrow will be possible while `self` is still immutably borrowed from this method,
            // so returning this slice is safe.
            let slice = unsafe {
                slice::from_raw_parts(seg.ptr as *const _, seg.allocated as usize * BYTES_PER_WORD)
            };
            OutputSegments::SingleSegment([slice])
        } else {
            #[cfg(feature = "alloc")]
            {
                let mut v = alloc::vec::Vec::with_capacity(reff.segments.len());
                for seg in &reff.segments {
                    // See safety argument in above branch.
                    let slice = unsafe {
                        slice::from_raw_parts(
                            seg.ptr as *const _,
                            seg.allocated as usize * BYTES_PER_WORD,
                        )
                    };
                    v.push(slice);
                }
                OutputSegments::MultiSegment(v)
            }
            #[cfg(not(feature = "alloc"))]
            {
                panic!("invalid number of segments: {}", reff.segments.len());
            }
        }
    }

    pub fn len(&self) -> usize {
        self.inner.segments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Retrieves the underlying `Allocator`, deallocating all currently-allocated
    /// segments.
    pub fn into_allocator(mut self) -> A {
        self.inner.deallocate_all();
        self.inner.allocator.take().unwrap()
    }
}

unsafe impl<A> ReaderArena for BuilderArenaImpl<A>
where
    A: Allocator,
{
    fn get_segment(&self, id: u32) -> Result<(*const u8, u32)> {
        let seg = &self.inner.segments[id as usize];
        Ok((seg.ptr, seg.allocated))
    }

    unsafe fn check_offset(
        &self,
        _segment_id: u32,
        start: *const u8,
        offset_in_words: i32,
    ) -> Result<*const u8> {
        unsafe { Ok(start.offset((i64::from(offset_in_words) * BYTES_PER_WORD as i64) as isize)) }
    }

    fn contains_interval(&self, _id: u32, _start: *const u8, _size: usize) -> Result<()> {
        Ok(())
    }

    fn amplified_read(&self, _virtual_amount: u64) -> Result<()> {
        Ok(())
    }

    fn nesting_limit(&self) -> i32 {
        0x7fffffff
    }

    fn size_in_words(&self) -> usize {
        let mut result = 0;
        for ii in 0..self.inner.segments.len() {
            result += self.inner.segments[ii].allocated as usize
        }
        result
    }
}

impl<A> BuilderArenaImplInner<A>
where
    A: Allocator,
{
    /// Allocates a new segment with capacity for at least `minimum_size` words.
    fn allocate_segment(&mut self, minimum_size: WordCount32) -> Result<()> {
        let seg = match &mut self.allocator {
            Some(a) => a.allocate_segment(minimum_size),
            None => unreachable!(),
        };
        self.segments.push(BuilderSegment {
            ptr: seg.0,
            capacity: seg.1,
            allocated: 0,
        });
        Ok(())
    }

    fn allocate(&mut self, segment_id: u32, amount: WordCount32) -> Option<u32> {
        let seg = &mut self.segments[segment_id as usize];
        if amount > seg.capacity - seg.allocated {
            None
        } else {
            let result = seg.allocated;
            seg.allocated += amount;
            Some(result)
        }
    }

    fn allocate_anywhere(&mut self, amount: u32) -> (SegmentId, u32) {
        // first try the existing segments, then try allocating a new segment.
        let allocated_len = self.segments.len() as u32;
        for segment_id in 0..allocated_len {
            if let Some(idx) = self.allocate(segment_id, amount) {
                return (segment_id, idx);
            }
        }

        // Need to allocate a new segment.

        self.allocate_segment(amount).expect("allocate new segment");
        (
            allocated_len,
            self.allocate(allocated_len, amount)
                .expect("use freshly-allocated segment"),
        )
    }

    fn deallocate_all(&mut self) {
        if let Some(a) = &mut self.allocator {
            #[cfg(feature = "alloc")]
            for seg in &self.segments {
                unsafe {
                    a.deallocate_segment(seg.ptr, seg.capacity, seg.allocated);
                }
            }

            #[cfg(not(feature = "alloc"))]
            if let Some(seg) = &self.segments.segment {
                unsafe {
                    a.deallocate_segment(seg.ptr, seg.capacity, seg.allocated);
                }
            }
        }
    }

    fn get_segment_mut(&mut self, id: u32) -> (*mut u8, u32) {
        let seg = &self.segments[id as usize];
        (seg.ptr, seg.capacity)
    }
}

unsafe impl<A> BuilderArena for BuilderArenaImpl<A>
where
    A: Allocator,
{
    fn allocate(&mut self, segment_id: u32, amount: WordCount32) -> Option<u32> {
        self.inner.allocate(segment_id, amount)
    }

    fn allocate_anywhere(&mut self, amount: u32) -> (SegmentId, u32) {
        self.inner.allocate_anywhere(amount)
    }

    fn get_segment_mut(&mut self, id: u32) -> (*mut u8, u32) {
        self.inner.get_segment_mut(id)
    }

    fn as_reader(&self) -> &dyn ReaderArena {
        self
    }
}

impl<A> Drop for BuilderArenaImplInner<A>
where
    A: Allocator,
{
    fn drop(&mut self) {
        self.deallocate_all()
    }
}

pub struct NullArena;

unsafe impl ReaderArena for NullArena {
    fn get_segment(&self, _id: u32) -> Result<(*const u8, u32)> {
        Err(Error::from_kind(ErrorKind::TriedToReadFromNullArena))
    }

    unsafe fn check_offset(
        &self,
        _segment_id: u32,
        start: *const u8,
        offset_in_words: i32,
    ) -> Result<*const u8> {
        unsafe { Ok(start.add(offset_in_words as usize * BYTES_PER_WORD)) }
    }

    fn contains_interval(&self, _id: u32, _start: *const u8, _size: usize) -> Result<()> {
        Ok(())
    }

    fn amplified_read(&self, _virtual_amount: u64) -> Result<()> {
        Ok(())
    }

    fn nesting_limit(&self) -> i32 {
        0x7fffffff
    }

    fn size_in_words(&self) -> usize {
        0
    }
}

/// An arena designed for the specific case of reading messages from single-segment
/// `Word` arrays in generated code, including constants and raw schema nodes. Performs
/// bounds checking, so its constructor does not need to be marked `unsafe`. Does
/// *not* enforce a read limit or a nesting limit.
pub struct GeneratedCodeArena {
    words: &'static [crate::Word],
}

impl GeneratedCodeArena {
    pub const fn new(words: &'static [crate::Word]) -> Self {
        assert!((words.len() as u64) < u32::MAX as u64);
        Self { words }
    }
}

unsafe impl ReaderArena for GeneratedCodeArena {
    fn get_segment(&self, id: u32) -> Result<(*const u8, u32)> {
        if id == 0 {
            Ok((self.words.as_ptr() as *const _, self.words.len() as u32))
        } else {
            Err(Error::from_kind(ErrorKind::InvalidSegmentId(id)))
        }
    }

    fn contains_interval(&self, id: u32, start: *const u8, size_in_words: usize) -> Result<()> {
        let (segment_start, segment_len) = self.get_segment(id)?;
        let this_start: usize = segment_start as usize;
        let this_size: usize = segment_len as usize * BYTES_PER_WORD;
        let start = start as usize;
        let size = size_in_words * BYTES_PER_WORD;

        if !(start >= this_start && start - this_start + size <= this_size) {
            Err(Error::from_kind(
                ErrorKind::MessageContainsOutOfBoundsPointer,
            ))
        } else {
            Ok(())
        }
    }

    fn amplified_read(&self, _virtual_amount: u64) -> Result<()> {
        Ok(())
    }

    fn nesting_limit(&self) -> i32 {
        0x7fffffff
    }

    fn size_in_words(&self) -> usize {
        self.words.len()
    }
}
