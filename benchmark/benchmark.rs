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

    pub fn write<T : std::io::Writer>(writer: &mut T,
                                          message: &capnprust::message::MessageBuilder) {
        capnprust::serialize::writeMessage(writer, message);
    }

    pub fn newReader<U : std::io::Reader, T>(
        inputStream : &mut U,
        options : capnprust::message::ReaderOptions,
        cont : &fn(v : &mut capnprust::message::MessageReader) -> T) -> T {
        capnprust::serialize::InputStreamMessageReader::new(inputStream, options, cont)
    }

    pub fn newBufferedReader<R: std::io::Reader, T>(
        inputStream : &mut capnprust::io::BufferedInputStream<R>,
        options : capnprust::message::ReaderOptions,
        cont : &fn(v : &mut capnprust::message::MessageReader) -> T) -> T {
        capnprust::serialize::InputStreamMessageReader::new(inputStream, options, cont)
    }
}

mod Packed {
    use capnprust;
    use std;
    use capnprust::serialize_packed::{WritePackedWrapper, WritePacked};

    pub fn write<T : std::io::Writer>(writer: &mut T,
                                          message: &capnprust::message::MessageBuilder) {
        let mut w = WritePackedWrapper{writer: writer};
        w.writePackedMessage(message);
    }

    pub fn newReader<U : std::io::Reader, T>(
        inputStream : &mut U,
        options : capnprust::message::ReaderOptions,
        cont : &fn(v : &mut capnprust::message::MessageReader) -> T) -> T {
        capnprust::serialize::InputStreamMessageReader::new(
            &mut capnprust::serialize_packed::PackedInputStream{
                inner : &mut capnprust::io::BufferedInputStream::new(inputStream)
            },
            options, cont)
    }

    pub fn newBufferedReader<R:std::io::Reader, T>(
        inputStream : &mut capnprust::io::BufferedInputStream<R>,
        options : capnprust::message::ReaderOptions,
        cont : &fn(v : &mut capnprust::message::MessageReader) -> T) -> T {
        capnprust::serialize::InputStreamMessageReader::new(
            &mut capnprust::serialize_packed::PackedInputStream{
                inner : inputStream
            },
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


static SCRATCH_SIZE : uint = 128 * 1024;
//static scratchSpace : [u8, .. 6 * SCRATCH_SIZE] = [0, .. 6 * SCRATCH_SIZE];

macro_rules! passByBytes(
    ( $testcase:ident, $compression:ident, $iters:expr ) => ({
            let mut requestBytes : ~[u8] = std::vec::from_elem(SCRATCH_SIZE * 8, 0u8);
            let mut responseBytes : ~[u8] = std::vec::from_elem(SCRATCH_SIZE * 8, 0u8);
            let mut rng = common::FastRand::new();
            for _ in range(0, $iters) {
                let mut messageReq = capnprust::message::MessageBuilder::new_default();
                let mut messageRes = capnprust::message::MessageBuilder::new_default();

                let request = messageReq.initRoot::<$testcase::RequestBuilder>();
                let response = messageRes.initRoot::<$testcase::ResponseBuilder>();
                let expected = $testcase::setupRequest(&mut rng, request);

                {
                    let mut writer = std::io::mem::BufWriter::new(requestBytes);
                    $compression::write(&mut writer, messageReq)
                }

                do $compression::newReader(
                      &mut std::io::mem::BufReader::new(requestBytes),
                      capnprust::message::DEFAULT_READER_OPTIONS) |requestReader| {
                    let requestReader = $testcase::newRequestReader(requestReader.getRoot());
                    $testcase::handleRequest(requestReader, response);
                }

                {
                    let mut writer = std::io::mem::BufWriter::new(responseBytes);
                    $compression::write(&mut writer, messageRes)
                }

                do $compression::newReader(
                    &mut std::io::mem::BufReader::new(responseBytes),
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
            let mut outBuffered = capnprust::io::BufferedOutputStream::new(&mut $output);
            let mut inBuffered = capnprust::io::BufferedInputStream::new(&mut $input);
            for _ in range(0, $iters) {
                let mut messageRes = capnprust::message::MessageBuilder::new_default();
                let response = messageRes.initRoot::<$testcase::ResponseBuilder>();
                do $compression::newBufferedReader(
                    &mut inBuffered,
                    capnprust::message::DEFAULT_READER_OPTIONS) |requestReader| {
                    let requestReader = $testcase::newRequestReader(requestReader.getRoot());
                    $testcase::handleRequest(requestReader, response);
                }
                $compression::write(&mut outBuffered, messageRes);
                outBuffered.flush();
            }
        });
    )

macro_rules! syncClient(
    ( $testcase:ident, $compression:ident, $iters:expr) => ({
            let mut outStream = std::io::stdout();
            let mut outBuffered = capnprust::io::BufferedOutputStream::new(&mut outStream);
            let mut inStream = std::io::stdin();
            let mut inBuffered = capnprust::io::BufferedInputStream::new(&mut inStream);
            let mut rng = common::FastRand::new();
            for _ in range(0, $iters) {
                let mut messageReq = capnprust::message::MessageBuilder::new_default();
                let request = messageReq.initRoot::<$testcase::RequestBuilder>();

                let expected = $testcase::setupRequest(&mut rng, request);
                $compression::write(&mut outBuffered, messageReq);
                outBuffered.flush();

                do $compression::newBufferedReader(
                    &mut inBuffered,
                    capnprust::message::DEFAULT_READER_OPTIONS) |responseReader| {
                    let responseReader = $testcase::newResponseReader(responseReader.getRoot());
                    assert!($testcase::checkResponse(responseReader, expected));
                }

            }
        });
    )


macro_rules! passByPipe(
    ( $testcase:ident, $compression:ident, $iters:expr) => ({
            use std::io::process;

            // get a rustc crash if we put this in line below
            let io = ~[process::CreatePipe(true, false), // stdin
                       process::CreatePipe(false, true), // stdout
                       process::Ignored];


            let mut args = std::os::args();
            args[2] = ~"client";

            let config = process::ProcessConfig {
                program: args[0].as_slice(),
                args: args.slice(1, args.len()),
                env : None,
                cwd: None,
                io : io
            };
            match process::Process::new(config) {
                Some(ref mut p) => {
                    p.io.pop();
                    let mut childStdOut = p.io.pop();
                    let mut childStdIn = p.io.pop();

                    server!($testcase, $compression, $iters, childStdOut, childStdIn);
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

    // For now, just insist that re-use = none
    match args[3] {
        ~"no-reuse" => {}
        _ => fail!("for now, 'no-reuse' is the only allowed option for REUSE")
    }

    match args[4] {
        ~"none" => doTestcase1!(args[1], args[2],  args[3], Uncompressed, iters),
        ~"packed" => doTestcase1!(args[1], args[2], args[3], Packed, iters),
        s => fail!("unrecognized compression: {}", s)
    }


}
