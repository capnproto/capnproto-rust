/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

//! # Cap'n Proto Schema Compiler Plugin Executable
//!
//! [See this.](http://kentonv.github.io/capnproto/otherlang.html#how-to-write-compiler-plugins)
//!
//!



#![feature(globs)]

#![crate_name="capnpc-rust"]
#![crate_type = "bin"]

extern crate capnp;
extern crate capnpc;

pub fn main() {
    //! Generate Rust code according to a `schema_capnp::code_generator_request` read from stdin.

    match ::capnpc::codegen::main(&mut ::std::io::stdin()) {
        Ok(()) => {}
        Err(e) => {
            std::os::set_exit_status(1);
            println!("error: {}", e)
        }
    }
}
