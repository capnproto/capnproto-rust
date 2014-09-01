/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#![feature(globs)]

#![crate_name="capnpc-rust"]
#![crate_type = "bin"]

extern crate capnp;

pub mod schema_capnp;
pub mod codegen;

pub fn main() {
    match codegen::main() {
        Ok(()) => {}
        Err(e) => {
            std::os::set_exit_status(1);
            println!("error: {}", e)
        }
    }
}
