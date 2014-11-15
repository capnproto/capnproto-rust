/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

//! # Cap'n Proto Runtime Library
//!
//! [Cap'n Proto](http://kentonv.github.io/capnproto/) is an extremely efficient protocol for
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


#![feature(globs)]
#![feature(macro_rules)]
#![feature(phase)]
#![allow(experimental)]
#![feature(unsafe_destructor)]

#![crate_name="capnp"]
#![crate_type = "lib"]

// import logging macros
#[phase(plugin, link)] extern crate log;
extern crate libc;

// reexports
pub use blob::{text, data};
pub use common::{MessageSize};
pub use list::{primitive_list, enum_list, struct_list, text_list, data_list, list_list};
pub use message::{MessageBuilder, BuilderOptions, MessageReader, ReaderOptions};
pub use message::MallocMessageBuilder;
pub use serialize::OwnedSpaceMessageReader;

pub mod any_pointer;
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




