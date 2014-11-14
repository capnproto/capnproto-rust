/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#![feature(macro_rules)]
#![feature(globs)]

#![crate_type = "bin"]

extern crate capnp;
extern crate rand;

use capnp::{MessageReader, MessageBuilder};

pub mod common;

pub mod carsales_capnp {
  include!(concat!(env!("OUT_DIR"), "/carsales_capnp.rs"))
}
pub mod carsales;

pub mod catrank_capnp {
  include!(concat!(env!("OUT_DIR"), "/catrank_capnp.rs"))
}
pub mod catrank;

pub mod eval_capnp {
  include!(concat!(env!("OUT_DIR"), "/eval_capnp.rs"))
}

pub mod eval;

mod uncompressed {
    use capnp;

    pub fn write<'a, T : ::std::io::Writer, U : capnp::message::MessageBuilder<'a>>(
        writer: &mut T,
        message: &U) {
        capnp::serialize::write_message(writer, message).unwrap();
    }

    pub fn write_buffered<'a, T : ::std::io::Writer, U : capnp::message::MessageBuilder<'a>>(
        writer: &mut T,
        message: &U) {
        capnp::serialize::write_message(writer, message).unwrap();
    }

    pub fn new_buffered_reader<R: capnp::io::BufferedInputStream>(
        input_stream : &mut R,
        options : capnp::message::ReaderOptions) -> capnp::serialize::OwnedSpaceMessageReader {
        capnp::serialize::new_reader(input_stream, options).unwrap()
    }
}

mod packed {
    use capnp;
    use capnp::serialize_packed::{write_packed_message, write_packed_message_unbuffered};

    pub fn write<'a, T : ::std::io::Writer, U : capnp::message::MessageBuilder<'a>>(
        writer: &mut T,
        message: &U) {
        write_packed_message_unbuffered(writer, message).unwrap();
    }

    pub fn write_buffered<'a, T : capnp::io::BufferedOutputStream, U : capnp::message::MessageBuilder<'a>>(
        writer: &mut T,
        message: &U) {
        write_packed_message(writer, message).unwrap();
    }

    pub fn new_buffered_reader<R:capnp::io::BufferedInputStream>(
        input_stream : &mut R,
        options : capnp::message::ReaderOptions) -> capnp::serialize::OwnedSpaceMessageReader {
        capnp::serialize_packed::new_reader(input_stream, options).unwrap()
    }

}

const SCRATCH_SIZE : uint = 128 * 1024;

pub struct NoScratch;

impl NoScratch {
    fn new_builder(&mut self, _idx : uint) -> capnp::message::MallocMessageBuilder {
        capnp::message::MallocMessageBuilder::new_default()
    }
}

pub struct UseScratch {
    scratch_space : ::std::vec::Vec<capnp::common::Word>
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
                ::std::mem::transmute(
                    ::std::raw::Slice { data : self.scratch_space.as_slice().unsafe_get(idx * SCRATCH_SIZE),
                                      len : SCRATCH_SIZE }))
        }
    }
}


macro_rules! pass_by_object(
    ( $testcase:ident, $reuse:ident, $iters:expr ) => ({
            let mut rng = common::FastRand::new();
            for _ in range(0, $iters) {
                let mut message_req = $reuse.new_builder(0);
                let mut message_res = $reuse.new_builder(1);

                let request = message_req.init_root::<$testcase::RequestBuilder>();
                let response = message_res.init_root::<$testcase::ResponseBuilder>();
                let expected = $testcase::setup_request(&mut rng, request);

                let request_reader = request.as_reader();
                $testcase::handle_request(request_reader, response);

                let response_reader = response.as_reader();
                if !$testcase::check_response(response_reader, expected) {
                    panic!("Incorrect response.");
                }
            }
        });
    )


macro_rules! pass_by_bytes(
    ( $testcase:ident, $reuse:ident, $compression:ident, $iters:expr ) => ({
            let mut request_bytes = ::std::vec::Vec::from_elem(SCRATCH_SIZE * 8, 0u8);
            let mut response_bytes = ::std::vec::Vec::from_elem(SCRATCH_SIZE * 8, 0u8);
            let mut rng = common::FastRand::new();
            for _ in range(0, $iters) {
                let mut message_req = $reuse.new_builder(0);
                let mut message_res = $reuse.new_builder(1);

                let expected = {
                    let request = message_req.init_root::<$testcase::RequestBuilder>();
                    $testcase::setup_request(&mut rng, request)
                };

                {
                    let response = message_res.init_root::<$testcase::ResponseBuilder>();

                    {
                        let mut writer = capnp::io::ArrayOutputStream::new(request_bytes.as_mut_slice());
                        $compression::write_buffered(&mut writer, &message_req)
                    }

                    let message_reader = $compression::new_buffered_reader(
                        &mut capnp::io::ArrayInputStream::new(request_bytes.as_slice()),
                        capnp::message::DEFAULT_READER_OPTIONS);

                    let request_reader : $testcase::RequestReader = message_reader.get_root();
                    $testcase::handle_request(request_reader, response);
                }

                {
                    let mut writer = capnp::io::ArrayOutputStream::new(response_bytes.as_mut_slice());
                    $compression::write_buffered(&mut writer, &message_res)
                }

                let message_reader = $compression::new_buffered_reader(
                    &mut capnp::io::ArrayInputStream::new(response_bytes.as_slice()),
                    capnp::message::DEFAULT_READER_OPTIONS);

                let response_reader : $testcase::ResponseReader = message_reader.get_root();
                if !$testcase::check_response(response_reader, expected) {
                    panic!("Incorrect response.");
                }
            }
        });
    )

macro_rules! server(
    ( $testcase:ident, $reuse:ident, $compression:ident, $iters:expr, $input:expr, $output:expr) => ({
            let mut out_buffered = capnp::io::BufferedOutputStreamWrapper::new(&mut $output);
            let mut in_buffered = capnp::io::BufferedInputStreamWrapper::new(&mut $input);
            for _ in range(0, $iters) {
                let mut message_res = $reuse.new_builder(0);

                {
                    let response = message_res.init_root::<$testcase::ResponseBuilder>();
                    let message_reader = $compression::new_buffered_reader(
                        &mut in_buffered,
                        capnp::message::DEFAULT_READER_OPTIONS);
                    let request_reader : $testcase::RequestReader = message_reader.get_root();
                    $testcase::handle_request(request_reader, response);
                }

                $compression::write_buffered(&mut out_buffered, &message_res);
                out_buffered.flush().unwrap();
            }
        });
    )

macro_rules! sync_client(
    ( $testcase:ident, $reuse:ident, $compression:ident, $iters:expr) => ({
            let mut out_stream = ::std::io::stdio::stdout_raw();
            let mut in_stream = ::std::io::stdio::stdin_raw();
            let mut in_buffered = capnp::io::BufferedInputStreamWrapper::new(&mut in_stream);
            let mut rng = common::FastRand::new();
            for _ in range(0, $iters) {
                let mut message_req = $reuse.new_builder(0);

                let expected = {
                    let request = message_req.init_root::<$testcase::RequestBuilder>();
                    $testcase::setup_request(&mut rng, request)
                };
                $compression::write(&mut out_stream, &message_req);

                let message_reader = $compression::new_buffered_reader(
                    &mut in_buffered,
                    capnp::message::DEFAULT_READER_OPTIONS);
                let response_reader : $testcase::ResponseReader = message_reader.get_root();
                assert!($testcase::check_response(response_reader, expected));

            }
        });
    )


macro_rules! pass_by_pipe(
    ( $testcase:ident, $reuse:ident, $compression:ident, $iters:expr) => ({
        use std::io::process;

        let mut args = ::std::os::args();
        args[2] = "client".to_string();

        let mut command = process::Command::new(args[0].as_slice());
        command.args(args.slice(1, args.len()));
        command.stdin(process::CreatePipe(true, false));
        command.stdout(process::CreatePipe(false, true));
        command.stderr(process::Ignored);
        match command.spawn() {
            Ok(ref mut p) => {
                let mut child_std_out = p.stdout.take().unwrap();
                let mut child_std_in = p.stdin.take().unwrap();

                server!($testcase, $reuse, $compression, $iters, child_std_out, child_std_in);
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
            match $mode.as_slice() {
                "object" => pass_by_object!($testcase, $reuse, $iters),
                "bytes" => pass_by_bytes!($testcase, $reuse, $compression, $iters),
                "client" => sync_client!($testcase, $reuse, $compression, $iters),
                "server" => {
                    let mut input = ::std::io::stdio::stdin_raw();
                    let mut output = ::std::io::stdio::stdout_raw();
                    server!($testcase, $reuse, $compression, $iters, input, output)
                }
                "pipe" => pass_by_pipe!($testcase, $reuse, $compression, $iters),
                s => panic!("unrecognized mode: {}", s)
            }
        });
    )

macro_rules! do_testcase1(
    ( $testcase:expr, $mode:expr, $reuse:ident, $compression:ident, $iters:expr) => ({
            match $testcase.as_slice() {
                "carsales" => do_testcase!(carsales, $mode, $reuse, $compression, $iters),
                "catrank" => do_testcase!(catrank, $mode, $reuse, $compression, $iters),
                "eval" => do_testcase!(eval, $mode, $reuse, $compression, $iters),
                s => panic!("unrecognized test case: {}", s)
            }
        });
    )

macro_rules! do_testcase2(
    ( $testcase:expr, $mode:expr, $reuse:expr, $compression:ident, $iters:expr) => ({
            match $reuse.as_slice() {
                "no-reuse" => {
                    let mut scratch = NoScratch;
                    do_testcase1!($testcase, $mode, scratch, $compression, $iters)
                }
                "reuse" => {
                    let mut scratch = UseScratch::new();
                    do_testcase1!($testcase, $mode, scratch, $compression, $iters)
                }
                s => panic!("unrecognized reuse option: {}", s)
            }
        });
    )

pub fn main() {
    let args = ::std::os::args();

    if args.len() != 6 {
        println!("USAGE: {} CASE MODE REUSE COMPRESSION ITERATION_COUNT", args[0]);
        ::std::os::set_exit_status(1);
        return;
    }

    let iters = match from_str::<u64>(args[5].as_slice()) {
        Some (n) => n,
        None => {
            println!("Could not parse a u64 from: {}", args[5]);
            ::std::os::set_exit_status(1);
            return;
        }
    };

    match args[4].as_slice() {
        "none" => do_testcase2!(args[1], args[2],  args[3], uncompressed, iters),
        "packed" => do_testcase2!(args[1], args[2], args[3], packed, iters),
        s => panic!("unrecognized compression: {}", s)
    }
}
