/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[feature(globs)];
#[feature(macro_rules)];

#[pkgid="capnp"];
#[link(name = "capnp", package_id = "github.com/dwrensha/capnproto-rust",
       vers = "alpha", author = "dwrensha")];

#[crate_type = "lib"];


pub mod common;
pub mod endian;
pub mod mask;
pub mod blob;
pub mod layout;
pub mod pointer_helpers;
pub mod any;
pub mod arena;
pub mod message;
pub mod io;
pub mod serialize;
pub mod serialize_packed;
pub mod list;

#[cfg(test)]
pub mod serialize_packed_test;
