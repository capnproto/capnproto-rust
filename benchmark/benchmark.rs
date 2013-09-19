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

pub mod catrank_capnp;
pub mod catrank;

//pub mod eval_capnp;
//pub mod eval;

macro_rules! passByObject(
    ( $testcase:ident, $iters:expr ) => ({
            let mut rng = ~common::FastRand::new();
            for _ in range(0, $iters) {
                let messageReq = capnprust::message::MessageBuilder::new_default();
                let messageRes = capnprust::message::MessageBuilder::new_default();


                let request = messageReq.initRoot::<$testcase::RequestBuilder>();
                let response = messageRes.initRoot::<$testcase::ResponseBuilder>();
                let expected = $testcase::setupRequest(rng, request);

                do request.asReader |requestReader| {
                    $testcase::handleRequest(requestReader, response);
                }

                do response.asReader |responseReader| {
                    if (! $testcase::checkResponse(responseReader, expected)) {
                        println("Incorrect response.");
                    }
                }

                messageReq.release();
                messageRes.release();
            }
        });
    )


pub fn main () {

    let args = std::os::args();

    if (args.len() != 5) {
        printfln!("USAGE: %s MODE REUSE COMPRESSION ITERATION_COUNT", args[0]);
        return;
    }

    let iters = match from_str::<u64>(args[4]) {
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

//    passByObject!(catrank, iters);
    passByObject!(carsales, iters);

}
