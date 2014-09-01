/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */


use std;
use std::vec::Vec;

pub type BitCount0 = uint; // `BitCount` clashes with a standard trait
pub type BitCount8 = u8;
pub type BitCount16 = u16;
pub type BitCount32 = u32;
pub type BitCount64 = u64;

pub type ByteCount = uint;
pub type ByteCount8 = u8;
pub type ByteCount16 = u16;
pub type ByteCount32 = u32;
pub type ByteCount64 = u64;

pub type WordCount = uint;
pub type WordCount8 = u8;
pub type WordCount16 = u16;
pub type WordCount32 = u32;
pub type WordCount64 = u64;

pub type ElementCount = uint;
pub type ElementCount8 = u8;
pub type ElementCount16 = u16;
pub type ElementCount32 = u32;
pub type ElementCount64 = u64;

pub type WirePointerCount = uint;
pub type WirePointerCount8 = u8;
pub type WirePointerCount16 = u16;
pub type WirePointerCount32 = u32;
pub type WirePointerCount64 = u64;

pub struct Word {_x : u64}

pub static BITS_PER_BYTE : BitCount0 = 8;
pub static BITS_PER_WORD : BitCount0 = 64;
pub static BYTES_PER_WORD : ByteCount = 8;

pub static BITS_PER_POINTER : BitCount0 = 64;
pub static BYTES_PER_POINTER : ByteCount = 8;
pub static WORDS_PER_POINTER : WordCount = 1;

pub static POINTER_SIZE_IN_WORDS : WordCount = 1;

pub fn bytes_per_element<T>() -> ByteCount {
    std::mem::size_of::<T>()
}

pub fn bits_per_element<T>() -> BitCount0 {
    8 * std::mem::size_of::<T>()
}

#[inline]
pub fn allocate_zeroed_words(size : WordCount) -> Vec<Word> {

//    Do this, but faster:
//    return std::vec::Vec::from_elem(size, 0);

    let mut result : Vec<Word> = Vec::with_capacity(size);
    unsafe {
        result.set_len(size);
        let p : *mut u8 = std::mem::transmute(result.as_mut_slice().as_mut_ptr());
        std::ptr::set_memory(p, 0, size * BYTES_PER_WORD);
    }
    return result;
}

pub struct MessageSize {
    //# Size of a message. Every struct type has a method `.total_size()` that returns this.
    pub word_count : u64,
    pub cap_count : uint
}

impl MessageSize {
    pub fn plus_eq(&mut self, other : MessageSize) {
        self.word_count += other.word_count;
        self.cap_count += other.cap_count;
    }
}

#[inline]
pub fn ptr_sub<T, U:std::ptr::RawPtr<T>, V: std::ptr::RawPtr<T>>(p1 : U, p2 : V) -> uint {
    return (p1.to_uint() - p2.to_uint()) / std::mem::size_of::<T>();
}
