/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use common::*;
use message;

pub type SegmentId = u32;

pub struct SegmentReader<'a> {
    messageReader : * message::MessageReader<'a>,
    segment : &'a [Word]
}

impl <'a> SegmentReader<'a> {

    pub unsafe fn get_start_ptr(&self) -> *Word {
        self.segment.unsafe_ref(0)
    }

    pub unsafe fn contains_interval(&self, from : *Word, to : *Word) -> bool {
        let fromAddr : uint = std::cast::transmute(from);
        let toAddr : uint = std::cast::transmute(to);
        let thisBegin : uint = std::cast::transmute(self.segment.unsafe_ref(0));
        let thisEnd : uint = std::cast::transmute(
            self.segment.unsafe_ref(self.segment.len()));
        return (fromAddr >= thisBegin && toAddr <= thisEnd);
        // TODO readLimiter
    }
}

pub struct SegmentBuilder {
    messageBuilder : *mut message::MessageBuilder,
    id : SegmentId,
    ptr : *mut Word,
    pos : WordCount,
    size : WordCount
}

impl SegmentBuilder {

    pub fn new(messageBuilder : *mut message::MessageBuilder,
               size : WordCount) -> SegmentBuilder {
        let idx = unsafe {((*messageBuilder).segments.len() - 1) as SegmentId};
        SegmentBuilder {
            messageBuilder : messageBuilder,
            ptr : unsafe {(*messageBuilder).segments[idx].unsafe_mut_ref(0)},
            id : idx,
            pos : 0,
            size : size
        }
    }

    pub fn get_word_offset_to(&mut self, ptr : *mut Word) -> WordCount {
        let thisAddr : uint = self.ptr.to_uint();
        let ptrAddr : uint = ptr.to_uint();
        assert!(ptrAddr >= thisAddr);
        let result = (ptrAddr - thisAddr) / BYTES_PER_WORD;
        return result;
    }

    pub fn allocate(&mut self, amount : WordCount) -> Option<*mut Word> {
        if (amount > self.size - self.pos) {
            return None;
        } else {
            let result = unsafe { self.ptr.offset(self.pos as int) };
            self.pos += amount;
            return Some(result);
        }
    }

    pub fn available(&self) -> WordCount {
        self.size - self.pos
    }

    #[inline]
    pub unsafe fn get_ptr_unchecked(&mut self, offset : WordCount) -> *mut Word {
        self.ptr.offset(offset as int)
    }

    #[inline]
    pub fn get_segment_id(&self) -> SegmentId { self.id }

    pub fn as_reader<T>(&mut self, f : |&SegmentReader| -> T) -> T {
        unsafe {
            (*self.messageBuilder).as_reader(|messageReader| {
                f(&*messageReader.get_segment_reader(self.id))
            })
        }
    }
}

// ----------------
// The following stuff is currently unused.

pub struct ReaderArena<'a> {
    message : message::MessageReader<'a>,
    segment0 : SegmentReader<'a>,

    moreSegments : Option<~[SegmentReader<'a>]>
    //XXX should this be a map as in capnproto-c++?
}

pub struct BuilderArena {
    message : *message::MessageBuilder,
    segment0 : SegmentBuilder
}

pub enum Arena<'a> {
    Reader_(ReaderArena<'a>),
    Builder_(BuilderArena)
}


impl <'a> Arena<'a>  {
    pub fn try_get_segment(&self, id : SegmentId) -> *SegmentReader<'a> {
        match self {
            &Reader_(ref reader) => {
                if (id == 0) {
                    return std::ptr::to_unsafe_ptr(&reader.segment0);
                } else {
                    match reader.moreSegments {
                        None => {fail!("no segments!")}
                        Some(ref segs) => {
                            unsafe {segs.unsafe_ref(id as uint - 1)}
                        }
                    }
                }
            }
            &Builder_(ref _builder) => {
                fail!()
            }
        }
    }
}
