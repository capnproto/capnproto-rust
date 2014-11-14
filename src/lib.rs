/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#![feature(globs)]

#![crate_name="capnpc"]
#![crate_type = "lib"]

extern crate capnp;

pub mod schema_capnp;
pub mod codegen;

