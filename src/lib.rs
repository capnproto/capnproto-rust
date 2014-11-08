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

pub mod rpc_capnp {
  include!(concat!(env!("OUT_DIR"), "/rpc_capnp.rs"))
}

pub mod capability;
pub mod ez_rpc;
pub mod rpc;


