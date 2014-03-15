/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */


#[crate_id="calculator"];
#[crate_type="bin"];

extern crate capnp;
extern crate capnp_rpc = "capnp-rpc";

pub mod calculator_capnp;

pub mod client;
pub mod server;

pub fn main() {
    let args = std::os::args();
    if args.len() >= 2 {
        match args[1].as_slice() {
            "client" => return client::main(),
            "server" => return server::main(),
            _ => (),
        }
    }

    println!("usage: {} [client | server] ADDRESS", args[0]);
}
