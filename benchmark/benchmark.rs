// Copyright (c) 2013-2017 Sandstorm Development Group, Inc. and contributors
// Licensed under the MIT License:
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

extern crate capnp;
extern crate fdstream;
extern crate rand;

use capnp::{message, serialize, serialize_packed};
use capnp::traits::Owned;

pub mod common;

pub mod carsales_capnp {
  include!(concat!(env!("OUT_DIR"), "/carsales_capnp.rs"));
}
pub mod carsales;

pub mod catrank_capnp {
  include!(concat!(env!("OUT_DIR"), "/catrank_capnp.rs"));
}
pub mod catrank;

pub mod eval_capnp {
  include!(concat!(env!("OUT_DIR"), "/eval_capnp.rs"));
}

pub mod eval;

trait TestCase {
    type Request: for<'a> Owned<'a>;
    type Response: for<'a> Owned<'a>;
    type Expectation;

    fn setup_request(&self, &mut common::FastRand, <Self::Request as Owned>::Builder) -> Self::Expectation;
    fn handle_request(&self, <Self::Request as Owned>::Reader, <Self::Response as Owned>::Builder);
    fn check_response(&self, <Self::Response as Owned>::Reader, Self::Expectation) -> bool;

    // HACK. The Builder::as_reader() method is not attached to Owned. Maybe it should be?
    fn request_as_reader<'a>(&self, <Self::Request as Owned<'a>>::Builder)
                             -> <Self::Request as Owned<'a>>::Reader;
    fn response_as_reader<'a>(&self, <Self::Response as Owned<'a>>::Builder)
                              -> <Self::Response as Owned<'a>>::Reader;
}

trait Serialize {
    fn read_message<R>(
        &self,
        read: &mut R,
        options: message::ReaderOptions)
                       -> ::capnp::Result<message::Reader<::capnp::serialize::OwnedSegments>>
        where R: ::std::io::BufRead;

    fn write_message<W, A>(&self, write: &mut W, message: &message::Builder<A>) -> ::capnp::Result<()>
        where W: ::std::io::Write, A: message::Allocator;
}

struct NoCompression;

impl Serialize for NoCompression {
    fn read_message<R>(&self, read: &mut R,
                       options: message::ReaderOptions)
                       -> ::capnp::Result<message::Reader<::capnp::serialize::OwnedSegments>>
        where R: ::std::io::BufRead
    {
        serialize::read_message(read, options)
    }

    fn write_message<W, A>(&self, write: &mut W, message: &message::Builder<A>) -> ::capnp::Result<()>
        where W: ::std::io::Write, A: message::Allocator {
        serialize::write_message(write, message).map_err(|e| e.into())
    }
}

struct Packed;

impl Serialize for Packed {
    fn read_message<R>(&self, read: &mut R,
                       options: message::ReaderOptions)
                       -> ::capnp::Result<message::Reader<::capnp::serialize::OwnedSegments>>
        where R: ::std::io::BufRead
    {
        serialize_packed::read_message(read, options)
    }

    fn write_message<W, A>(&self, write: &mut W, message: &message::Builder<A>) -> ::capnp::Result<()>
        where W: ::std::io::Write, A: message::Allocator {
        serialize_packed::write_message(write, message).map_err(|e| e.into())
    }
}

trait Scratch<'a> {
    type Allocator: message::Allocator;

    fn get_builders(&'a mut self) -> (message::Builder<Self::Allocator>, message::Builder<Self::Allocator>);
}

const SCRATCH_SIZE: usize = 128 * 1024;

#[derive(Clone, Copy)]
pub struct NoScratch;

impl <'a> Scratch<'a> for NoScratch {
    type Allocator = message::HeapAllocator;

    fn get_builders(&'a mut self) -> (message::Builder<Self::Allocator>, message::Builder<Self::Allocator>) {
        (capnp::message::Builder::new_default(), capnp::message::Builder::new_default())
    }
}

pub struct UseScratch {
    _owned_space: ::std::vec::Vec<::std::vec::Vec<capnp::Word>>,
    scratch_space: ::std::vec::Vec<::capnp::message::ScratchSpace<'static>>,
}

impl UseScratch {
    pub fn new() -> UseScratch {
        let mut owned = Vec::new();
        let mut scratch = Vec::new();
        for _ in 0..6 {
            let mut words = ::capnp::Word::allocate_zeroed_vec(SCRATCH_SIZE);
            scratch.push(::capnp::message::ScratchSpace::new(
                unsafe {::std::mem::transmute(&mut words[..])}));
            owned.push(words);
        }
        UseScratch {
            _owned_space: owned,
            scratch_space: scratch,
        }
    }
}

impl <'a> Scratch<'a> for UseScratch {
    type Allocator = capnp::message::ScratchSpaceHeapAllocator<'a, 'a>;

    fn get_builders(&'a mut self) -> (message::Builder<Self::Allocator>, message::Builder<Self::Allocator>) {
        (capnp::message::Builder::new(::capnp::message::ScratchSpaceHeapAllocator::new(
            unsafe{::std::mem::transmute(&mut self.scratch_space[0])})),
         capnp::message::Builder::new(::capnp::message::ScratchSpaceHeapAllocator::new(
             unsafe{::std::mem::transmute(&mut self.scratch_space[1])})))

    }
}

fn pass_by_object<S, T>(testcase: T, mut reuse: S, iters: u64)
    where S: for<'a> Scratch<'a>, T: TestCase,
{
    let mut rng = common::FastRand::new();
    for _ in 0..iters {
        let (mut message_req, mut message_res) = reuse.get_builders();

        let expected = testcase.setup_request(
            &mut rng,
            message_req.init_root());

        testcase.handle_request(
            testcase.request_as_reader(message_req.get_root().unwrap()),
            message_res.init_root());

        if !testcase.check_response(
            testcase.response_as_reader(message_res.get_root().unwrap()),
            expected) {
            panic!("Incorrect response.");
        }
    }
}

fn pass_by_bytes<C, S, T>(testcase: T, mut reuse: S, compression: C, iters: u64)
    where C: Serialize, S: for<'a> Scratch<'a>, T: TestCase,
{
    let mut request_bytes: ::std::vec::Vec<u8> =
        ::std::iter::repeat(0u8).take(SCRATCH_SIZE * 8).collect();
    let mut response_bytes: ::std::vec::Vec<u8> =
        ::std::iter::repeat(0u8).take(SCRATCH_SIZE * 8).collect();
    let mut rng = common::FastRand::new();
    for _ in 0..iters {
        let (mut message_req, mut message_res) = reuse.get_builders();

        let expected = {
            let request = message_req.init_root();
            testcase.setup_request(&mut rng, request)
        };

        {
            let response = message_res.init_root();

            {
                let mut writer: &mut [u8] = &mut request_bytes;
                compression.write_message(&mut writer, &mut message_req).unwrap()
            }

            let mut request_bytes1: &[u8] = &request_bytes;
            let message_reader = compression.read_message(
                &mut request_bytes1,
                capnp::message::DEFAULT_READER_OPTIONS).unwrap();

            let request_reader = message_reader.get_root().unwrap();
            testcase.handle_request(request_reader, response);
        }

        {
            let mut writer: &mut [u8] = &mut response_bytes;
            compression.write_message(&mut writer, &mut message_res).unwrap()
        }

        let mut response_bytes1: &[u8] = &response_bytes;
        let message_reader = compression.read_message(
            &mut response_bytes1,
            capnp::message::DEFAULT_READER_OPTIONS).unwrap();

        let response_reader = message_reader.get_root().unwrap();
        if !testcase.check_response(response_reader, expected) {
            panic!("Incorrect response.");
        }
    }
}

fn server<C, S, T, R, W>(testcase: T, mut reuse: S, compression: C, iters: u64, mut input: R, mut output: W)
    where C: Serialize, S: for<'a> Scratch<'a>, T: TestCase, R: ::std::io::Read, W: ::std::io::Write,
{
    let mut out_buffered = ::std::io::BufWriter::new(&mut output);
    let mut in_buffered = ::std::io::BufReader::new(&mut input);
    for _ in 0..iters {
        use std::io::Write;
        let (mut message_res, _) = reuse.get_builders();

        {
            let response = message_res.init_root();
            let message_reader = compression.read_message(
                &mut in_buffered,
                capnp::message::DEFAULT_READER_OPTIONS).unwrap();
            let request_reader = message_reader.get_root().unwrap();
            testcase.handle_request(request_reader, response);
        }

        compression.write_message(&mut out_buffered, &mut message_res).unwrap();
        out_buffered.flush().unwrap();
    }
}

fn sync_client<C, S, T>(testcase: T, mut reuse: S, compression: C, iters: u64)
    where C: Serialize, S: for<'a> Scratch<'a>, T: TestCase,
{
    let mut out_stream = ::fdstream::FdStream::new(1);
    let mut in_stream = ::fdstream::FdStream::new(0);
    let mut in_buffered = ::std::io::BufReader::new(&mut in_stream);
    let mut out_buffered = ::std::io::BufWriter::new(&mut out_stream);
    let mut rng = common::FastRand::new();
    for _ in 0..iters {
        use std::io::Write;
        let (mut message_req, _) = reuse.get_builders();

        let expected = {
            let request = message_req.init_root();
            testcase.setup_request(&mut rng, request)
        };
        compression.write_message(&mut out_buffered, &mut message_req).unwrap();
        out_buffered.flush().unwrap();

        let message_reader = compression.read_message(
            &mut in_buffered,
            capnp::message::DEFAULT_READER_OPTIONS).unwrap();
        let response_reader = message_reader.get_root().unwrap();
        assert!(testcase.check_response(response_reader, expected));
    }
}

fn pass_by_pipe<C, S, T>(testcase: T, reuse: S, compression: C, iters: u64)
    where C: Serialize, S: for<'a> Scratch<'a>, T: TestCase,
{
    use std::process;

    let mut args: Vec<String> = ::std::env::args().collect();
    args[2] = "client".to_string();

    let mut command = process::Command::new(&args[0]);
    command.args(&args[1..args.len()]);
    command.stdin(process::Stdio::piped());
    command.stdout(process::Stdio::piped());
    command.stderr(process::Stdio::null());
    match command.spawn() {
        Ok(ref mut p) => {
            let child_std_out = p.stdout.take().unwrap();
            let child_std_in = p.stdin.take().unwrap();
            server(testcase, reuse, compression, iters, child_std_out, child_std_in);
            println!("{}", p.wait().unwrap());
        }
        Err(e) => {
            println!("could not start process: {}", e);
        }
    }
}

fn do_testcase<C, S, T>(testcase: T, mode: &str, reuse: S, compression: C, iters: u64)
    where C: Serialize, S: for<'a> Scratch<'a>, T: TestCase,
{
    match mode {
        "object" => pass_by_object(testcase, reuse, iters),
        "bytes" => pass_by_bytes(testcase, reuse, compression, iters),
        "client" => sync_client(testcase, reuse, compression, iters),
        "server" => {
            let input = ::fdstream::FdStream::new(0);
            let output = ::fdstream::FdStream::new(1);
            server(testcase, reuse, compression, iters, input, output)
        }
        "pipe" => pass_by_pipe(testcase, reuse, compression, iters),
        s => panic!("unrecognized mode: {}", s)
    }
}

fn do_testcase1<C, S>(case: &str, mode: &str, scratch: S, compression: C, iters: u64)
    where C: Serialize, S: for<'a> Scratch<'a>,
{
    match case {
        "carsales" => do_testcase(carsales::CarSales, mode, scratch, compression, iters),
        "catrank" => do_testcase(catrank::CatRank, mode, scratch, compression, iters),
        "eval" => do_testcase(eval::Eval, mode, scratch, compression, iters),
        s => panic!("unrecognized test case: {}", s)
    }
}

fn do_testcase2<C>(case: &str, mode: &str, scratch: &str, compression: C, iters: u64)
    where C: Serialize,
{
    match scratch {
        "no-reuse" => do_testcase1(case, mode, NoScratch, compression, iters),
        "reuse" => do_testcase1(case, mode, UseScratch::new(), compression, iters),
        s => panic!("unrecognized reuse option: {}", s),
    };

}

pub fn main() {
    let args: Vec<String> = ::std::env::args().collect();

    assert!(args.len() == 6,
            "USAGE: {} CASE MODE REUSE COMPRESSION ITERATION_COUNT",
            args[0]);

    let iters = match args[5].parse::<u64>() {
        Ok(n) => n,
        Err(_) => {
            panic!("Could not parse a u64 from: {}", args[5]);
        }
    };

    match &*args[4] {
        "none" => do_testcase2(&*args[1], &*args[2], &*args[3], NoCompression, iters),
        "packed" => do_testcase2(&*args[1], &*args[2], &*args[3], Packed, iters),
        s => panic!("unrecognized compression: {}", s)
    };
}
