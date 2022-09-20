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

use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::slice;
use core::u64;

use crate::private::units::*;
use crate::private::read_limiter::ReadLimiter;
use crate::message;
use crate::message::{Allocator, ReaderSegments};
use crate::{Error, OutputSegments, Result};

pub type SegmentId = u32;

pub trait ReaderArena {
    // return pointer to start of segment, and number of words in that segment
    fn get_segment(&self, id: u32) -> Result<(*const u8, u32)>;

    fn check_offset(&self, segment_id: u32, start: *const u8, offset_in_words: i32) -> Result<*const u8>;
    fn contains_interval(&self, segment_id: u32, start: *const u8, size: usize) -> Result<()>;
    fn amplified_read(&self, virtual_amount: u64) -> Result<()>;

    fn nesting_limit(&self) -> i32;

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

impl <S> ReaderArenaImpl <S> where S: ReaderSegments {
    pub fn new(segments: S,
               options: message::ReaderOptions)
               -> Self
    {
        let limiter = ReadLimiter::new(options.traversal_limit_in_words);
        ReaderArenaImpl {
            segments,
            read_limiter: limiter,
            nesting_limit: options.nesting_limit,
        }
    }

    pub fn into_segments(self) -> S {
        self.segments
    }
}

impl <S> ReaderArena for ReaderArenaImpl<S> where S: ReaderSegments {
    fn get_segment(&self, id: u32) -> Result<(*const u8, u32)> {
        match self.segments.get_segment(id) {
            Some(seg) => {
                #[cfg(not(feature = "unaligned"))]
                {
                    if seg.as_ptr() as usize % BYTES_PER_WORD != 0 {
                        return Err(Error::failed(
                            String::from("Detected unaligned segment. You must either ensure all of your \
                                          segments are 8-byte aligned, or you must enable the \"unaligned\" \
                                          feature in the capnp crate")))
                    }
                }

                Ok((seg.as_ptr(), (seg.len() / BYTES_PER_WORD) as u32))
            }
            None => Err(Error::failed(format!("Invalid segment id: {}", id))),
        }
    }

    fn check_offset(&self, segment_id: u32, start: *const u8, offset_in_words: i32) -> Result<*const u8> {
        let (segment_start, segment_len) = self.get_segment(segment_id)?;
        let this_start: usize = segment_start as usize;
        let this_size: usize = segment_len as usize * BYTES_PER_WORD;
        let offset: i64 = offset_in_words as i64 * BYTES_PER_WORD as i64;
        let start_idx = start as usize;
        if start_idx < this_start || ((start_idx - this_start) as i64 + offset) as usize > this_size {
            Err(Error::failed(String::from("message contained out-of-bounds pointer")))
        } else {
            unsafe { Ok(start.offset(offset as isize)) }
        }
    }

    fn contains_interval(&self, id: u32, start: *const u8, size_in_words: usize) -> Result<()> {
        let (segment_start, segment_len) = self.get_segment(id)?;
        let this_start: usize = segment_start as usize;
        let this_size: usize = segment_len as usize * BYTES_PER_WORD;
        let start = start as usize;
        let size = size_in_words * BYTES_PER_WORD;

        if !(start >= this_start && start - this_start + size <= this_size) {
            Err(Error::failed(String::from("message contained out-of-bounds pointer")))
        } else {
            self.read_limiter.can_read(size_in_words)
        }
    }

    fn amplified_read(&self, virtual_amount: u64) -> Result<()> {
        self.read_limiter.can_read(virtual_amount as usize)
    }

    fn nesting_limit(&self) -> i32 { self.nesting_limit }
}

pub trait BuilderArena: ReaderArena {
    // These methods all take an immutable &self because otherwise a StructBuilder<'a>
    // would need a `&'a mut BuilderArena` and `StructBuilder::borrow()` would
    // have lifetime issues. (If `'a: 'b`, then a `&'a (BuilderArena + 'a)` can be
    // converted to a `&'b (BuilderArena + 'b)`, but a `&'a mut (BuilderArena + 'a)`
    // *cannot* be converted to a `&'b mut (BuilderArena + 'b)`. See some discussion here:
    // https://botbot.me/mozilla/rust/2017-01-31/?msg=80228117&page=19 .)
    fn allocate(&self, segment_id: u32, amount: WordCount32) -> Option<u32>;
    fn allocate_anywhere(&self, amount: u32) -> (SegmentId, u32);
    fn get_segment_mut(&self, id: u32) -> (*mut u8, u32);

    fn as_reader(&self) -> &dyn ReaderArena;
}

struct BuilderSegment {
    ptr: *mut u8,
    capacity: u32, // in words
    allocated: u32, // in words
}

pub struct BuilderArenaImplInner<A> where A: Allocator {
    allocator: Option<A>, // None if has already be deallocated.

    // TODO(perf): Try using smallvec to avoid heap allocations in the single-segment case?
    segments: Vec<BuilderSegment>,
}

pub struct BuilderArenaImpl<A> where A: Allocator {
    inner: RefCell<BuilderArenaImplInner<A>>
}

impl <A> BuilderArenaImpl<A> where A: Allocator {
    pub fn new(allocator: A) -> Self {
        BuilderArenaImpl {
            inner: RefCell::new(BuilderArenaImplInner {
                allocator: Some(allocator),
                segments: Vec::new(),
            }),
        }
    }

    /// Allocates a new segment with capacity for at least `minimum_size` words.
    pub fn allocate_segment(&self, minimum_size: u32) -> Result<()> {
        self.inner.borrow_mut().allocate_segment(minimum_size)
    }

    pub fn get_segments_for_output(&self) -> OutputSegments<'_> {
        let reff = self.inner.borrow();
        if reff.segments.len() == 1 {
            let seg = &reff.segments[0];

            // The user must mutably borrow the `message::Builder` to be able to modify segment memory.
            // No such borrow will be possible while `self` is still immutably borrowed from this method,
            // so returning this slice is safe.
            let slice = unsafe { slice::from_raw_parts(seg.ptr as *const _, seg.allocated as usize * BYTES_PER_WORD) };
            OutputSegments::SingleSegment([slice])
        } else {
            let mut v = Vec::with_capacity(reff.segments.len());
            for seg in &reff.segments {
                // See safety argument in above branch.
                let slice = unsafe { slice::from_raw_parts(seg.ptr as *const _, seg.allocated as usize * BYTES_PER_WORD) };
                v.push(slice);
            }
            OutputSegments::MultiSegment(v)
        }
    }

    pub fn len(&self) -> usize {
        self.inner.borrow().segments.len()
    }

    pub fn into_allocator(self) -> A {
        let mut inner = self.inner.into_inner();
        inner.deallocate_all();
        inner.allocator.take().unwrap()
    }
}

impl <A> ReaderArena for BuilderArenaImpl<A> where A: Allocator {
    fn get_segment(&self, id: u32) -> Result<(*const u8, u32)> {
        let borrow = self.inner.borrow();
        let seg = &borrow.segments[id as usize];
        Ok((seg.ptr, seg.allocated))
    }

    fn check_offset(&self, _segment_id: u32, start: *const u8, offset_in_words: i32) -> Result<*const u8> {
        unsafe { Ok(start.offset((offset_in_words as i64 * BYTES_PER_WORD as i64) as isize)) }
    }

    fn contains_interval(&self, _id: u32, _start: *const u8, _size: usize) -> Result<()> {
        Ok(())
    }

    fn amplified_read(&self, _virtual_amount: u64) -> Result<()> {
        Ok(())
    }

    fn nesting_limit(&self) -> i32 { 0x7fffffff }
}

impl <A> BuilderArenaImplInner<A> where A: Allocator {
    /// Allocates a new segment with capacity for at least `minimum_size` words.
    fn allocate_segment(&mut self, minimum_size: WordCount32) -> Result<()> {
        let seg = match self.allocator {
            Some(ref mut a) => a.allocate_segment(minimum_size),
            None => unreachable!(),
        };
        self.segments.push(BuilderSegment { ptr: seg.0, capacity: seg.1, allocated: 0});
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
        for segment_id in 0.. allocated_len {
            if let Some(idx) = self.allocate(segment_id, amount) {
                return (segment_id, idx);
            }
        }

        // Need to allocate a new segment.

        self.allocate_segment(amount).expect("allocate new segment");
        (allocated_len,
         self.allocate(allocated_len, amount).expect("use freshly-allocated segment"))
    }

    fn deallocate_all(&mut self) {
        if let Some(ref mut a) = self.allocator {
            for seg in &self.segments {
                a.deallocate_segment(seg.ptr, seg.capacity, seg.allocated);
            }
        }
    }

    fn get_segment_mut(&mut self, id: u32) -> (*mut u8, u32) {
        let seg = &self.segments[id as usize];
        (seg.ptr, seg.capacity)
    }
}

impl <A> BuilderArena for BuilderArenaImpl<A> where A: Allocator {
    fn allocate(&self, segment_id: u32, amount: WordCount32) -> Option<u32> {
        self.inner.borrow_mut().allocate(segment_id, amount)
    }

    fn allocate_anywhere(&self, amount: u32) -> (SegmentId, u32) {
        self.inner.borrow_mut().allocate_anywhere(amount)
    }

    fn get_segment_mut(&self, id: u32) -> (*mut u8, u32) {
        self.inner.borrow_mut().get_segment_mut(id)
    }

    fn as_reader(&self) -> &dyn ReaderArena {
        self
    }
}

impl <A> Drop for BuilderArenaImplInner<A> where A: Allocator {
    fn drop(&mut self) {
        self.deallocate_all()
    }
}

pub struct NullArena;

impl ReaderArena for NullArena {
    fn get_segment(&self, _id: u32) -> Result<(*const u8, u32)> {
        Err(Error::failed(String::from("tried to read from null arena")))
    }

    fn check_offset(&self, _segment_id: u32, start: *const u8, offset_in_words: i32) -> Result<*const u8> {
        unsafe { Ok(start.offset((offset_in_words as usize * BYTES_PER_WORD)as isize)) }
    }

    fn contains_interval(&self, _id: u32, _start: *const u8, _size: usize) -> Result<()> {
        Ok(())
    }

    fn amplified_read(&self, _virtual_amount: u64) -> Result<()> {
        Ok(())
    }

    fn nesting_limit(&self) -> i32 { 0x7fffffff }
}

impl BuilderArena for NullArena {
    fn allocate(&self, _segment_id: u32, _amount: WordCount32) -> Option<u32> {
        None
    }

    fn allocate_anywhere(&self, _amount: u32) -> (SegmentId, u32) {
        panic!("tried to allocate from a null arena")
    }

    fn get_segment_mut(&self, _id: u32) -> (*mut u8, u32) {
        (core::ptr::null_mut(), 0)
    }

    fn as_reader(&self) -> &dyn ReaderArena {
        self
    }
}

