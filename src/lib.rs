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
//! [Cap'n Proto](https://capnproto.org) is an extremely efficient protocol for
//! sharing data and capabilities.
//!
//! The Rust implementation is split into three separate crates.
//!
//! Code generation is handled by [capnpc-rust](https://github.com/dwrensha/capnpc-rust).
//!
//! The present crate is the runtime library required by that generated code. It is hosted on Github
//! [here](https://github.com/dwrensha/capnproto-rust).
//!
//! [capnp-rpc-rust](https://github.com/dwrensha/capnp-rpc-rust) is an implementation of a
//! distributed object-capability layer.

extern crate byteorder;

#[cfg(any(feature="quickcheck", test))]
extern crate quickcheck;

#[cfg(feature = "rpc")]
extern crate gj;

/// Constructs a [`Word`](struct.Word.html) from its constituent bytes.
/// This macro can be used to construct constants. In the future, when
/// Rust supports [constant functions](https://github.com/rust-lang/rust/issues/24111),
///  this macro will be replaced by such a function.
#[macro_export]
#[cfg(target_endian = "little")]
macro_rules! capnp_word {
  ($b0:expr, $b1:expr, $b2:expr, $b3:expr,
   $b4:expr, $b5:expr, $b6:expr, $b7:expr) => (
    $crate::Word {
        raw_content: (($b0 as u64) << 0) + (($b1 as u64) << 8) +
                     (($b2 as u64) << 16) + (($b3 as u64) << 24) +
                     (($b4 as u64) << 32) + (($b5 as u64) << 40) +
                     (($b6 as u64) << 48) + (($b7 as u64) << 56)
    }
  )
}
#[cfg(target_endian = "big")]
macro_rules! capnp_word {
  ($b0:expr, $b1:expr, $b2:expr, $b3:expr,
   $b4:expr, $b5:expr, $b6:expr, $b7:expr) => (
     $crate::Word {
         raw_content: (($b7 as u64) << 0) + (($b6 as u64) << 8) +
                      (($b5 as u64) << 16) + (($b4 as u64) << 24) +
                      (($b3 as u64) << 32) + (($b2 as u64) << 40) +
                      (($b1 as u64) << 48) + (($b0 as u64) << 56)
     }
  )
}

pub mod any_pointer;
pub mod capability;
pub mod data;
pub mod data_list;
pub mod enum_list;
pub mod list_list;
pub mod message;
pub mod primitive_list;
pub mod private;
pub mod serialize;
pub mod serialize_packed;
pub mod struct_list;
pub mod text;
pub mod text_list;
pub mod traits;

mod util;

/// Eight bytes of memory with opaque interior. Use [`capnp_word!()`](macro.capnp_word!.html)
/// to construct one of these.
///
/// This type is used to ensure that the data of a message is properly aligned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Word {
    #[doc(hidden)]
    pub raw_content: u64,
}

impl Word {
    /// Does this, but faster:
    /// `::std::iter::repeat(Word(0)).take(length).collect()`
    pub fn allocate_zeroed_vec(length: usize) -> Vec<Word> {
        let mut result : Vec<Word> = Vec::with_capacity(length);
        unsafe {
            result.set_len(length);
            let p : *mut u8 = result.as_mut_ptr() as *mut u8;
            ::std::ptr::write_bytes(p, 0u8, length * ::std::mem::size_of::<Word>());
        }
        result
    }

    pub fn bytes_to_words<'a>(bytes: &'a [u8]) -> &'a [Word] {
        unsafe {
            ::std::slice::from_raw_parts(bytes.as_ptr() as *const Word, bytes.len() / 8)
        }
    }

    pub fn bytes_to_words_mut<'a>(bytes: &'a mut [u8]) -> &'a mut [Word] {
        unsafe {
            ::std::slice::from_raw_parts_mut(bytes.as_ptr() as *mut Word, bytes.len() / 8)
        }
    }

    pub fn words_to_bytes<'a>(words: &'a [Word]) -> &'a [u8] {
        unsafe {
            ::std::slice::from_raw_parts(words.as_ptr() as *const u8, words.len() * 8)
        }
    }

    pub fn words_to_bytes_mut<'a>(words: &'a mut [Word]) -> &'a mut [u8] {
        unsafe {
            ::std::slice::from_raw_parts_mut(words.as_mut_ptr() as *mut u8, words.len() * 8)
        }
    }

    #[cfg(test)]
    pub fn from(n: u64) -> Word {
        Word { raw_content: n }
    }
}

#[cfg(any(feature="quickcheck", test))]
impl quickcheck::Arbitrary for Word {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Word {
        Word { raw_content: quickcheck::Arbitrary::arbitrary(g) }
    }
}

/// Size of a message. Every generated struct has a method `.total_size()` that returns this.
#[derive(Clone, Copy, PartialEq)]
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

impl ::std::fmt::Display for NotInSchema {
    fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(fmt, "Enum value or union discriminant {} was not present in the schema.", self.0)
    }
}

impl ::std::error::Error for NotInSchema {
    fn description<'a>(&'a self) -> &'a str {
        "Enum value or union disriminant was not present in schema."
    }
}

/// Because messages are lazily validated, the return type of any method that reads a pointer field
/// must be wrapped in a Result.
pub type Result<T> = ::std::result::Result<T, Error>;

/// Describes an arbitrary error that prevented an operation from completing.
#[derive(Debug, Clone)]
pub struct Error {
    /// The type of the error. The purpose of this enum is not to describe the error itself, but
    /// rather to describe how the client might want to respond to the error.
    pub kind: ErrorKind,

    /// Human-readable failure description.
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Something went wrong. This is the usual error kind. It includes decoding errors.
    Failed,

    /// The call failed because of a temporary lack of resources. This could be space resources
    /// (out of memory, out of disk space) or time resources (request queue overflow, operation
    /// timed out).
    //
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
        Error { description: description, kind: ErrorKind::Failed }
    }
    pub fn overloaded(description: String) -> Error {
        Error { description: description, kind: ErrorKind::Overloaded }
    }
    pub fn disconnected(description: String) -> Error {
        Error { description: description, kind: ErrorKind::Disconnected }
    }
    pub fn unimplemented(description: String) -> Error {
        Error { description: description, kind: ErrorKind::Unimplemented }
    }
}

impl ::std::convert::From<::std::io::Error> for Error {
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
        Error { description: format!("{}", err), kind: kind }
    }
}

impl ::std::convert::From<NotInSchema> for Error {
    fn from(e: NotInSchema) -> Error {
        Error::failed(format!("Enum value or union discriminant {} was not present in schema.", e.0))
    }
}

impl ::std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(fmt, "{:?}: {}", self.kind, self.description)
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        &self.description
    }
    fn cause(&self) -> Option<&::std::error::Error> {
        None
    }
}

#[cfg(feature = "rpc")]
impl ::gj::FulfillerDropped for Error {
    fn fulfiller_dropped() -> Error {
        Error::failed("Promise fulfiller was dropped.".to_string())
    }
}

/// Helper struct that allows `MessageBuilder::get_segments_for_output()` to avoid heap allocations
/// in the single-segment case.
pub enum OutputSegments<'a> {
    #[doc(hidden)]
    SingleSegment([&'a [Word]; 1]),

    #[doc(hidden)]
    MultiSegment(Vec<&'a [Word]>),
}

impl <'a> ::std::ops::Deref for OutputSegments<'a> {
    type Target = [&'a [Word]];
    fn deref<'b>(&'b self) -> &'b [&'a [Word]] {
        match self {
            &OutputSegments::SingleSegment(ref s) => {
                s
            }
            &OutputSegments::MultiSegment(ref v) => {
                &*v
            }
        }
    }
}
