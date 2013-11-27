/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[feature(globs)];
#[feature(macro_rules)];

#[link(name = "benchmark", vers = "alpha", author = "dwrensha")];

#[crate_type = "bin"];

extern mod capnp;

pub mod common;

pub mod carsales_capnp;
pub mod carsales;

pub mod catrank_capnp;
pub mod catrank;

pub mod eval_capnp;
pub mod eval;

mod Uncompressed {
    use capnp;
    use std;

    pub fn write<T : std::io::Writer>(writer: &mut T,
                                      message: &capnp::message::MessageBuilder) {
        capnp::serialize::write_message(writer, message);
    }

    pub fn new_reader<U : std::io::Reader, T>(
        inputStream : &mut U,
        options : capnp::message::ReaderOptions,
        cont : |&mut capnp::message::MessageReader| -> T) -> T {
        capnp::serialize::InputStreamMessageReader::new(inputStream, options, cont)
    }

    pub fn new_buffered_reader<R: std::io::Reader, T>(
        inputStream : &mut capnp::io::BufferedInputStream<R>,
        options : capnp::message::ReaderOptions,
        cont : |&mut capnp::message::MessageReader| -> T) -> T {
        capnp::serialize::InputStreamMessageReader::new(inputStream, options, cont)
    }
}

mod Packed {
    use capnp;
    use std;
    use capnp::serialize_packed::{WritePackedWrapper, WritePacked};

    pub fn write<T : std::io::Writer>(writer: &mut T,
                                      message: &capnp::message::MessageBuilder) {
        let mut w = WritePackedWrapper{writer: writer};
        w.write_packed_message(message);
    }

    pub fn new_reader<U : std::io::Reader, T>(
        inputStream : &mut U,
        options : capnp::message::ReaderOptions,
        cont : |&mut capnp::message::MessageReader| -> T) -> T {
        capnp::serialize::InputStreamMessageReader::new(
            &mut capnp::serialize_packed::PackedInputStream{
                inner : &mut capnp::io::BufferedInputStream::new(inputStream)
            },
            options, cont)
    }

    pub fn new_buffered_reader<R:std::io::Reader, T>(
        inputStream : &mut capnp::io::BufferedInputStream<R>,
        options : capnp::message::ReaderOptions,
        cont : |&mut capnp::message::MessageReader| -> T) -> T {
        capnp::serialize::InputStreamMessageReader::new(
            &mut capnp::serialize_packed::PackedInputStream{
                inner : inputStream
            },
            options, cont)
    }

}


macro_rules! pass_by_object(
    ( $testcase:ident, $iters:expr ) => ({
            let mut rng = common::FastRand::new();
            for _ in range(0, $iters) {
                let mut messageReq = capnp::message::MessageBuilder::new_default();
                let mut messageRes = capnp::message::MessageBuilder::new_default();

                let request = messageReq.init_root::<$testcase::RequestBuilder>();
                let response = messageRes.init_root::<$testcase::ResponseBuilder>();
                let expected = $testcase::setup_request(&mut rng, request);

                request.as_reader(|requestReader| {
                    $testcase::handle_request(requestReader, response);
                });

                response.as_reader(|responseReader| {
                    if (! $testcase::check_response(responseReader, expected)) {
                        fail!("Incorrect response.");
                    }
                });
            }
        });
    )


static SCRATCH_SIZE : uint = 128 * 1024;
//static scratchSpace : [u8, .. 6 * SCRATCH_SIZE] = [0, .. 6 * SCRATCH_SIZE];

macro_rules! pass_by_bytes(
    ( $testcase:ident, $compression:ident, $iters:expr ) => ({
            let mut requestBytes : ~[u8] = std::vec::from_elem(SCRATCH_SIZE * 8, 0u8);
            let mut responseBytes : ~[u8] = std::vec::from_elem(SCRATCH_SIZE * 8, 0u8);
            let mut rng = common::FastRand::new();
            for _ in range(0, $iters) {
                let mut messageReq = capnp::message::MessageBuilder::new_default();
                let mut messageRes = capnp::message::MessageBuilder::new_default();

                let request = messageReq.init_root::<$testcase::RequestBuilder>();
                let response = messageRes.init_root::<$testcase::ResponseBuilder>();
                let expected = $testcase::setup_request(&mut rng, request);

                {
                    let mut writer = std::io::mem::BufWriter::new(requestBytes);
                    $compression::write(&mut writer, messageReq)
                }

                $compression::new_reader(
                    &mut std::io::mem::BufReader::new(requestBytes),
                    capnp::message::DEFAULT_READER_OPTIONS,
                    |requestReader| {
                        let requestReader = $testcase::new_request_reader(requestReader.get_root());
                        $testcase::handle_request(requestReader, response);
                    });

                {
                    let mut writer = std::io::mem::BufWriter::new(responseBytes);
                    $compression::write(&mut writer, messageRes)
                }

                $compression::new_reader(
                    &mut std::io::mem::BufReader::new(responseBytes),
                    capnp::message::DEFAULT_READER_OPTIONS,
                    |responseReader| {
                        let responseReader =
                            $testcase::new_response_reader(responseReader.get_root());
                        if (! $testcase::check_response(responseReader, expected)) {
                            fail!("Incorrect response.");
                        }
                    });
            }
        });
    )

macro_rules! server(
    ( $testcase:ident, $compression:ident, $iters:expr, $input:expr, $output:expr) => ({
            let mut outBuffered = capnp::io::BufferedOutputStream::new(&mut $output);
            let mut inBuffered = capnp::io::BufferedInputStream::new(&mut $input);
            for _ in range(0, $iters) {
                let mut messageRes = capnp::message::MessageBuilder::new_default();
                let response = messageRes.init_root::<$testcase::ResponseBuilder>();
                $compression::new_buffered_reader(
                    &mut inBuffered,
                    capnp::message::DEFAULT_READER_OPTIONS,
                    |requestReader| {
                        let requestReader = $testcase::new_request_reader(requestReader.get_root());
                        $testcase::handle_request(requestReader, response);
                    });
                $compression::write(&mut outBuffered, messageRes);
                outBuffered.flush();
            }
        });
    )

macro_rules! sync_client(
    ( $testcase:ident, $compression:ident, $iters:expr) => ({
            let mut outStream = std::io::stdout();
            let mut outBuffered = capnp::io::BufferedOutputStream::new(&mut outStream);
            let mut inStream = std::io::stdin();
            let mut inBuffered = capnp::io::BufferedInputStream::new(&mut inStream);
            let mut rng = common::FastRand::new();
            for _ in range(0, $iters) {
                let mut messageReq = capnp::message::MessageBuilder::new_default();
                let request = messageReq.init_root::<$testcase::RequestBuilder>();

                let expected = $testcase::setup_request(&mut rng, request);
                $compression::write(&mut outBuffered, messageReq);
                outBuffered.flush();

                $compression::new_buffered_reader(
                    &mut inBuffered,
                    capnp::message::DEFAULT_READER_OPTIONS,
                    |responseReader| {
                        let responseReader =
                            $testcase::new_response_reader(responseReader.get_root());
                        assert!($testcase::check_response(responseReader, expected));
                    });
            }
        });
    )


macro_rules! pass_by_pipe(
    ( $testcase:ident, $compression:ident, $iters:expr) => ({
            use std::io::process;

            let mut args = std::os::args();
            args[2] = ~"client";

            let config = process::ProcessConfig {
                program: args[0].as_slice(),
                args: args.slice(1, args.len()),
                env : None,
                cwd: None,
                io : [process::CreatePipe(true, false), // stdin
                      process::CreatePipe(false, true), // stdout
                      process::Ignored]
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

macro_rules! do_testcase(
    ( $testcase:ident, $mode:expr, $reuse:expr, $compression:ident, $iters:expr ) => ({
            match $mode {
                ~"object" => pass_by_object!($testcase, $iters),
                ~"bytes" => pass_by_bytes!($testcase, $compression, $iters),
                ~"client" => sync_client!($testcase, $compression, $iters),
                ~"server" => {
                    let mut input = std::io::stdin();
                    let mut output = std::io::stdout();
                    server!($testcase, $compression, $iters, input, output)
                }
                ~"pipe" => pass_by_pipe!($testcase, $compression, $iters),
                s => fail!("unrecognized mode: {}", s)
            }
        });
    )

macro_rules! do_testcase1(
    ( $testcase:expr, $mode:expr, $reuse:expr, $compression:ident, $iters:expr) => ({
            match $testcase {
                ~"carsales" => do_testcase!(carsales, $mode, $reuse, $compression, $iters),
                ~"catrank" => do_testcase!(catrank, $mode, $reuse, $compression, $iters),
                ~"eval" => do_testcase!(eval, $mode, $reuse, $compression, $iters),
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

    // For now, just insist that reuse = none
    match args[3] {
        ~"no-reuse" => {}
        _ => fail!("for now, 'no-reuse' is the only allowed option for REUSE")
    }

    match args[4] {
        ~"none" => do_testcase1!(args[1], args[2],  args[3], Uncompressed, iters),
        ~"packed" => do_testcase1!(args[1], args[2], args[3], Packed, iters),
        s => fail!("unrecognized compression: {}", s)
    }
}
