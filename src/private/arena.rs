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

use private::capability::ClientHook;
use private::units::*;
use message;
use {Error, Result, Word};

pub use self::FirstSegment::{NumWords, ZeroedWords};

pub type SegmentId = u32;

pub struct SegmentReader {
    pub arena : ArenaPtr,
    pub ptr : *const Word,
    pub size : WordCount32,
    pub read_limiter : ::std::rc::Rc<ReadLimiter>,
}

unsafe impl Send for SegmentReader {}

impl SegmentReader {

    #[inline]
    pub unsafe fn get_start_ptr(&self) -> *const Word {
        self.ptr.offset(0)
    }

    #[inline]
    pub fn contains_interval(&self, from : *const Word, to : *const Word) -> bool {
        let this_begin : usize = self.ptr as usize;
        let this_end : usize = unsafe { self.ptr.offset(self.size as isize) as usize };
        return from as usize >= this_begin && to as usize <= this_end && from as usize <= to as usize &&
            self.read_limiter.can_read((to as usize - from as usize) as u64 / BYTES_PER_WORD as u64);
    }

    #[inline]
    pub fn amplified_read(&self, virtual_amount : u64) -> bool {
        return self.read_limiter.can_read(virtual_amount);
    }
}

pub struct SegmentBuilder {
    pub reader : SegmentReader,
    pub id : SegmentId,
    pos : *mut Word,
}

unsafe impl Send for SegmentBuilder {}

impl SegmentBuilder {

    pub fn new(arena : *mut BuilderArena,
               limiter : ::std::rc::Rc<ReadLimiter>,
               id : SegmentId,
               ptr : *mut Word,
               size : WordCount32) -> SegmentBuilder {
        SegmentBuilder {
            reader : SegmentReader {
                arena : ArenaPtr::Builder(arena),
                ptr : unsafe {::std::mem::transmute(ptr)},
                size : size,
                read_limiter: limiter,
            },
            id : id,
            pos : ptr,
        }
    }

    pub fn get_word_offset_to(&mut self, ptr : *mut Word) -> WordCount32 {
        let this_addr : usize = self.reader.ptr as usize;
        let ptr_addr : usize = ptr as usize;
        assert!(ptr_addr >= this_addr);
        let result = (ptr_addr - this_addr) / BYTES_PER_WORD;
        return result as u32;
    }

    #[inline]
    pub fn current_size(&self) -> WordCount32 {
        ((self.pos as usize - self.reader.ptr as usize) / BYTES_PER_WORD) as u32
    }

    #[inline]
    pub fn allocate(&mut self, amount : WordCount32) -> Option<*mut Word> {
        if amount > self.reader.size - self.current_size() {
            return None;
        } else {
            let result = self.pos;
            self.pos = unsafe { self.pos.offset(amount as isize) };
            return Some(result);
        }
    }

    #[inline]
    pub fn get_ptr_unchecked(&self, offset : WordCount32) -> *mut Word {
        unsafe {
            ::std::mem::transmute(self.reader.ptr.offset(offset as isize))
        }
    }

    #[inline]
    pub fn get_segment_id(&self) -> SegmentId { self.id }

    #[inline]
    pub fn get_arena(&self) -> *mut BuilderArena {
        match self.reader.arena {
            ArenaPtr::Builder(b) => b,
            _ => unreachable!()
        }
    }

    pub unsafe fn currently_allocated(&self) -> &'static [Word] {
        ::std::slice::from_raw_parts(self.get_ptr_unchecked(0), self.current_size() as usize)
    }
}

pub struct ReadLimiter {
    pub limit : ::std::cell::RefCell<u64>,
}

impl ReadLimiter {
    pub fn new(limit : u64) -> ReadLimiter {
        ReadLimiter { limit : ::std::cell::RefCell::new(limit) }
    }

    #[inline]
    pub fn can_read(&self, amount : u64) -> bool {
        let current = *self.limit.borrow();
        if amount > current {
            // TODO arena->reportReadLimitReached()
            return false;
        } else {
            *self.limit.borrow_mut() = current - amount;
            return true;
        }
    }
}

pub struct ReaderArena {
    //    message : *message::MessageReader<'a>,
    pub segment0 : SegmentReader,

    pub more_segments : Vec<SegmentReader>,
    //XXX should this be a map as in capnproto-c++?

    pub cap_table : Vec<Option<Box<ClientHook+Send>>>,

    pub read_limiter : ::std::rc::Rc<ReadLimiter>,
}

unsafe impl Send for ReaderArena {}

impl ReaderArena {
    pub fn new(segments : &[&[Word]], options : message::ReaderOptions) -> Box<ReaderArena> {
        assert!(segments.len() > 0);
        let limiter = ::std::rc::Rc::new(ReadLimiter::new(options.traversal_limit_in_words));
        let mut arena = Box::new(ReaderArena {
            segment0 : SegmentReader {
                arena : ArenaPtr::Null,
                ptr : unsafe { segments[0].get_unchecked(0) },
                size : segments[0].len() as u32,
                read_limiter : limiter.clone(),
            },
            more_segments : Vec::new(),
            cap_table : Vec::new(),
            read_limiter : limiter.clone(),
        });


        let arena_ptr = ArenaPtr::Reader (&*arena);

        arena.segment0.arena = arena_ptr;

        if segments.len() > 1 {
            let mut more_segment_readers = Vec::new();
            for segment in segments[1 ..].iter() {
                let segment_reader = SegmentReader {
                    arena : arena_ptr,
                    ptr : unsafe { segment.get_unchecked(0) },
                    size : segment.len() as u32,
                    read_limiter : limiter.clone(),
                };
                more_segment_readers.push(segment_reader);
            }
            arena.more_segments = more_segment_readers;
        }

        arena
    }

    pub fn try_get_segment(&self, id : SegmentId) -> Result<*const SegmentReader> {
        if id == 0 {
            return Ok(&self.segment0);
        } else if ((id - 1) as usize) < self.more_segments.len() {
            unsafe { Ok(self.more_segments.get_unchecked(id as usize - 1)) }
        } else {
            Err(Error::new_decode_error("Invalid segment id.", Some(format!("{}", id))))
        }
    }

    #[inline]
    pub fn init_cap_table(&mut self, cap_table : Vec<Option<Box<ClientHook+Send>>>) {
        self.cap_table = cap_table;
    }

}

pub struct BuilderArena {
    pub segment0 : SegmentBuilder,
    pub more_segments : Vec<Box<SegmentBuilder>>,
    pub allocation_strategy : message::AllocationStrategy,
    pub owned_memory : Vec<Vec<Word>>,
    pub next_size : u32,
    pub cap_table : Vec<Option<Box<ClientHook+Send>>>,
    pub dummy_limiter : ::std::rc::Rc<ReadLimiter>,
}

pub enum FirstSegment<'a> {
    NumWords(u32),
    ZeroedWords(&'a mut [Word])
}

impl BuilderArena {

    pub fn new(allocation_strategy : message::AllocationStrategy,
               first_segment : FirstSegment) -> Box<BuilderArena> {
        let limiter = ::std::rc::Rc::new(ReadLimiter::new(::std::u64::MAX));

        let (first_segment, num_words, owned_memory) : (*mut Word, u32, Vec<Vec<Word>>) =
            match first_segment {
                NumWords(n) => {
                    let mut vec = Word::allocate_zeroed_vec(n as usize);
                    let ptr : *mut Word = vec.as_mut_ptr();
                    (ptr, n, vec![vec])
                }
                ZeroedWords(w) => (w.as_mut_ptr(), w.len() as u32, Vec::new())
            };


        let mut result = Box::new(BuilderArena {
            segment0 : SegmentBuilder {
                reader : SegmentReader {
                    ptr : first_segment,
                    size : num_words,
                    arena : ArenaPtr::Null,
                    read_limiter : limiter.clone()},
                id : 0,
                pos : first_segment,
            },
            more_segments : Vec::new(),
            allocation_strategy : allocation_strategy,
            owned_memory : owned_memory,
            next_size : num_words,
            cap_table : Vec::new(),
            dummy_limiter : limiter,
        });

        let arena_ptr : *mut BuilderArena = { let ref mut ptr = *result; ptr};
        result.segment0.reader.arena = ArenaPtr::Builder(arena_ptr);

        result
    }

    pub fn allocate_owned_memory(&mut self, minimum_size : WordCount32) -> (*mut Word, WordCount32) {
        let size = ::std::cmp::max(minimum_size, self.next_size);
        let mut new_words = Word::allocate_zeroed_vec(size as usize);
        let ptr = new_words.as_mut_ptr();

        self.owned_memory.push(new_words);

        match self.allocation_strategy {
            message::AllocationStrategy::GrowHeuristically => { self.next_size += size; }
            _ => { }
        }
        (ptr, size)
    }

    #[inline]
    pub fn allocate(&mut self, amount : WordCount32) -> (*mut SegmentBuilder, *mut Word) {
        unsafe {
            match self.segment0.allocate(amount) {
                Some(result) => { return (&mut self.segment0, result) }
                None => {}
            }

            //# Need to fall back to additional segments.

            let id = {
                let len = self.more_segments.len();
                if len == 0 { 1 }
                else {
                    let result_ptr : *mut SegmentBuilder = &mut *self.more_segments[len-1];
                    match self.more_segments[len - 1].allocate(amount) {
                        Some(result) => { return (result_ptr, result) }
                        None => { len + 1 }
                    }
                }};

            let (words, size) = self.allocate_owned_memory(amount);
            let mut new_builder = Box::new(SegmentBuilder::new(self, self.dummy_limiter.clone(),
                                                               id as u32, words, size));
            let builder_ptr : *mut SegmentBuilder = &mut *new_builder;

            self.more_segments.push(new_builder);

            (builder_ptr, (*builder_ptr).allocate(amount).unwrap() )
        }
    }

    pub fn get_segment(&mut self, id : SegmentId) -> Result<*mut SegmentBuilder> {
        if id == 0 {
            Ok(&mut self.segment0)
        } else if ((id - 1) as usize) < self.more_segments.len() {
            Ok(&mut *self.more_segments[(id - 1) as usize])
        } else {
            Err(Error::new_decode_error("Invalid segment id.", Some(format!("{}", id))))
        }
    }

    pub fn get_segments_for_output<'a>(&'a self) -> Vec<&'a [Word]> {
        let mut segments = Vec::with_capacity(self.more_segments.len() + 1);
        unsafe {
            segments.push(self.segment0.currently_allocated());
            segments.extend(self.more_segments.iter().map(|segment| segment.currently_allocated()));
        }
        segments
    }

    pub fn get_cap_table<'a>(&'a self) -> &'a [Option<Box<ClientHook+Send>>] {
        &self.cap_table
    }

    pub fn inject_cap(&mut self, cap : Box<ClientHook+Send>) -> u32 {
        self.cap_table.push(Some(cap));
        self.cap_table.len() as u32 - 1
    }
}

#[derive(Clone, Copy)]
pub enum ArenaPtr {
    Reader(*const ReaderArena),
    Builder(*mut BuilderArena),
    Null
}

impl ArenaPtr {
    pub fn try_get_segment(&self, id : SegmentId) -> Result<*const SegmentReader> {
        unsafe {
            match self {
                &ArenaPtr::Reader(reader) => {
                    (&*reader).try_get_segment(id)
                }
                &ArenaPtr::Builder(builder) => {
                    if id == 0 {
                        Ok(&(*builder).segment0.reader)
                    } else if ((id - 1) as usize) < (*builder).more_segments.len() {
                        Ok(&(*builder).more_segments[(id - 1) as usize].reader)
                    } else {
                        Err(Error::new_decode_error("Invalid segment id.", Some(format!("{}", id))))
                    }
                }
                &ArenaPtr::Null => {
                    Err(Error::new_decode_error("Null arena.", None))
                }
            }
        }
    }

    pub fn extract_cap(&self, index : usize) -> Option<Box<ClientHook+Send>> {
        unsafe {
            match self {
                &ArenaPtr::Reader(reader) => {
                    if index < (*reader).cap_table.len() {
                        match (*reader).cap_table[index] {
                            Some( ref hook ) => { Some(hook.copy()) }
                            None => {
                                None
                            }
                        }
                    } else {
                        None
                    }
                }
                &ArenaPtr::Builder(builder) => {
                    if index < (*builder).cap_table.len() {
                        match (*builder).cap_table[index] {
                            Some( ref hook ) => { Some(hook.copy()) }
                            None => {
                                None
                            }
                        }
                    } else {
                        None
                    }
                }
                &ArenaPtr::Null => {
                    panic!();
                }
            }
        }
    }
}

#[cfg(test)]
mod test {

    use quickcheck::quickcheck;

    use message::AllocationStrategy;
    use super::{BuilderArena, FirstSegment};

    #[test]
    fn check_builder_arena_output_segments() {
        fn output_segments(allocations: Vec<u32>,
                           strategy: AllocationStrategy,
                           segment0_size: u32) -> bool {
            let mut arena = BuilderArena::new(strategy, FirstSegment::NumWords(segment0_size));

            for &allocation in allocations.iter() {
                arena.allocate(allocation);
            }

            let expected: usize = allocations.into_iter().fold(0, |a, b| a + b as usize);
            let actual: usize = arena.get_segments_for_output().into_iter().fold(0, |acc, segment| acc + segment.len());
            expected == actual
        }

        quickcheck(output_segments as fn(Vec<u32>, AllocationStrategy, u32) -> bool);
    }
}
