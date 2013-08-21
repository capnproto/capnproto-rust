/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[link(name = "capnproto-rust-benchmark", vers = "alpha", author = "dwrensha")];

#[crate_type = "bin"];

extern mod capnprust;

pub mod common;

pub mod carsales_capnp;
pub mod carsales;

pub fn main () {

    let args = std::os::args();

    if (args.len() != 5) {
        printfln!("USAGE: %s MODE REUSE COMPRESSION ITERATION_COUNT", args[0]);
        return;
    }

    let _iters = match std::u64::from_str(args[4]) {
        Some (n) => n,
        None => {
            printfln!("Could not parse a u64 from: %s", args[4]);
            return;
        }
    };

/* TODO use std::run
    unsafe {
        let child = funcs::posix88::unistd::fork();
        if (child == 0 ) {
            printfln!("%s", "Hello world. I am the child and client.");
        } else {
            printfln!("%s", "Hello world. I am the parent and server.");
        }
    }
*/
}
