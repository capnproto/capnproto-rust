/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */


#![crate_name="calculator"]
#![crate_type="bin"]

extern crate capnp;
extern crate "capnp-rpc" as capnp_rpc;

pub mod calculator_capnp {
  include!(concat!(env!("OUT_DIR"), "/calculator_capnp.rs"));
}

pub mod client;
pub mod server;

pub fn main() {
    let args = std::os::args();
    if args.len() >= 2 {
        match &args[1][] {
            "client" => return client::main(),
            "server" => return server::main(),
            _ => (),
        }
    }

    println!("usage: {} [client | server] ADDRESS", args[0]);
}
