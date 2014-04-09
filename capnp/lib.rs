/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#![feature(globs)]
#![feature(macro_rules)]
#![feature(phase)]

#![crate_id="capnp"]
#![crate_type = "lib"]

// import logging macros
#[phase(syntax, link)] extern crate log;
extern crate libc;

// reexports
pub use any::AnyPointer;
pub use blob::{Text, Data};
pub use common::{MessageSize};
pub use list::{PrimitiveList, EnumList, StructList, TextList, DataList, ListList};
pub use message::{MessageBuilder, BuilderOptions, MessageReader, ReaderOptions};
pub use message::MallocMessageBuilder;
pub use serialize::OwnedSpaceMessageReader;

pub mod any;
pub mod arena;
pub mod blob;
pub mod capability;
pub mod common;
pub mod endian;
pub mod io;
pub mod layout;
pub mod list;
pub mod mask;
pub mod message;
pub mod serialize;
pub mod serialize_packed;


#[cfg(test)]
pub mod layout_test;
#[cfg(test)]
pub mod serialize_packed_test;




