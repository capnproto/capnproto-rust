/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[feature(globs)];
#[feature(macro_rules)];

#[link(name = "benchmark", vers = "alpha", author = "dwrensha")];

#[crate_type = "bin"];

extern mod capnprust;

pub mod common;

pub mod carsales_capnp;
pub mod carsales;

pub mod catrank_capnp;
pub mod catrank;

pub mod eval_capnp;
pub mod eval;

mod Uncompressed {
    use capnprust;
    use std;

    pub fn write<T : std::rt::io::Writer>(writer: &mut T,
                                          message: &capnprust::message::MessageBuilder) {
        capnprust::serialize::writeMessage(writer, message);
    }

    pub fn newReader<U : std::rt::io::Reader, T>(
        inputStream : &mut U,
        options : capnprust::message::ReaderOptions,
        cont : &fn(v : &mut capnprust::message::MessageReader) -> T) -> T {
        capnprust::serialize::InputStreamMessageReader::new(inputStream, options, cont)
    }
}

mod Packed {
    use capnprust;
    use std;
    use capnprust::serialize_packed::{WritePackedWrapper, WritePacked};

    pub fn write<T : std::rt::io::Writer>(writer: &mut T,
                                          message: &capnprust::message::MessageBuilder) {
        let mut w = WritePackedWrapper{writer: writer};
        w.writePackedMessage(message);
    }

    pub fn newReader<U : std::rt::io::Reader, T>(
        inputStream : &mut U,
        options : capnprust::message::ReaderOptions,
        cont : &fn(v : &mut capnprust::message::MessageReader) -> T) -> T {
        capnprust::serialize::InputStreamMessageReader::new(
            &mut capnprust::serialize_packed::PackedInputStream{inner : inputStream},
            options, cont)
    }
}


macro_rules! passByObject(
    ( $testcase:ident, $iters:expr ) => ({
            let mut rng = common::FastRand::new();
            for _ in range(0, $iters) {
                let mut messageReq = capnprust::message::MessageBuilder::new_default();
                let mut messageRes = capnprust::message::MessageBuilder::new_default();

                let request = messageReq.initRoot::<$testcase::RequestBuilder>();
                let response = messageRes.initRoot::<$testcase::ResponseBuilder>();
                let expected = $testcase::setupRequest(&mut rng, request);

                do request.asReader |requestReader| {
                    $testcase::handleRequest(requestReader, response);
                }

                do response.asReader |responseReader| {
                    if (! $testcase::checkResponse(responseReader, expected)) {
                        fail!("Incorrect response.");
                    }
                }
            }
        });
    )

macro_rules! passByBytes(
    ( $testcase:ident, $compression:ident, $iters:expr ) => ({
            let mut rng = common::FastRand::new();
            for _ in range(0, $iters) {
                let mut messageReq = capnprust::message::MessageBuilder::new_default();
                let mut messageRes = capnprust::message::MessageBuilder::new_default();

                let request = messageReq.initRoot::<$testcase::RequestBuilder>();
                let response = messageRes.initRoot::<$testcase::ResponseBuilder>();
                let expected = $testcase::setupRequest(&mut rng, request);

                let requestBytes = do std::rt::io::mem::with_mem_writer |writer| {
                    $compression::write(writer, messageReq)
                };

                do $compression::newReader(
                      &mut std::rt::io::mem::BufReader::new(requestBytes),
                      capnprust::message::DEFAULT_READER_OPTIONS) |requestReader| {
                    let requestReader = $testcase::newRequestReader(requestReader.getRoot());
                    $testcase::handleRequest(requestReader, response);
                }

                let responseBytes = do std::rt::io::mem::with_mem_writer |writer| {
                    $compression::write(writer, messageRes);
                };

                do $compression::newReader(
                    &mut std::rt::io::mem::BufReader::new(responseBytes),
                    capnprust::message::DEFAULT_READER_OPTIONS) |responseReader| {
                    let responseReader = $testcase::newResponseReader(responseReader.getRoot());
                    if (! $testcase::checkResponse(responseReader, expected)) {
                        fail!("Incorrect response.");
                    }
                }
            }
        });
    )

macro_rules! server(
    ( $testcase:ident, $compression:ident, $iters:expr, $input:expr, $output:expr) => ({
            for _ in range(0, $iters) {
                let mut messageRes = capnprust::message::MessageBuilder::new_default();
                let response = messageRes.initRoot::<$testcase::ResponseBuilder>();
                do $compression::newReader(
                    &mut $input,
                    capnprust::message::DEFAULT_READER_OPTIONS) |requestReader| {
                    let requestReader = $testcase::newRequestReader(requestReader.getRoot());
                    $testcase::handleRequest(requestReader, response);
                }
                $compression::write(&mut $output, messageRes);
            }
        });
    )

macro_rules! syncClient(
    ( $testcase:ident, $compression:ident, $iters:expr) => ({
            let mut outStream = std::rt::io::stdout();
            let mut inStream = std::rt::io::stdin();
            let mut rng = common::FastRand::new();
            for _ in range(0, $iters) {
                let mut messageReq = capnprust::message::MessageBuilder::new_default();
                let request = messageReq.initRoot::<$testcase::RequestBuilder>();

                let expected = $testcase::setupRequest(&mut rng, request);
                $compression::write(&mut outStream, messageReq);

                do $compression::newReader(
                    &mut inStream,
                    capnprust::message::DEFAULT_READER_OPTIONS) |responseReader| {
                    let responseReader = $testcase::newResponseReader(responseReader.getRoot());
                    assert!($testcase::checkResponse(responseReader, expected));
                }

            }
        });
    )


macro_rules! passByPipe(
    ( $testcase:ident, $compression:ident, $iters:expr) => ({
            use std::rt::io::process;

            // get a rustc crash if we put this in line below
            let io = ~[process::CreatePipe(true, false), // stdin
                       process::CreatePipe(false, true), // stdout
                       process::Ignored];

            let mut args = std::os::args();
            args[2] = ~"client";

            let config = process::ProcessConfig {
                program: "./benchmark/benchmark",
                args: args.slice(1, args.len()),
                env : None,
                cwd: None,
                io : io
            };
            match process::Process::new(config) {
                Some(ref mut p) => {
                    server!($testcase, $compression, $iters, p.io[1], p.io[0]);
                    println!("{}", p.wait());
                }
                None => {
                    println("bummer");
                }
            }
        });
    )

macro_rules! doTestcase(
    ( $testcase:ident, $mode:expr, $reuse:expr, $compression:ident, $iters:expr ) => ({
            match $mode {
                ~"object" => passByObject!($testcase, $iters),
                ~"bytes" => passByBytes!($testcase, $compression, $iters),
                ~"client" => syncClient!($testcase, $compression, $iters),
                ~"pipe" => passByPipe!($testcase, $compression, $iters),
                s => fail!("unrecognized mode: {}", s)
            }
        });
    )

macro_rules! doTestcase1(
    ( $testcase:expr, $mode:expr, $reuse:expr, $compression:ident, $iters:expr) => ({
            match $testcase {
                ~"carsales" => doTestcase!(carsales, $mode, $reuse, $compression, $iters),
                ~"catrank" => doTestcase!(catrank, $mode, $reuse, $compression, $iters),
                ~"eval" => doTestcase!(eval, $mode, $reuse, $compression, $iters),
                s => fail!("unrecognized test case: {}", s)
            }
        });
    )

pub fn main () {

    let args = std::os::args();

    if (args.len() != 6) {
        println!("USAGE: {} CASE MODE REUSE COMPRESSION ITERATION_COUNT", args[0]);
        std::os::set_exit_status(1);
        return;
    }

    let iters = match from_str::<u64>(args[5]) {
        Some (n) => n,
        None => {
            println!("Could not parse a u64 from: {}", args[5]);
            std::os::set_exit_status(1);
            return;
        }
    };

    match args[4] {
        ~"none" => doTestcase1!(args[1], args[2],  args[3], Uncompressed, iters),
        ~"packed" => doTestcase1!(args[1], args[2], args[3], Packed, iters),
        s => fail!("unrecognized compression: {}", s)
    }


}
