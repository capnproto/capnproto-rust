/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#![feature(globs)]

#![crate_name="capnpc-rust"]
#![crate_type = "bin"]

extern crate capnp;
extern crate capnpc;

pub fn main() {
    match ::capnpc::codegen::main(&mut ::std::io::stdin()) {
        Ok(()) => {}
        Err(e) => {
            std::os::set_exit_status(1);
            println!("error: {}", e)
        }
    }
}
