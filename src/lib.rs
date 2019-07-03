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
#![feature(try_trait)]

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
    // type WriterType: capnp::traits::FromPointerBuilder<'a>;
    type WriterType: capnp::traits::FromPointerBuilder<'a>;

    /// Converts a Rust struct to corresponding Capnp struct. This should not fail.
    fn write_capnp(&self, writer: &mut Self::WriterType);
}

/// Convert Capnp struct to Rust.
pub trait ReadCapnp<'a>: Sized {
    /// The corresponding Capnp reader type.
    type ReaderType: capnp::traits::FromPointerReader<'a>;

    /// Converts a Capnp struct to corresponding Rust struct.     
    fn read_capnp(reader: &Self::ReaderType) -> Result<Self, CapnpConvError>;
}

pub trait ToCapnpBytes {
    /// Serialize a Rust struct into bytes using Capnp
    fn to_capnp_bytes(&self) -> Result<Vec<u8>, CapnpConvError>;
}

pub trait FromCapnpBytes: Sized {
    /// Deserialize a Rust struct from bytes using Capnp
    fn from_capnp_bytes(bytes: &[u8]) -> Result<Self, CapnpConvError>;
}

/// A shim allowing to merge cases where either
/// Result<T,Into<CapnoConvError>> or a T is returned.
pub enum CapnpResult<T> {
    Ok(T),
    Err(CapnpConvError),
}

// -------------------------------------------------------
// -------------------------------------------------------

impl<T> CapnpResult<T> {
    pub fn into_result(self) -> Result<T, CapnpConvError> {
        match self {
            CapnpResult::Ok(t) => Ok(t),
            CapnpResult::Err(e) => Err(e),
        }
    }
}

impl<T> From<T> for CapnpResult<T> {
    fn from(input: T) -> Self {
        CapnpResult::Ok(input)
    }
}

impl<T, E> From<Result<T, E>> for CapnpResult<T>
where
    E: Into<CapnpConvError>,
{
    fn from(input: Result<T, E>) -> Self {
        match input {
            Ok(t) => CapnpResult::Ok(t),
            Err(e) => CapnpResult::Err(e.into()),
        }
    }
}

// -------------------------------------------------------
// -------------------------------------------------------

impl<T> ToCapnpBytes for T
where
    T: for<'a> WriteCapnp<'a>,
{
    fn to_capnp_bytes(&self) -> Result<Vec<u8>, CapnpConvError> {
        let mut builder = capnp::message::Builder::new_default();

        // A trick to avoid borrow checker issues:
        {
            let mut struct_builder = builder.init_root::<T::WriterType>();
            self.write_capnp(&mut struct_builder);
        }

        let mut data = Vec::new();
        capnp::serialize_packed::write_message(&mut data, &builder)?;
        Ok(data)
    }
}

impl<T> FromCapnpBytes for T
where
    T: for<'a> ReadCapnp<'a>,
{
    fn from_capnp_bytes(bytes: &[u8]) -> Result<Self, CapnpConvError> {
        let mut cursor = io::Cursor::new(&bytes);
        let reader = capnp::serialize_packed::read_message(
            &mut cursor,
            capnp::message::ReaderOptions::new(),
        )?;
        let struct_reader = reader.get_root::<T::ReaderType>()?;
        Ok(Self::read_capnp(&struct_reader)?)
    }
}
