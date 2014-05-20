/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */


#![crate_id="capnp-rpc"]
#![crate_type="lib"]

extern crate collections;
extern crate core;
extern crate capnp;
extern crate sync;

pub mod rpc_capnp;
pub mod rpc_twoparty_capnp;

pub mod capability;
pub mod ez_rpc;
pub mod rpc;


