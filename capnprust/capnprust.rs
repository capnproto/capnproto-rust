/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */


#[link(name = "capnprust", vers = "alpha", author = "dwrensha")];

#[crate_type = "lib"];

pub mod common;
pub mod endian;
pub mod mask;
pub mod layout;
pub mod arena;
pub mod message;
pub mod serialize;
pub mod serialize_packed;
pub mod list;

#[cfg(test)]
pub mod serialize_packed_test;