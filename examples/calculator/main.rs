/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */


#[crate_id="calculator"];
#[crate_type="bin"];

extern mod capnp;
extern mod extra;
extern mod capnp_rpc = "capnp-rpc";

pub mod calculator_capnp;

pub mod client;
pub mod server;

pub fn main() {
    match std::os::args() {
        [_, ~"client", ..] => client::main(),
        [_, ~"server", ..] => server::main(),
        args => println!("usage: {} [client | server] ADDRESS", args[0]),
    }
}
