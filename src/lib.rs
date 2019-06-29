#![crate_type = "lib"]
#![feature(async_await, await_macro, arbitrary_self_types)]
#![feature(nll)]
#![feature(generators)]
#![feature(never_type)]
#![deny(trivial_numeric_casts, warnings)]
#![allow(intra_doc_link_resolution_failure)]
#![allow(
    clippy::too_many_arguments,
    clippy::implicit_hasher,
    clippy::module_inception,
    clippy::new_without_default
)]

use std::io;

use capnp;

use derive_more::From;

pub use capnp_conv_derive::capnp_conv;

#[derive(Debug, From)]
pub enum CapnpConvError {
    CapnpError(capnp::Error),
    NotInSchema(capnp::NotInSchema),
    IoError(io::Error),
}

/// Convert Rust struct to Capnp.
pub trait WriteCapnp<'a> {
    /// The corresponding Capnp writer type.
    type WriterType;

    /// Converts a Rust struct to corresponding Capnp struct. This should not fail.
    fn write_capnp(&'a self, writer: &'a mut Self::WriterType);
}

/// Convert Capnp struct to Rust.
pub trait ReadCapnp<'a>: Sized {
    /// The corresponding Capnp reader type.
    type ReaderType;

    /// Converts a Capnp struct to corresponding Rust struct.     
    fn read_capnp(reader: &'a Self::ReaderType) -> Result<Self, CapnpConvError>;
}

// String implementation:
impl<'a> WriteCapnp<'a> for String {
    type WriterType = capnp::text::Builder<'a>;

    fn write_capnp(&'a self, writer: &'a mut Self::WriterType) {
        writer.push_str(&self);
    }
}

impl<'a> ReadCapnp<'a> for String {
    type ReaderType = capnp::text::Reader<'a>;

    fn read_capnp(reader: &'a Self::ReaderType) -> Result<Self, CapnpConvError> {
        // A text reader is actually a &str:
        Ok(reader.to_string())
    }
}
