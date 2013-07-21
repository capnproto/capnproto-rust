/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */


use std;

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

pub static BITS_PER_BYTE : BitCount0 = 8;
pub static BITS_PER_WORD : BitCount0 = 64;
pub static BYTES_PER_WORD : ByteCount = 8;

pub static BITS_PER_POINTER : BitCount0 = 64;
pub static BYTES_PER_POINTER : ByteCount = 8;
pub static WORDS_PER_POINTER : WordCount = 1;

pub static POINTER_SIZE_IN_WORDS : WordCount = 1;

pub fn bytesPerElement<T>() -> ByteCount {
    std::sys::size_of::<T>()
}

pub fn bitsPerElement<T>() -> BitCount0 {
    8 * std::sys::size_of::<T>()
}


