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

use alloc::vec::Vec;
use core::cell::{Cell};
use core::slice;
use core::u64;

use crate::private::units::*;
use crate::message;
use crate::message::{Allocator, ReaderSegments};
use crate::{Error, OutputSegments, Result};

pub type SegmentId = u32;

pub trait ReadLimiter {
    fn can_read(&self, amount: u64) -> Result<()>;
}

pub struct ReadLimiterImpl {
    pub limit: Cell<u64>,
}

impl ReadLimiterImpl {
    pub fn new(limit: u64) -> Self {
        ReadLimiterImpl { limit: Cell::new(limit) }
    }
}

impl ReadLimiter for ReadLimiterImpl {
   fn can_read(&self, amount: u64) -> Result<()> {
        let current = self.limit.get();
        if amount > current {
            Err(Error::failed(format!("read limit exceeded")))
        } else {
            self.limit.set(current - amount);
            Ok(())
        }
    }
}

pub trait ReaderArena {
    type Alignment: crate::private::primitive::Alignment;

    // return pointer to start of segment, and number of words in that segment
    fn get_segment(&self, id: u32) -> Result<(*const u8, u32)>;

    fn check_offset(&self, segment_id: u32, start: *const u8, offset_in_words: i32) -> Result<*const u8>;
    fn contains_interval(&self, segment_id: u32, start: *const u8, size: usize) -> Result<()>;
    fn amplified_read(&self, virtual_amount: u64) -> Result<()>;

    fn nesting_limit(&self) -> i32;

    // TODO(version 0.9): Consider putting extract_cap(), inject_cap(), drop_cap() here
    //   and on message::Reader. Then we could get rid of Imbue and ImbueMut, and
    //   layout::StructReader, layout::ListReader, etc. could drop their `cap_table` fields.
}

pub struct ReaderArenaImpl<S, L, A> {
    alignment: core::marker::PhantomData<A>,
    segments: S,
    read_limiter: L,
    nesting_limit: i32,
}

impl <S> ReaderArenaImpl <S, ReadLimiterImpl, crate::private::primitive::Unaligned> where S: ReaderSegments {
    pub fn new<A>(segments: S,
                  options: message::ReaderOptions)
                  -> ReaderArenaImpl<S, ReadLimiterImpl, A>
        where A: crate::private::primitive::Alignment
    {
        let limiter = ReadLimiterImpl::new(options.traversal_limit_in_words);
        ReaderArenaImpl {
            alignment: core::marker::PhantomData,
            segments: segments,
            read_limiter: limiter,
            nesting_limit: options.nesting_limit,
        }
    }

    pub fn into_segments(self) -> S {
        self.segments
    }
}

impl <S, L, A> ReaderArena for ReaderArenaImpl<S, L, A> where S: ReaderSegments, L: ReadLimiter, A: crate::private::primitive::Alignment {
    type Alignment = A;

    fn get_segment<'a>(&'a self, id: u32) -> Result<(*const u8, u32)> {
        match self.segments.get_segment(id) {
            Some(seg) => {
                #[cfg(not(feature = "unaligned"))]
                {
                    if seg.as_ptr() as usize % BYTES_PER_WORD != 0 {
                        return Err(Error::failed(
                            format!("Detected unaligned segment. You must either ensure all of your \
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
        let offset: i64 = offset_in_words as i64 * BYTES_PER_WORD as i64;
        let (segment_start, segment_len) = self.get_segment(segment_id)?;
        let this_start: usize = segment_start as usize;
        let this_size: usize = segment_len as usize * BYTES_PER_WORD;
        let start_idx = start as usize;
        if start_idx < this_start || ((start_idx - this_start) as i64 + offset) as usize > this_size {
            return Err(Error::failed(format!("message contained out-of-bounds pointer")));
        }

        unsafe { Ok(start.offset(offset as isize)) }
    }

    fn contains_interval(&self, id: u32, start: *const u8, size_in_words: usize) -> Result<()> {
        let (segment_start, segment_len) = self.get_segment(id)?;
        let this_start: usize = segment_start as usize;
        let this_size: usize = segment_len as usize * BYTES_PER_WORD;
        let start = start as usize;
        let size = size_in_words * BYTES_PER_WORD;

        if !(start >= this_start && start - this_start + size <= this_size) {
            Err(Error::failed(format!("message contained out-of-bounds pointer")))
        } else {
            self.read_limiter.can_read(size_in_words as u64)
        }
    }

    fn amplified_read(&self, virtual_amount: u64) -> Result<()> {
        self.read_limiter.can_read(virtual_amount)
    }

    fn nesting_limit(&self) -> i32 {
        self.nesting_limit
    }
}

pub trait BuilderArena: ReaderArena {
    fn allocate(&mut self, segment_id: u32, amount: WordCount32) -> Option<u32>;
    fn allocate_anywhere(&mut self, amount: u32) -> (SegmentId, u32);
    fn get_segment_mut(&mut self, id: u32) -> (*mut u8, u32);
}

struct BuilderSegment {
    ptr: *mut u8,
    capacity: u32, // in words
    allocated: u32, // in words
}

pub struct BuilderArenaImpl<A> where A: Allocator {
    allocator: Option<A>, // None if has already be deallocated.

    // TODO(perf): Try using smallvec to avoid heap allocations in the single-segment case?
    segments: Vec<BuilderSegment>,
}

impl <A> BuilderArenaImpl<A> where A: Allocator {
    pub fn new(allocator: A) -> Self {
        BuilderArenaImpl {
            allocator: Some(allocator),
            segments: Vec::new(),
        }
    }

    pub fn get_segments_for_output<'a>(&'a self) -> OutputSegments<'a> {
        if self.segments.len() == 1 {
            let seg = &self.segments[0];

            // The user must mutably borrow the `message::Builder` to be able to modify segment memory.
            // No such borrow will be possible while `self` is still immutably borrowed from this method,
            // so returning this slice is safe.
            let slice = unsafe { slice::from_raw_parts(seg.ptr as *const _, seg.allocated as usize * BYTES_PER_WORD) };
            OutputSegments::SingleSegment([slice])
        } else {
            let mut v = Vec::with_capacity(self.segments.len());
            for ref seg in &self.segments {
                // See safety argument in above branch.
                let slice = unsafe { slice::from_raw_parts(seg.ptr as *const _, seg.allocated as usize * BYTES_PER_WORD) };
                v.push(slice);
            }
            OutputSegments::MultiSegment(v)
        }
    }

    pub fn len(&self) -> usize {
        self.segments.len()
    }

    pub fn into_allocator(mut self) -> A {
        self.deallocate_all();
        self.allocator.take().unwrap()
    }
}

impl <A> ReaderArena for BuilderArenaImpl<A> where A: Allocator {
    type Alignment = crate::private::primitive::Unaligned; // TODO

    fn get_segment(&self, id: u32) -> Result<(*const u8, u32)> {
        let seg = &self.segments[id as usize];
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

    fn nesting_limit(&self) -> i32 {
        0x7fffffff
    }
}

impl <A> BuilderArenaImpl<A> where A: Allocator {
    pub fn allocate_segment(&mut self, minimum_size: WordCount32) -> Result<()> {
        let seg = match self.allocator {
            Some(ref mut a) => a.allocate_segment(minimum_size),
            None => unreachable!(),
        };
        self.segments.push(BuilderSegment { ptr: seg.0, capacity: seg.1, allocated: 0});
        Ok(())
    }

    fn allocate(&mut self, segment_id: u32, amount: WordCount32) -> Option<u32> {
        let ref mut seg = &mut self.segments[segment_id as usize];
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
            match self.allocate(segment_id, amount) {
                Some(idx) => return (segment_id, idx),
                None => (),
            }
        }

        // Need to allocate a new segment.

        self.allocate_segment(amount).expect("allocate new segment");
        (allocated_len,
         self.allocate(allocated_len, amount).expect("use freshly-allocated segment"))
    }

    fn deallocate_all(&mut self) {
        if let Some(ref mut a) = self.allocator {
            for ref seg in &self.segments {
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
    fn allocate(&mut self, segment_id: u32, amount: WordCount32) -> Option<u32> {
        self.allocate(segment_id, amount)
    }

    fn allocate_anywhere(&mut self, amount: u32) -> (SegmentId, u32) {
        self.allocate_anywhere(amount)
    }

    fn get_segment_mut(&mut self, id: u32) -> (*mut u8, u32) {
        self.get_segment_mut(id)
    }
}

impl <A> Drop for BuilderArenaImpl<A> where A: Allocator {
    fn drop(&mut self) {
        self.deallocate_all()
    }
}

pub struct NullArena;

impl ReaderArena for NullArena {
    type Alignment = crate::private::primitive::Unaligned;

    fn get_segment(&self, _id: u32) -> Result<(*const u8, u32)> {
        Err(Error::failed(format!("tried to read from null arena")))
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

    fn nesting_limit(&self) -> i32 {
        0x7fffffff
    }
}
