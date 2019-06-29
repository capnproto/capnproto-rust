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

#[derive(Debug, From)]
pub enum SerCapnpError {
    CapnpError(capnp::Error),
    NotInSchema(capnp::NotInSchema),
    IoError(io::Error),
}

/// Convert Rust struct to Capnp.
pub trait IntoCapnp {
    /// The corresponding Capnp writer type.
    type WriterType;

    /// Converts a Rust struct to corresponding Capnp struct. This should not fail.
    fn into_capnp(self, writer_type: &mut Self::WriterType);
}

/// Convert Capnp struct to Rust.
pub trait FromCapnp: Sized {
    /// The corresponding Capnp reader type.
    type ReaderType;

    /// Converts a Capnp struct to corresponding Rust struct.     
    fn from_capnp(object: &Self::ReaderType) -> Result<Self, SerCapnpError>;
}
