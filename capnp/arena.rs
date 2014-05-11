/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use std::vec::Vec;
use libc;
use capability::ClientHook;
use common::*;
use common::ptr_sub;
use message;

pub type SegmentId = u32;

pub struct SegmentReader {
    pub arena : ArenaPtr,
    pub ptr : *Word,
    pub size : WordCount
}

impl SegmentReader {

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

pub struct SegmentBuilder {
    pub reader : SegmentReader,
    pub id : SegmentId,
    pos : *mut Word,
}

impl SegmentBuilder {

    pub fn new(arena : *mut BuilderArena,
               id : SegmentId,
               ptr : *mut Word,
               size : WordCount) -> SegmentBuilder {
        SegmentBuilder {
            reader : SegmentReader {
                arena : BuilderArenaPtr(arena),
                ptr : unsafe {std::mem::transmute(ptr)},
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
    pub fn get_ptr_unchecked(&self, offset : WordCount) -> *mut Word {
        unsafe {
            std::mem::transmute(self.reader.ptr.offset(offset as int))
        }
    }

    #[inline]
    pub fn get_segment_id(&self) -> SegmentId { self.id }

    #[inline]
    pub fn get_arena(&self) -> *mut BuilderArena {
        match self.reader.arena {
            BuilderArenaPtr(b) => b,
            _ => unreachable!()
        }
    }
}

pub struct ReaderArena {
    //    message : *message::MessageReader<'a>,
    pub segment0 : SegmentReader,

    pub more_segments : Vec<SegmentReader>,
    //XXX should this be a map as in capnproto-c++?

    pub cap_table : Vec<Option<Box<ClientHook:Send>>>,

    pub fail_fast : bool,
}

impl ReaderArena {
    pub fn new(segments : &[&[Word]], options : message::ReaderOptions) -> Box<ReaderArena> {
        assert!(segments.len() > 0);
        let mut arena = box ReaderArena {
            segment0 : SegmentReader {
                arena : Null,
                ptr : unsafe { segments[0].unsafe_ref(0) },
                size : segments[0].len()
            },
            more_segments : Vec::new(),
            cap_table : Vec::new(),
            fail_fast : options.fail_fast,
        };


        let arena_ptr = ReaderArenaPtr (&*arena);

        arena.segment0.arena = arena_ptr;

        if segments.len() > 1 {
            let mut moreSegmentReaders = Vec::new();
            for segment in segments.slice_from(1).iter() {
                let segmentReader = SegmentReader {
                    arena : arena_ptr,
                    ptr : unsafe { segment.unsafe_ref(0) },
                    size : segment.len()
                };
                moreSegmentReaders.push(segmentReader);
            }
            arena.more_segments = moreSegmentReaders;
        }

        arena
    }

    pub fn try_get_segment(&self, id : SegmentId) -> *SegmentReader {
        if id == 0 {
            return &self.segment0 as *SegmentReader;
        } else {
            unsafe { self.more_segments.as_slice().unsafe_ref(id as uint - 1) as *SegmentReader }
        }
    }

    #[inline]
    pub fn init_cap_table(&mut self, cap_table : Vec<Option<Box<ClientHook:Send>>>) {
        self.cap_table = cap_table;
    }

}

pub struct BuilderArena {
    pub segment0 : SegmentBuilder,
    pub more_segments : Vec<Box<SegmentBuilder>>,
    pub allocation_strategy : message::AllocationStrategy,
    pub owned_memory : Vec<*mut Word>,
    pub nextSize : uint,
    pub cap_table : Vec<Option<Box<ClientHook:Send>>>,
    pub fail_fast : bool,
}

impl Drop for BuilderArena {
    fn drop(&mut self) {
        for &segment_ptr in self.owned_memory.iter() {
            unsafe { libc::free(std::mem::transmute(segment_ptr)); }
        }
    }
}

pub enum FirstSegment<'a> {
    NumWords(uint),
    ZeroedWords(&'a mut [Word])
}

impl BuilderArena {

    pub fn new(allocationStrategy : message::AllocationStrategy,
               first_segment : FirstSegment,
               fail_fast : bool) -> Box<BuilderArena> {

        let (first_segment, num_words, owned_memory) : (*mut Word, uint, Vec<*mut Word>) = unsafe {
            match first_segment {
                NumWords(n) => {
                    let ptr = std::mem::transmute(
                        libc::calloc(n as libc::size_t,
                                          BYTES_PER_WORD as libc::size_t));
                    (ptr, n, vec!(ptr))
                }
                ZeroedWords(w) => (w.as_mut_ptr(), w.len(), Vec::new())
            }};

        let mut result = box BuilderArena {
            segment0 : SegmentBuilder {
                reader : SegmentReader {
                    ptr : first_segment as * Word,
                    size : num_words,
                    arena : Null },
                id : 0,
                pos : first_segment,
            },
            more_segments : Vec::new(),
            allocation_strategy : allocationStrategy,
            owned_memory : owned_memory,
            nextSize : num_words,
            cap_table : Vec::new(),
            fail_fast : fail_fast,
        };

        let arena_ptr = { let ref mut ptr = *result; ptr as *mut BuilderArena};
        result.segment0.reader.arena = BuilderArenaPtr(arena_ptr);

        result
    }

    pub fn allocate_owned_memory(&mut self, minimumSize : WordCount) -> (*mut Word, WordCount) {
        let size = std::cmp::max(minimumSize, self.nextSize);
        let new_words : *mut Word = unsafe {
            std::mem::transmute(libc::calloc(size as libc::size_t,
                                                   BYTES_PER_WORD as libc::size_t)) };

        self.owned_memory.push(new_words);

        match self.allocation_strategy {
            message::GrowHeuristically => { self.nextSize += size; }
            _ => { }
        }
        (new_words, size)
    }


    #[inline]
    pub fn allocate(&mut self, amount : WordCount) -> (*mut SegmentBuilder, *mut Word) {
        unsafe {
            match self.segment0.allocate(amount) {
                Some(result) => { return ((&mut self.segment0) as *mut SegmentBuilder, result) }
                None => {}
            }

            //# Need to fall back to additional segments.

            let id = {
                let len = self.more_segments.len();
                if len == 0 { 1 }
                else {
                    let result_ptr = &mut *self.more_segments.as_mut_slice()[len-1] as *mut SegmentBuilder;
                    match self.more_segments.as_mut_slice()[len - 1].allocate(amount) {
                        Some(result) => { return (result_ptr, result) }
                        None => { len + 1 }
                    }
                }};

            let (words, size) = self.allocate_owned_memory(amount);
            let mut new_builder = box SegmentBuilder::new(self, id as u32, words, size);
            let builder_ptr = &mut *new_builder as *mut SegmentBuilder;

            self.more_segments.push(new_builder);

            (builder_ptr, (*builder_ptr).allocate(amount).unwrap() )
        }
    }

    pub fn get_segment(&mut self, id : SegmentId) -> *mut SegmentBuilder {
        if id == 0 {
            &mut self.segment0 as *mut SegmentBuilder
        } else {
            &mut *self.more_segments.as_mut_slice()[(id - 1) as uint] as *mut SegmentBuilder
        }
    }

    pub fn get_segments_for_output<T>(&self, cont : |&[&[Word]]| -> T) -> T {
        unsafe {
            if self.more_segments.len() == 0 {
                std::slice::raw::buf_as_slice::<Word, T>(
                    self.segment0.reader.ptr,
                    self.segment0.current_size(),
                    |v| cont([v]) )
            } else {
                let mut result = Vec::new();
                result.push(std::mem::transmute(
                    std::raw::Slice { data : self.segment0.reader.ptr,
                                      len : self.segment0.current_size()}));

                for seg in self.more_segments.iter() {
                    result.push(std::mem::transmute(
                        std::raw::Slice { data : seg.reader.ptr,
                                          len : seg.current_size()}));
                }
                cont(result.as_slice())
            }
        }
    }

    pub fn get_cap_table<'a>(&'a self) -> &'a [Option<Box<ClientHook:Send>>] {
        self.cap_table.as_slice()
    }

    pub fn inject_cap(&mut self, cap : Box<ClientHook:Send>) -> u32 {
        self.cap_table.push(Some(cap));
        self.cap_table.len() as u32 - 1
    }
}

pub enum ArenaPtr {
    ReaderArenaPtr(*ReaderArena),
    BuilderArenaPtr(*mut BuilderArena),
    Null
}

impl ArenaPtr {
    pub fn try_get_segment(&self, id : SegmentId) -> *SegmentReader {
        unsafe {
            match self {
                &ReaderArenaPtr(reader) => {
                    (&*reader).try_get_segment(id)
                }
                &BuilderArenaPtr(builder) => {
                    if id == 0 {
                        &(*builder).segment0.reader as *SegmentReader
                    } else {
                        &(*builder).more_segments.as_slice()[id as uint - 1].reader as *SegmentReader
                    }
                }
                &Null => {
                    fail!()
                }
            }
        }
    }

    pub fn extract_cap(&self, index : uint) -> Option<Box<ClientHook:Send>> {
        unsafe {
            match self {
                &ReaderArenaPtr(reader) => {
                    if index < (*reader).cap_table.len() {
                        match (*reader).cap_table.as_slice()[index] {
                            Some( ref hook ) => { Some(hook.copy()) }
                            None => {
                                None
                            }
                        }
                    } else {
                        None
                    }
                }
                &BuilderArenaPtr(builder) => {
                    if index < (*builder).cap_table.len() {
                        match (*builder).cap_table.as_slice()[index] {
                            Some( ref hook ) => { Some(hook.copy()) }
                            None => {
                                None
                            }
                        }
                    } else {
                        None
                    }
                }
                &Null => {
                    fail!();
                }
            }
        }
    }

    pub fn fail_fast(&self) -> bool {
        unsafe {
            match self {
                &ReaderArenaPtr(reader) => {
                    (*reader).fail_fast
                }
                &BuilderArenaPtr(builder) => {
                    (*builder).fail_fast
                }
                &Null => {
                    fail!()
                }
            }
        }
    }

}
