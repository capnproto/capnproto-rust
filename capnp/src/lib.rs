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

//! # Cap'n Proto Runtime Library
//!
//! This crate contains basic facilities for reading and writing
//! [Cap'n Proto](https://capnproto.org) messages in Rust. It is intended to
//! be used in conjunction with code generated by the
//! [capnpc-rust](https://crates.io/crates/capnpc) crate.

#![cfg_attr(feature = "rpc_try", feature(try_trait_v2))]
#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate alloc;

pub mod any_pointer;
pub mod any_pointer_list;
pub mod capability;
pub mod capability_list;
pub mod constant;
pub mod data;
pub mod data_list;
pub mod enum_list;
pub mod io;
pub mod list_list;
pub mod message;
pub mod primitive_list;
pub mod private;
pub mod raw;
pub mod serialize;
pub mod serialize_packed;
pub mod struct_list;
pub mod text;
pub mod text_list;
pub mod traits;

use alloc::string::String;
use alloc::vec::Vec;

///
/// 8 bytes, aligned to an 8-byte boundary.
///
/// Internally, capnproto-rust allocates message buffers using this type,
/// to guarantee alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C, align(8))]
pub struct Word {
    raw_content: [u8; 8]
}

///
/// Constructs a word with the given bytes.
///
pub const fn word(b0: u8, b1: u8, b2: u8, b3: u8, b4: u8, b5: u8, b6: u8, b7: u8) -> Word {
    Word { raw_content: [b0,b1,b2,b3,b4,b5,b6,b7] }
}

impl Word {
    /// Does this, but faster: `vec![word(0,0,0,0,0,0,0,0); length]`.
    pub fn allocate_zeroed_vec(length: usize) -> Vec<Word> {
        let mut result: Vec<Word> = Vec::with_capacity(length);
        unsafe {
            let p: *mut u8 = result.as_mut_ptr() as *mut u8;
            core::ptr::write_bytes(p, 0u8, length * core::mem::size_of::<Word>());
            result.set_len(length);
        }
        result
    }

    pub fn words_to_bytes(words: &[Word]) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(words.as_ptr() as *const u8, words.len() * 8)
        }
    }

    pub fn words_to_bytes_mut(words: &mut [Word]) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(words.as_mut_ptr() as *mut u8, words.len() * 8)
        }
    }
}

#[cfg(any(feature="quickcheck", test))]
impl quickcheck::Arbitrary for Word {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Word {
        crate::word(quickcheck::Arbitrary::arbitrary(g),
                    quickcheck::Arbitrary::arbitrary(g),
                    quickcheck::Arbitrary::arbitrary(g),
                    quickcheck::Arbitrary::arbitrary(g),
                    quickcheck::Arbitrary::arbitrary(g),
                    quickcheck::Arbitrary::arbitrary(g),
                    quickcheck::Arbitrary::arbitrary(g),
                    quickcheck::Arbitrary::arbitrary(g))
    }
}

/// Size of a message. Every generated struct has a method `.total_size()` that returns this.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MessageSize {
    pub word_count: u64,

    /// Size of the capability table.
    pub cap_count: u32
}

impl MessageSize {
    pub fn plus_eq(&mut self, other : MessageSize) {
        self.word_count += other.word_count;
        self.cap_count += other.cap_count;
    }
}

/// An enum value or union discriminant that was not found among those defined in a schema.
#[derive(PartialEq, Clone, Copy, Debug)]
pub struct NotInSchema(pub u16);

impl ::core::fmt::Display for NotInSchema {
    fn fmt(&self, fmt: &mut ::core::fmt::Formatter) -> ::core::result::Result<(), ::core::fmt::Error> {
        write!(fmt, "Enum value or union discriminant {} was not present in the schema.", self.0)
    }
}

#[cfg(feature="std")]
impl ::std::error::Error for NotInSchema {
    fn description(&self) -> &str {
        "Enum value or union discriminant was not present in schema."
    }
}

/// Because messages are lazily validated, the return type of any method that reads a pointer field
/// must be wrapped in a Result.
pub type Result<T> = ::core::result::Result<T, Error>;

/// Describes an arbitrary error that prevented an operation from completing.
#[derive(Debug, Clone)]
pub struct Error {
    /// The general kind of the error. Code that decides how to respond to an error
    /// should read only this field in making its decision.
    pub kind: ErrorKind,

    /// Human-readable failure description.
    pub description: String,
}

/// The general nature of an error. The purpose of this enum is not to describe the error itself,
/// but rather to describe how the client might want to respond to the error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Something went wrong. This is the usual error kind. It includes decoding errors.
    Failed,

    /// The call failed because of a temporary lack of resources. This could be space resources
    /// (out of memory, out of disk space) or time resources (request queue overflow, operation
    /// timed out).
    ///
    /// The operation might work if tried again, but it should NOT be repeated immediately as this
    /// may simply exacerbate the problem.
    Overloaded,

    /// The call required communication over a connection that has been lost. The callee will need
    /// to re-establish connections and try again.
    Disconnected,

    /// The requested method is not implemented. The caller may wish to revert to a fallback
    /// approach based on other methods.
    Unimplemented,
}

impl Error {
    pub fn failed(description: String) -> Error {
        Error { description, kind: ErrorKind::Failed }
    }
    pub fn overloaded(description: String) -> Error {
        Error { description, kind: ErrorKind::Overloaded }
    }
    pub fn disconnected(description: String) -> Error {
        Error { description, kind: ErrorKind::Disconnected }
    }
    pub fn unimplemented(description: String) -> Error {
        Error { description, kind: ErrorKind::Unimplemented }
    }
}

#[cfg(feature="std")]
impl core::convert::From<::std::io::Error> for Error {
    fn from(err: ::std::io::Error) -> Error {
        use std::io;
        let kind = match err.kind() {
            io::ErrorKind::TimedOut => ErrorKind::Overloaded,
            io::ErrorKind::BrokenPipe |
            io::ErrorKind::ConnectionRefused |
            io::ErrorKind::ConnectionReset |
            io::ErrorKind::ConnectionAborted |
            io::ErrorKind::NotConnected  => ErrorKind::Disconnected,
            _ => ErrorKind::Failed,
        };
        Error { description: format!("{}", err), kind }
    }
}

impl core::convert::From<alloc::string::FromUtf8Error> for Error {
    fn from(err: alloc::string::FromUtf8Error) -> Error {
        Error::failed(format!("{}", err))
    }
}

impl core::convert::From<alloc::str::Utf8Error> for Error {
    fn from(err: alloc::str::Utf8Error) -> Error {
        Error::failed(format!("{}", err))
    }
}

impl core::convert::From<NotInSchema> for Error {
    fn from(e: NotInSchema) -> Error {
        Error::failed(format!("Enum value or union discriminant {} was not present in schema.", e.0))
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{:?}: {}", self.kind, self.description)
    }
}

#[cfg(feature="std")]
impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        &self.description
    }
    fn cause(&self) -> Option<&dyn (::std::error::Error)> {
        None
    }
}

/// Helper struct that allows `MessageBuilder::get_segments_for_output()` to avoid heap allocations
/// in the single-segment case.
pub enum OutputSegments<'a> {
    SingleSegment([&'a [u8]; 1]),
    MultiSegment(Vec<&'a [u8]>),
}

impl <'a> core::ops::Deref for OutputSegments<'a> {
    type Target = [&'a [u8]];
    fn deref(&self) -> &[&'a [u8]] {
        match *self {
            OutputSegments::SingleSegment(ref s) => {
                s
            }
            OutputSegments::MultiSegment(ref v) => {
                v
            }
        }
    }
}

impl<'s> message::ReaderSegments for OutputSegments<'s> {
    fn get_segment(&self, id: u32) -> Option<&[u8]> {
        match *self {
            OutputSegments::SingleSegment(ref s) => {
                s.get(id as usize).copied()
            }
            OutputSegments::MultiSegment(ref v) => {
                v.get(id as usize).copied()
            }
        }
    }
}
