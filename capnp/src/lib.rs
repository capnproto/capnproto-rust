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

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

/// Code generated from
/// [schema.capnp](https://github.com/capnproto/capnproto/blob/master/c%2B%2B/src/capnp/schema.capnp).
pub mod schema_capnp;

pub mod any_pointer;
pub mod any_pointer_list;
pub mod capability;
pub mod capability_list;
pub mod constant;
pub mod data;
pub mod data_list;
pub mod dynamic_list;
pub mod dynamic_struct;
pub mod dynamic_value;
pub mod enum_list;
pub mod introspect;
pub mod io;
pub mod list_list;
pub mod message;
pub mod primitive_list;
pub mod private;
pub mod raw;
pub mod schema;
pub mod serialize;
pub mod serialize_packed;
pub(crate) mod stringify;
pub mod struct_list;
pub mod text;
pub mod text_list;
pub mod traits;

#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

///
/// 8 bytes, aligned to an 8-byte boundary.
///
/// Internally, capnproto-rust allocates message buffers using this type,
/// to guarantee alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C, align(8))]
pub struct Word {
    raw_content: [u8; 8],
}

///
/// Constructs a word with the given bytes.
///
#[allow(clippy::too_many_arguments)]
pub const fn word(b0: u8, b1: u8, b2: u8, b3: u8, b4: u8, b5: u8, b6: u8, b7: u8) -> Word {
    Word {
        raw_content: [b0, b1, b2, b3, b4, b5, b6, b7],
    }
}

impl Word {
    /// Allocates a vec of `length` words, all set to zero.
    #[cfg(feature = "alloc")]
    pub fn allocate_zeroed_vec(length: usize) -> Vec<Self> {
        vec![word(0, 0, 0, 0, 0, 0, 0, 0); length]
    }

    pub fn words_to_bytes(words: &[Self]) -> &[u8] {
        unsafe { core::slice::from_raw_parts(words.as_ptr() as *const u8, words.len() * 8) }
    }

    pub fn words_to_bytes_mut(words: &mut [Self]) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(words.as_mut_ptr() as *mut u8, words.len() * 8) }
    }
}

#[cfg(any(feature = "quickcheck", test))]
impl quickcheck::Arbitrary for Word {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        crate::word(
            quickcheck::Arbitrary::arbitrary(g),
            quickcheck::Arbitrary::arbitrary(g),
            quickcheck::Arbitrary::arbitrary(g),
            quickcheck::Arbitrary::arbitrary(g),
            quickcheck::Arbitrary::arbitrary(g),
            quickcheck::Arbitrary::arbitrary(g),
            quickcheck::Arbitrary::arbitrary(g),
            quickcheck::Arbitrary::arbitrary(g),
        )
    }
}

/// Size of a message. Every generated struct has a method `.total_size()` that returns this.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MessageSize {
    pub word_count: u64,

    /// Size of the capability table.
    pub cap_count: u32,
}

impl core::ops::AddAssign for MessageSize {
    fn add_assign(&mut self, rhs: Self) {
        self.word_count += rhs.word_count;
        self.cap_count += rhs.cap_count;
    }
}

/// An enum value or union discriminant that was not found among those defined in a schema.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct NotInSchema(pub u16);

impl ::core::fmt::Display for NotInSchema {
    fn fmt(
        &self,
        fmt: &mut ::core::fmt::Formatter,
    ) -> ::core::result::Result<(), ::core::fmt::Error> {
        write!(
            fmt,
            "Enum value or union discriminant {} was not present in the schema.",
            self.0
        )
    }
}

#[cfg(feature = "std")]
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

    /// Extra context about error
    #[cfg(feature = "alloc")]
    pub extra: String,
}

/// The general nature of an error. The purpose of this enum is not to describe the error itself,
/// but rather to describe how the client might want to respond to the error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Something went wrong
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

    /// Buffer is not large enough
    BufferNotLargeEnough,

    /// Cannot create a canonical message with a capability
    CannotCreateACanonicalMessageWithACapability,

    /// Cannot set AnyPointer field to a primitive value
    CannotSetAnyPointerFieldToAPrimitiveValue,

    /// Don't know how to handle non-STRUCT inline composite.
    CantHandleNonStructInlineComposite,

    /// Empty buffer
    EmptyBuffer,

    /// Empty slice
    EmptySlice,

    /// Enum value or union discriminant {} was not present in schema
    EnumValueOrUnionDiscriminantNotPresent(NotInSchema),

    /// Called get_writable_{data|text}_pointer() but existing list pointer is not byte-sized.
    ExistingListPointerIsNotByteSized,

    /// Existing list value is incompatible with expected type.
    ExistingListValueIsIncompatibleWithExpectedType,

    /// Called get_writable_{data|text|list|struct_list}_pointer() but existing pointer is not a list.
    ExistingPointerIsNotAList,

    /// Expected a list or blob.
    ExpectedAListOrBlob,

    /// Expected a pointer list, but got a list of data-only structs
    ExpectedAPointerListButGotAListOfDataOnlyStructs,

    /// Expected a primitive list, but got a list of pointer-only structs
    ExpectedAPrimitiveListButGotAListOfPointerOnlyStructs,

    /// failed to fill the whole buffer
    FailedToFillTheWholeBuffer,

    /// field and default mismatch
    FieldAndDefaultMismatch,

    /// field not found
    FieldNotFound,

    /// Found bit list where struct list was expected; upgrading boolean lists to struct lists is no longer supported
    FoundBitListWhereStructListWasExpected,

    /// Found struct list where bit list was expected.
    FoundStructListWhereBitListWasExpected,

    /// Cannot represent 4 byte length as `usize`. This may indicate that you are running on 8 or 16 bit platform or message is too large.
    FourByteLengthTooBigForUSize,

    /// Cannot represent 4 byte segment length as usize. This may indicate that you are running on 8 or 16 bit platform or segment is too large
    FourByteSegmentLengthTooBigForUSize,

    /// group field but type is not Struct
    GroupFieldButTypeIsNotStruct,

    /// init() is only valid for struct and AnyPointer fields
    InitIsOnlyValidForStructAndAnyPointerFields,

    /// initn() is only valid for list, text, or data fields
    InitnIsOnlyValidForListTextOrDataFields,

    /// InlineComposite list with non-STRUCT elements not supported.
    InlineCompositeListWithNonStructElementsNotSupported,

    /// InlineComposite list's elements overrun its word count.
    InlineCompositeListsElementsOverrunItsWordCount,

    /// InlineComposite lists of non-STRUCT type are not supported.
    InlineCompositeListsOfNonStructTypeAreNotSupported,

    /// Too many or too few segments {segment_count}
    InvalidNumberOfSegments(usize),

    /// Invalid segment id {id}
    InvalidSegmentId(u32),

    /// List(AnyPointer) not supported.
    ListAnyPointerNotSupported,

    /// List(Capability) not supported
    ListCapabilityNotSupported,

    /// Malformed double-far pointer.
    MalformedDoubleFarPointer,

    /// Message contains invalid capability pointer.
    MessageContainsInvalidCapabilityPointer,

    /// Message contains list pointer of non-bytes where data was expected.
    MessageContainsListPointerOfNonBytesWhereDataWasExpected,

    /// Message contains list pointer of non-bytes where text was expected.
    MessageContainsListPointerOfNonBytesWhereTextWasExpected,

    /// Message contains list with incompatible element type.
    MessageContainsListWithIncompatibleElementType,

    /// Message contains non-capability pointer where capability pointer was expected.
    MessageContainsNonCapabilityPointerWhereCapabilityPointerWasExpected,

    /// Message contains non-struct pointer where struct pointer was expected.
    MessageContainsNonStructPointerWhereStructPointerWasExpected,

    /// Message contains non-list pointer where data was expected.
    MessageContainsNonListPointerWhereDataWasExpected,

    /// Message contains non-list pointer where list pointer was expected
    MessageContainsNonListPointerWhereListPointerWasExpected,

    /// Message contains non-list pointer where text was expected.
    MessageContainsNonListPointerWhereTextWasExpected,

    /// Message contains null capability pointer.
    MessageContainsNullCapabilityPointer,

    /// Message contains out-of-bounds pointer,
    MessageContainsOutOfBoundsPointer,

    /// Message contains text that is not NUL-terminated
    MessageContainsTextThatIsNotNULTerminated,

    /// Message ends prematurely. Header claimed {header} words, but message only has {body} words,
    MessageEndsPrematurely(usize, usize),

    /// Message is too deeply nested.
    MessageIsTooDeeplyNested,

    /// Message is too deeply-nested or contains cycles.
    MessageIsTooDeeplyNestedOrContainsCycles,

    /// Message was not aligned by 8 bytes boundary. Either ensure that message is properly aligned or compile `capnp` crate with \"unaligned\" feature enabled.
    MessageNotAlignedBy8BytesBoundary,

    /// Message's size cannot be represented in usize
    MessageSizeOverflow,

    /// Message is too large
    MessageTooLarge(usize),

    /// Nesting limit exceeded
    NestingLimitExceeded,

    /// Not a struct
    NotAStruct,

    /// Only one of the section pointers is pointing to ourself
    OnlyOneOfTheSectionPointersIsPointingToOurself,

    /// Packed input did not end cleanly on a segment boundary.
    PackedInputDidNotEndCleanlyOnASegmentBoundary,

    /// Premature end of file
    PrematureEndOfFile,

    /// Premature end of packed input.
    PrematureEndOfPackedInput,

    /// Read limit exceeded
    ReadLimitExceeded,

    /// setting dynamic capabilities is unsupported
    SettingDynamicCapabilitiesIsUnsupported,

    /// Struct reader had bitwidth other than 1
    StructReaderHadBitwidthOtherThan1,

    /// Text blob missing NUL terminator.
    TextBlobMissingNULTerminator,

    /// Text contains non-utf8 data
    TextContainsNonUtf8Data(core::str::Utf8Error),

    /// Tried to read from null arena
    TriedToReadFromNullArena,

    /// type mismatch
    TypeMismatch,

    /// Detected unaligned segment. You must either ensure all of your segments are 8-byte aligned,
    /// or you must enable the "unaligned" feature in the capnp crate
    UnalignedSegment,

    /// Unexpected far pointer
    UnexepectedFarPointer,

    /// Unknown pointer type.
    UnknownPointerType,
}

impl Error {
    /// Writes to the `extra` field. Does nothing if the "alloc" feature is not enabled.
    /// This is intended to be used with the `write!()` macro from core.
    pub fn write_fmt(&mut self, fmt: core::fmt::Arguments<'_>) {
        #[cfg(feature = "alloc")]
        {
            use core::fmt::Write;
            let _ = self.extra.write_fmt(fmt);
        }
    }

    #[cfg(feature = "alloc")]
    pub fn failed(description: String) -> Self {
        Self {
            extra: description,
            kind: ErrorKind::Failed,
        }
    }

    pub fn from_kind(kind: ErrorKind) -> Self {
        #[cfg(not(feature = "alloc"))]
        return Self { kind };
        #[cfg(feature = "alloc")]
        return Self {
            kind,
            extra: String::new(),
        };
    }

    #[cfg(feature = "alloc")]
    pub fn overloaded(description: String) -> Self {
        Self {
            extra: description,
            kind: ErrorKind::Overloaded,
        }
    }
    #[cfg(feature = "alloc")]
    pub fn disconnected(description: String) -> Self {
        Self {
            extra: description,
            kind: ErrorKind::Disconnected,
        }
    }

    #[cfg(feature = "alloc")]
    pub fn unimplemented(description: String) -> Self {
        Self {
            extra: description,
            kind: ErrorKind::Unimplemented,
        }
    }
}

#[cfg(feature = "std")]
impl core::convert::From<::std::io::Error> for Error {
    fn from(err: ::std::io::Error) -> Self {
        use std::io;
        let kind = match err.kind() {
            io::ErrorKind::TimedOut => ErrorKind::Overloaded,
            io::ErrorKind::BrokenPipe
            | io::ErrorKind::ConnectionRefused
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::NotConnected => ErrorKind::Disconnected,
            io::ErrorKind::UnexpectedEof => ErrorKind::PrematureEndOfFile,
            _ => ErrorKind::Failed,
        };
        #[cfg(feature = "alloc")]
        return Self {
            kind,
            extra: format!("{err}"),
        };
        #[cfg(not(feature = "alloc"))]
        return Self { kind };
    }
}

#[cfg(feature = "embedded-io")]
impl From<embedded_io::ErrorKind> for ErrorKind {
    fn from(value: embedded_io::ErrorKind) -> Self {
        match value {
            embedded_io::ErrorKind::Other => Self::Failed,
            embedded_io::ErrorKind::NotFound => Self::Failed,
            embedded_io::ErrorKind::PermissionDenied => Self::Failed,
            embedded_io::ErrorKind::ConnectionRefused => Self::Failed,
            embedded_io::ErrorKind::ConnectionReset => Self::Failed,
            embedded_io::ErrorKind::ConnectionAborted => Self::Failed,
            embedded_io::ErrorKind::NotConnected => Self::Failed,
            embedded_io::ErrorKind::AddrInUse => Self::Failed,
            embedded_io::ErrorKind::AddrNotAvailable => Self::Failed,
            embedded_io::ErrorKind::BrokenPipe => Self::Failed,
            embedded_io::ErrorKind::AlreadyExists => Self::Failed,
            embedded_io::ErrorKind::InvalidInput => Self::Failed,
            embedded_io::ErrorKind::InvalidData => Self::Failed,
            embedded_io::ErrorKind::TimedOut => Self::Failed,
            embedded_io::ErrorKind::Interrupted => Self::Failed,
            embedded_io::ErrorKind::Unsupported => Self::Failed,
            embedded_io::ErrorKind::OutOfMemory => Self::Failed,
            _ => Self::Failed,
        }
    }
}

#[cfg(feature = "alloc")]
impl core::convert::From<alloc::string::FromUtf8Error> for Error {
    fn from(err: alloc::string::FromUtf8Error) -> Self {
        Self::failed(format!("{err}"))
    }
}

impl core::convert::From<core::str::Utf8Error> for Error {
    fn from(err: core::str::Utf8Error) -> Self {
        Self::from_kind(ErrorKind::TextContainsNonUtf8Data(err))
    }
}

impl core::convert::From<NotInSchema> for Error {
    fn from(e: NotInSchema) -> Self {
        Self::from_kind(ErrorKind::EnumValueOrUnionDiscriminantNotPresent(e))
    }
}

impl core::fmt::Display for ErrorKind {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        match self {
            Self::Failed => write!(fmt, "Failed"),
            Self::Overloaded => write!(fmt, "Overloaded"),
            Self::Disconnected => write!(fmt, "Disconnected"),
            Self::Unimplemented => write!(fmt, "Unimplemented"),
            Self::BufferNotLargeEnough => write!(fmt, "buffer is not large enough"),
            Self::ExistingListPointerIsNotByteSized => write!(fmt, "Called get_writable_{{data|text}}_pointer() but existing list pointer is not byte-sized."),
            Self::ExistingPointerIsNotAList => write!(fmt, "Called get_writable_{{data|text|list|struct_list}}_pointer() but existing pointer is not a list."),
            Self::CannotCreateACanonicalMessageWithACapability => write!(fmt, "Cannot create a canonical message with a capability"),
            Self::FourByteLengthTooBigForUSize => write!(fmt, "Cannot represent 4 byte length as `usize`. This may indicate that you are running on 8 or 16 bit platform or message is too large."),
            Self::FourByteSegmentLengthTooBigForUSize => write!(fmt, "Cannot represent 4 byte segment length as usize. This may indicate that you are running on 8 or 16 bit platform or segment is too large"),
            Self::CannotSetAnyPointerFieldToAPrimitiveValue => write!(fmt, "cannot set AnyPointer field to a primitive value"),
            Self::CantHandleNonStructInlineComposite => write!(fmt, "Don't know how to handle non-STRUCT inline composite."),
            Self::EmptyBuffer => write!(fmt, "empty buffer"),
            Self::EmptySlice => write!(fmt, "empty slice"),
            Self::EnumValueOrUnionDiscriminantNotPresent(val) => write!(fmt, "Enum value or union discriminant {val} was not present in schema"),
            Self::ExistingListValueIsIncompatibleWithExpectedType => write!(fmt, "Existing list value is incompatible with expected type."),
            Self::ExpectedAListOrBlob => write!(fmt, "Expected a list or blob."),
            Self::ExpectedAPointerListButGotAListOfDataOnlyStructs => write!(fmt, "Expected a pointer list, but got a list of data-only structs"),
            Self::ExpectedAPrimitiveListButGotAListOfPointerOnlyStructs => write!(fmt, "Expected a primitive list, but got a list of pointer-only structs"),
            Self::FailedToFillTheWholeBuffer => write!(fmt, "failed to fill the whole buffer"),
            Self::FieldAndDefaultMismatch => write!(fmt, "field and default mismatch"),
            Self::FieldNotFound => write!(fmt, "field not found"),
            Self::FoundBitListWhereStructListWasExpected => write!(fmt, "Found bit list where struct list was expected; upgrading boolean lists to struct lists is no longer supported."),
            Self::FoundStructListWhereBitListWasExpected => write!(fmt, "Found struct list where bit list was expected."),
            Self::GroupFieldButTypeIsNotStruct => write!(fmt, "group field but type is not Struct"),
            Self::InitIsOnlyValidForStructAndAnyPointerFields => write!(fmt, "init() is only valid for struct and AnyPointer fields"),
            Self::InitnIsOnlyValidForListTextOrDataFields => write!(fmt, "initn() is only valid for list, text, or data fields"),
            Self::InlineCompositeListWithNonStructElementsNotSupported => write!(fmt, "InlineComposite list with non-STRUCT elements not supported."),
            Self::InlineCompositeListsElementsOverrunItsWordCount => write!(fmt, "InlineComposite list's elements overrun its word count."),
            Self::InlineCompositeListsOfNonStructTypeAreNotSupported => write!(fmt, "InlineComposite lists of non-STRUCT type are not supported."),
            Self::InvalidNumberOfSegments(segment_count) => write!(fmt, "Too many or too few segments {segment_count}"),
            Self::InvalidSegmentId(id) => write!(fmt, "Invalid segment id {id}"),
            Self::ListAnyPointerNotSupported => write!(fmt, "List(AnyPointer) not supported."),
            Self::ListCapabilityNotSupported => write!(fmt, "List(Capability) not supported"),
            Self::MalformedDoubleFarPointer => write!(fmt, "Malformed double-far pointer."),
            Self::MessageContainsInvalidCapabilityPointer => write!(fmt, "Message contained invalid capability pointer."),
            Self::MessageContainsListPointerOfNonBytesWhereDataWasExpected => write!(fmt, "Message contains list pointer of non-bytes where data was expected."),
            Self::MessageContainsListPointerOfNonBytesWhereTextWasExpected => write!(fmt, "Message contains list pointer of non-bytes where text was expected."),
            Self::MessageContainsListWithIncompatibleElementType => write!(fmt, "Message contains list with incompatible element type."),
            Self::MessageContainsNonCapabilityPointerWhereCapabilityPointerWasExpected => write!(fmt, "Message contains non-capability pointer where capability pointer was expected."),
            Self::MessageContainsNonListPointerWhereDataWasExpected => write!(fmt, "Message contains non-list pointer where data was expected."),
            Self::MessageContainsNonListPointerWhereListPointerWasExpected => write!(fmt, "Message contains non-list pointer where list pointer was expected"),
            Self::MessageContainsNonListPointerWhereTextWasExpected => write!(fmt, "Message contains non-list pointer where text was expected."),
            Self::MessageContainsNonStructPointerWhereStructPointerWasExpected => write!(fmt, "Message contains non-struct pointer where struct pointer was expected."),
            Self::MessageContainsNullCapabilityPointer => write!(fmt, "Message contains null capability pointer."),
            Self::MessageContainsOutOfBoundsPointer => write!(fmt, "Message contains out-of-bounds pointer"),
            Self::MessageContainsTextThatIsNotNULTerminated => write!(fmt, "Message contains text that is not NUL-terminated"),
            Self::MessageEndsPrematurely(header, body) => write!(fmt, "Message ends prematurely. Header claimed {header} words, but message only has {body} words"),
            Self::MessageIsTooDeeplyNested => write!(fmt, "Message is too deeply nested."),
            Self::MessageIsTooDeeplyNestedOrContainsCycles => write!(fmt, "Message is too deeply-nested or contains cycles."),
            Self::MessageSizeOverflow => write!(fmt, "Message's size cannot be represented in usize"),
            Self::MessageTooLarge(val) => write!(fmt, "Message is too large: {val}"),
            Self::MessageNotAlignedBy8BytesBoundary => write!(fmt, "Message was not aligned by 8 bytes boundary. Either ensure that message is properly aligned or compile `capnp` crate with \"unaligned\" feature enabled."),
            Self::NestingLimitExceeded => write!(fmt, "nesting limit exceeded"),
            Self::NotAStruct => write!(fmt, "not a struct"),
            Self::OnlyOneOfTheSectionPointersIsPointingToOurself => write!(fmt, "Only one of the section pointers is pointing to ourself"),
            Self::PackedInputDidNotEndCleanlyOnASegmentBoundary => write!(fmt, "Packed input did not end cleanly on a segment boundary."),
            Self::PrematureEndOfFile => write!(fmt, "Premature end of file"),
            Self::PrematureEndOfPackedInput => write!(fmt, "Premature end of packed input."),
            Self::ReadLimitExceeded => write!(fmt, "Read limit exceeded"),
            Self::SettingDynamicCapabilitiesIsUnsupported => write!(fmt, "setting dynamic capabilities is unsupported"),
            Self::StructReaderHadBitwidthOtherThan1 => write!(fmt, "struct reader had bitwidth other than 1"),
            Self::TextBlobMissingNULTerminator => write!(fmt, "Text blob missing NUL terminator."),
            Self::TextContainsNonUtf8Data(e) => write!(fmt, "Text contains non-utf8 data: {e}"),
            Self::TriedToReadFromNullArena => write!(fmt, "Tried to read from null arena"),
            Self::TypeMismatch => write!(fmt, "type mismatch"),
            Self::UnalignedSegment => write!(fmt, "Detected unaligned segment. You must either ensure all of your segments are 8-byte aligned, or you must enable the \"unaligned\" feature in the capnp crate"),
            Self::UnexepectedFarPointer => write!(fmt, "Unexpected far pointer"),
            Self::UnknownPointerType => write!(fmt, "Unknown pointer type."),
        }
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        #[cfg(feature = "alloc")]
        let result = if self.extra.is_empty() {
            write!(fmt, "{}", self.kind)
        } else {
            write!(fmt, "{}: {}", self.kind, self.extra)
        };
        #[cfg(not(feature = "alloc"))]
        let result = write!(fmt, "{}", self.kind);
        result
    }
}

#[cfg(feature = "std")]
impl ::std::error::Error for Error {
    #[cfg(feature = "alloc")]
    fn description(&self) -> &str {
        &self.extra
    }
    fn cause(&self) -> Option<&dyn (::std::error::Error)> {
        None
    }
}

/// Helper struct that allows `MessageBuilder::get_segments_for_output()` to avoid heap allocations
/// in the single-segment case.
pub enum OutputSegments<'a> {
    SingleSegment([&'a [u8]; 1]),

    #[cfg(feature = "alloc")]
    MultiSegment(Vec<&'a [u8]>),
}

impl<'a> core::ops::Deref for OutputSegments<'a> {
    type Target = [&'a [u8]];
    fn deref(&self) -> &[&'a [u8]] {
        match self {
            OutputSegments::SingleSegment(s) => s,

            #[cfg(feature = "alloc")]
            OutputSegments::MultiSegment(v) => v,
        }
    }
}

impl<'s> message::ReaderSegments for OutputSegments<'s> {
    fn get_segment(&self, id: u32) -> Option<&[u8]> {
        match self {
            OutputSegments::SingleSegment(s) => s.get(id as usize).copied(),

            #[cfg(feature = "alloc")]
            OutputSegments::MultiSegment(v) => v.get(id as usize).copied(),
        }
    }
}
