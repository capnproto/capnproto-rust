/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[feature(macro_rules)];
#[feature(globs)];

#[crate_type = "bin"];
#[no_uv];

extern crate capnp;
extern crate native;

use capnp::message::{MessageReader, MessageBuilder};

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

    pub fn write<T : std::io::Writer, U : capnp::message::MessageBuilder>(
        writer: &mut T,
        message: &U) {
        capnp::serialize::write_message(writer, message).unwrap();
    }

    pub fn write_buffered<T : std::io::Writer, U : capnp::message::MessageBuilder>(
        writer: &mut T,
        message: &U) {
        capnp::serialize::write_message(writer, message).unwrap();
    }

    pub fn new_buffered_reader<R: capnp::io::BufferedInputStream>(
        inputStream : &mut R,
        options : capnp::message::ReaderOptions) -> capnp::serialize::OwnedSpaceMessageReader {
        capnp::serialize::new_reader(inputStream, options).unwrap()
    }
}

mod Packed {
    use capnp;
    use std;
    use capnp::serialize_packed::{write_packed_message, write_packed_message_unbuffered};

    pub fn write<T : std::io::Writer, U : capnp::message::MessageBuilder>(
        writer: &mut T,
        message: &U) {
        write_packed_message_unbuffered(writer, message).unwrap();
    }

    pub fn write_buffered<T : capnp::io::BufferedOutputStream, U : capnp::message::MessageBuilder>(
        writer: &mut T,
        message: &U) {
        write_packed_message(writer, message).unwrap();
    }

    pub fn new_buffered_reader<R:capnp::io::BufferedInputStream>(
        inputStream : &mut R,
        options : capnp::message::ReaderOptions) -> capnp::serialize::OwnedSpaceMessageReader {
        capnp::serialize_packed::new_reader(inputStream, options).unwrap()
    }

}

static SCRATCH_SIZE : uint = 128 * 1024;

pub struct NoScratch;

impl NoScratch {
    fn new_builder(&mut self, _idx : uint) -> capnp::message::MallocMessageBuilder {
        capnp::message::MallocMessageBuilder::new_default()
    }
}

pub struct UseScratch {
    scratch_space : ~[capnp::common::Word]
}

impl UseScratch {
    pub fn new() -> UseScratch {
        UseScratch {
            scratch_space : capnp::common::allocate_zeroed_words(SCRATCH_SIZE * 6)
        }
    }

    fn new_builder<'a>(&mut self, idx : uint) -> capnp::message::ScratchSpaceMallocMessageBuilder<'a> {
        assert!(idx < 6);
        unsafe {
            capnp::message::ScratchSpaceMallocMessageBuilder::new_default(
                std::cast::transmute(
                    std::raw::Slice { data : self.scratch_space.unsafe_ref(idx * SCRATCH_SIZE),
                                      len : SCRATCH_SIZE }))
        }
    }
}


macro_rules! pass_by_object(
    ( $testcase:ident, $reuse:ident, $iters:expr ) => ({
            let mut rng = common::FastRand::new();
            for _ in range(0, $iters) {
                let mut messageReq = $reuse.new_builder(0);
                let mut messageRes = $reuse.new_builder(1);

                let request = messageReq.init_root::<$testcase::RequestBuilder>();
                let response = messageRes.init_root::<$testcase::ResponseBuilder>();
                let expected = $testcase::setup_request(&mut rng, request);

                let requestReader = request.as_reader();
                $testcase::handle_request(requestReader, response);

                let responseReader = response.as_reader();
                if !$testcase::check_response(responseReader, expected) {
                    fail!("Incorrect response.");
                }
            }
        });
    )


macro_rules! pass_by_bytes(
    ( $testcase:ident, $reuse:ident, $compression:ident, $iters:expr ) => ({
            let mut requestBytes : ~[u8] = std::vec::from_elem(SCRATCH_SIZE * 8, 0u8);
            let mut responseBytes : ~[u8] = std::vec::from_elem(SCRATCH_SIZE * 8, 0u8);
            let mut rng = common::FastRand::new();
            for _ in range(0, $iters) {
                let mut messageReq = $reuse.new_builder(0);
                let mut messageRes = $reuse.new_builder(1);

                let request = messageReq.init_root::<$testcase::RequestBuilder>();
                let response = messageRes.init_root::<$testcase::ResponseBuilder>();
                let expected = $testcase::setup_request(&mut rng, request);

                {
                    let mut writer = capnp::io::ArrayOutputStream::new(requestBytes);
                    $compression::write_buffered(&mut writer, &messageReq)
                }

                let messageReader = $compression::new_buffered_reader(
                    &mut capnp::io::ArrayInputStream::new(requestBytes),
                    capnp::message::DefaultReaderOptions);

                let requestReader : $testcase::RequestReader = messageReader.get_root();
                $testcase::handle_request(requestReader, response);

                {
                    let mut writer = capnp::io::ArrayOutputStream::new(responseBytes);
                    $compression::write_buffered(&mut writer, &messageRes)
                }

                let messageReader = $compression::new_buffered_reader(
                    &mut capnp::io::ArrayInputStream::new(responseBytes),
                    capnp::message::DefaultReaderOptions);

                let responseReader : $testcase::ResponseReader = messageReader.get_root();
                if !$testcase::check_response(responseReader, expected) {
                    fail!("Incorrect response.");
                }
            }
        });
    )

macro_rules! server(
    ( $testcase:ident, $reuse:ident, $compression:ident, $iters:expr, $input:expr, $output:expr) => ({
            let mut outBuffered = capnp::io::BufferedOutputStreamWrapper::new(&mut $output);
            let mut inBuffered = capnp::io::BufferedInputStreamWrapper::new(&mut $input);
            for _ in range(0, $iters) {
                let mut messageRes = $reuse.new_builder(0);

                let response = messageRes.init_root::<$testcase::ResponseBuilder>();
                let messageReader = $compression::new_buffered_reader(
                    &mut inBuffered,
                    capnp::message::DefaultReaderOptions);
                let requestReader : $testcase::RequestReader = messageReader.get_root();
                $testcase::handle_request(requestReader, response);

                $compression::write_buffered(&mut outBuffered, &messageRes);
                outBuffered.flush().unwrap();
            }
        });
    )

macro_rules! sync_client(
    ( $testcase:ident, $reuse:ident, $compression:ident, $iters:expr) => ({
            let mut outStream = std::io::stdout();
            let mut inStream = std::io::stdin();
            let mut inBuffered = capnp::io::BufferedInputStreamWrapper::new(&mut inStream);
            let mut rng = common::FastRand::new();
            for _ in range(0, $iters) {
                let mut messageReq = $reuse.new_builder(0);

                let request = messageReq.init_root::<$testcase::RequestBuilder>();

                let expected = $testcase::setup_request(&mut rng, request);
                $compression::write(&mut outStream, &messageReq);

                let messageReader = $compression::new_buffered_reader(
                    &mut inBuffered,
                    capnp::message::DefaultReaderOptions);
                let responseReader : $testcase::ResponseReader = messageReader.get_root();
                assert!($testcase::check_response(responseReader, expected));

            }
        });
    )


macro_rules! pass_by_pipe(
    ( $testcase:ident, $reuse:ident, $compression:ident, $iters:expr) => ({
            use std::io::process;

            let mut args = std::os::args();
            args[2] = ~"client";

            let config = process::ProcessConfig {
                program: args[0].as_slice(),
                args: args.slice(1, args.len()),
                stdin : process::CreatePipe(true, false),
                stdout : process::CreatePipe(false, true),
                stderr : process::Ignored,
                .. process::ProcessConfig::new()
            };
            match process::Process::configure(config) {
                Ok(ref mut p) => {
                    let mut childStdOut = p.stdout.take().unwrap();
                    let mut childStdIn = p.stdin.take().unwrap();

                    server!($testcase, $reuse, $compression, $iters, childStdOut, childStdIn);
                    println!("{}", p.wait());
                }
                Err(e) => {
                    println!("could not start process: {}", e);
                }
            }
        });
    )

macro_rules! do_testcase(
    ( $testcase:ident, $mode:expr, $reuse:ident, $compression:ident, $iters:expr ) => ({
            match $mode {
                ~"object" => pass_by_object!($testcase, $reuse, $iters),
                ~"bytes" => pass_by_bytes!($testcase, $reuse, $compression, $iters),
                ~"client" => sync_client!($testcase, $reuse, $compression, $iters),
                ~"server" => {
                    let mut input = std::io::stdin();
                    let mut output = std::io::stdout();
                    server!($testcase, $reuse, $compression, $iters, input, output)
                }
                ~"pipe" => pass_by_pipe!($testcase, $reuse, $compression, $iters),
                s => fail!("unrecognized mode: {}", s)
            }
        });
    )

macro_rules! do_testcase1(
    ( $testcase:expr, $mode:expr, $reuse:ident, $compression:ident, $iters:expr) => ({
            match $testcase {
                ~"carsales" => do_testcase!(carsales, $mode, $reuse, $compression, $iters),
                ~"catrank" => do_testcase!(catrank, $mode, $reuse, $compression, $iters),
                ~"eval" => do_testcase!(eval, $mode, $reuse, $compression, $iters),
                s => fail!("unrecognized test case: {}", s)
            }
        });
    )

macro_rules! do_testcase2(
    ( $testcase:expr, $mode:expr, $reuse:expr, $compression:ident, $iters:expr) => ({
            match $reuse {
                ~"no-reuse" => {
                    let mut scratch = NoScratch;
                    do_testcase1!($testcase, $mode, scratch, $compression, $iters)
                }
                ~"reuse" => {
                    let mut scratch = UseScratch::new();
                    do_testcase1!($testcase, $mode, scratch, $compression, $iters)
                }
                s => fail!("unrecognized reuse option: {}", s)
            }
        });
    )

#[start]
pub fn start (argc : int, argv: **u8) -> int {
    native::start(argc, argv, proc () {
            main();
        })
}

pub fn main() {
    let args = std::os::args();

    if args.len() != 6 {
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
        ~"none" => do_testcase2!(args[1], args[2],  args[3], Uncompressed, iters),
        ~"packed" => do_testcase2!(args[1], args[2], args[3], Packed, iters),
        s => fail!("unrecognized compression: {}", s)
    }
}
