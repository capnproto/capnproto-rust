/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */


#![crate_name="capnp-rpc"]
#![crate_type="lib"]

extern crate core;
extern crate capnp;
extern crate sync;

extern crate capnp_rpc_include_generated;

pub use capnp_rpc_include_generated::rpc_capnp;

pub mod capability;
pub mod ez_rpc;
pub mod rpc;


