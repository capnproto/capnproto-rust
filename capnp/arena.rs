/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use common::*;
use common::ptr_sub;
use message;

pub type SegmentId = u32;

pub struct SegmentReader<'a> {
    arena : ArenaPtr<'a>,
    ptr : * Word,
    size : WordCount
}

impl <'a> SegmentReader<'a> {

    #[inline]
    pub unsafe fn get_start_ptr(&self) -> *Word {
        self.ptr.offset(0)
    }

    #[inline]
    pub fn contains_interval(&self, from : *Word, to : *Word) -> bool {
        let thisBegin : uint = self.ptr.to_uint();
        let thisEnd : uint = unsafe { self.ptr.offset(self.size as int).to_uint() };
        return from.to_uint() >= thisBegin && to.to_uint() <= thisEnd && from.to_uint() <= to.to_uint();
        // TODO readLimiter
    }
}

pub struct SegmentBuilder<'a> {
    reader : SegmentReader<'a>,
    id : SegmentId,
    pos : *mut Word,
}

impl <'a> SegmentBuilder<'a> {

    pub fn new<'b>(arena : *mut BuilderArena<'b>,
                   id : SegmentId,
                   ptr : *mut Word,
                   size : WordCount) -> SegmentBuilder<'b> {
        SegmentBuilder {
            reader : SegmentReader {
                arena : BuilderArenaPtr(arena),
                ptr : unsafe {std::cast::transmute(ptr)},
                size : size
            },
            id : id,
            pos : ptr,
        }
    }

    pub fn get_word_offset_to(&mut self, ptr : *mut Word) -> WordCount {
        let thisAddr : uint = self.reader.ptr.to_uint();
        let ptrAddr : uint = ptr.to_uint();
        assert!(ptrAddr >= thisAddr);
        let result = (ptrAddr - thisAddr) / BYTES_PER_WORD;
        return result;
    }

    #[inline]
    pub fn current_size(&self) -> WordCount {
        ptr_sub(self.pos, self.reader.ptr)
    }

    #[inline]
    pub fn allocate(&mut self, amount : WordCount) -> Option<*mut Word> {
        if amount > self.reader.size - self.current_size() {
            return None;
        } else {
            let result = self.pos;
            self.pos = unsafe { self.pos.offset(amount as int) };
            return Some(result);
        }
    }

    #[inline]
    pub unsafe fn get_ptr_unchecked(&mut self, offset : WordCount) -> *mut Word {
        std::cast::transmute_mut_unsafe(self.reader.ptr.offset(offset as int))
    }

    #[inline]
    pub fn get_segment_id(&self) -> SegmentId { self.id }

    #[inline]
    pub fn get_arena(&self) -> *mut BuilderArena<'a> {
        match self.reader.arena {
            BuilderArenaPtr(b) => b,
            _ => unreachable!()
        }
    }
}

pub struct ReaderArena<'a> {
//    message : *message::MessageReader<'a>,
    segment0 : SegmentReader<'a>,

    more_segments : Option<~[SegmentReader<'a>]>
    //XXX should this be a map as in capnproto-c++?
}

pub struct BuilderArena<'a> {
    message : *mut message::MessageBuilder<'a>,
    segment0 : SegmentBuilder<'a>,
    more_segments : Option<~[~SegmentBuilder<'a>]>,
}

impl <'a> BuilderArena<'a> {

    #[inline]
    pub fn allocate(&mut self, amount : WordCount) -> (*mut SegmentBuilder<'a>, *mut Word) {
        unsafe {
            match self.segment0.allocate(amount) {
                Some(result) => { return (std::ptr::to_mut_unsafe_ptr(&mut self.segment0), result) }
                None => {}
            }

            //# Need to fall back to additional segments.

            let id = match self.more_segments {
                None => {
                    self.more_segments = Some(~[]);
                    1
                }
                Some(ref mut msegs) => {
                    let len = msegs.len();
                    let result_ptr = std::ptr::to_mut_unsafe_ptr(msegs[len-1]);
                    match msegs[len - 1].allocate(amount) {
                        Some(result) => { return (result_ptr, result) }
                        None => { len + 1 }
                    }
                }
            };

            let (words, size) = (*self.message).allocate_segment(amount);
            let mut new_builder = ~SegmentBuilder::new(std::ptr::to_mut_unsafe_ptr(self), id as u32, words, size);
            let builder_ptr = std::ptr::to_mut_unsafe_ptr(new_builder);

            match self.more_segments {
                None => fail!("impossible"),
                Some(ref mut msegs) => {
                    msegs.push(new_builder);
                }
            }

            (builder_ptr, (*builder_ptr).allocate(amount).unwrap() )
        }
    }

    pub fn get_segment(&mut self, id : SegmentId) -> *mut SegmentBuilder<'a> {
        if id == 0 {
            std::ptr::to_mut_unsafe_ptr(&mut self.segment0)
        } else {
            match self.more_segments {
                None => fail!("invalid segment id {}", id),
                Some(ref mut msegs) => {
                    std::ptr::to_mut_unsafe_ptr(msegs[id - 1])
                }
            }
        }
    }

    pub fn get_segments_for_output<T>(&self, cont : |&[&[Word]]| -> T) -> T {
        unsafe {
            match self.more_segments {
                None => {
                    std::vec::raw::buf_as_slice::<Word, T>(
                        self.segment0.reader.ptr,
                        self.segment0.current_size(),
                        |v| cont([v]) )
                }
                Some(ref msegs) => {
                    let mut result = ~[];
                    result.push(std::cast::transmute(
                            std::unstable::raw::Slice { data : self.segment0.reader.ptr,
                                                       len : self.segment0.current_size()}));

                    for seg in msegs.iter() {
                        result.push(std::cast::transmute(
                            std::unstable::raw::Slice { data : seg.reader.ptr,
                                                        len : seg.current_size()}));
                    }
                    cont(result)
                }
            }
        }
    }
}

pub enum ArenaPtr<'a> {
    ReaderArenaPtr(*ReaderArena<'a>),
    BuilderArenaPtr(*mut BuilderArena<'a>),
    Null
}

impl <'a> ArenaPtr<'a>  {
    pub fn try_get_segment(&self, id : SegmentId) -> *SegmentReader<'a> {
        unsafe {
            match self {
                &ReaderArenaPtr(reader) => {
                    if id == 0 {
                        return std::ptr::to_unsafe_ptr(&(*reader).segment0);
                    } else {
                        match (*reader).more_segments {
                            None => {fail!("no segments!")}
                            Some(ref segs) => {
                                segs.unsafe_ref(id as uint - 1)
                            }
                        }
                    }
                }
                &BuilderArenaPtr(builder) => {
                    if id == 0 {
                        std::ptr::to_unsafe_ptr(&(*builder).segment0.reader)
                    } else {
                        match (*builder).more_segments {
                            None => {fail!("no more segments!")}
                            Some(ref segs) => {
                               std::ptr::to_unsafe_ptr(&segs[id as uint - 1].reader)
                            }
                        }
                    }
                }
                &Null => {
                    fail!()
                }
            }
        }
    }
}
