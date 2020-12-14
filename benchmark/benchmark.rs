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

#![allow(incomplete_features)]
#![feature(generic_associated_types)]

use std::{io};

use capnp::{message, serialize, serialize_packed};
use capnp::private::arena::{BuilderArena, ReaderArena};
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
    type Request: Owned;
    type Response: Owned;
    type Expectation;

    fn setup_request<'a, A>(&self, rnd: &mut crate::common::FastRand, b: <Self::Request as Owned>::Builder<'a, A>)
                            -> Self::Expectation where A: BuilderArena;

    fn handle_request<'a, 'b, A, B>(&self,
                                    r: <Self::Request as Owned>::Reader<'a, A>,
                                    b: <Self::Response as Owned>::Builder<'b, B>)
                                    -> ::capnp::Result<()> where A: ReaderArena, B: BuilderArena;

    fn check_response<'a, A>(&self, r: <Self::Response as Owned>::Reader<'a, A>, e: Self::Expectation)
                             -> ::capnp::Result<()> where A: ReaderArena;
}

trait Serialize {
    fn read_message<R>(
        &self,
        read: &mut R,
        options: message::ReaderOptions)
                       -> ::capnp::Result<message::Reader<::capnp::serialize::OwnedSegments>>
        where R: io::BufRead;

    fn write_message<W, A>(&self, write: &mut W, message: &message::Builder<A>) -> ::capnp::Result<()>
        where W: io::Write, A: message::Allocator;
}

struct NoCompression;

impl Serialize for NoCompression {
    fn read_message<R>(&self, read: &mut R,
                       options: message::ReaderOptions)
                       -> ::capnp::Result<message::Reader<::capnp::serialize::OwnedSegments>>
        where R: io::BufRead
    {
        serialize::read_message(read, options)
    }

    fn write_message<W, A>(&self, write: &mut W, message: &message::Builder<A>) -> ::capnp::Result<()>
        where W: io::Write, A: message::Allocator {
        serialize::write_message(write, message).map_err(|e| e.into())
    }
}

struct Packed;

impl Serialize for Packed {
    fn read_message<R>(&self, read: &mut R,
                       options: message::ReaderOptions)
                       -> ::capnp::Result<message::Reader<::capnp::serialize::OwnedSegments>>
        where R: io::BufRead
    {
        serialize_packed::read_message(read, options)
    }

    fn write_message<W, A>(&self, write: &mut W, message: &message::Builder<A>) -> ::capnp::Result<()>
        where W: io::Write, A: message::Allocator {
        serialize_packed::write_message(write, message).map_err(|e| e.into())
    }
}

trait Scratch<'a> {
    type Allocator: message::Allocator;

    fn get_allocators(&'a mut self) -> (Self::Allocator, Self::Allocator);
}

const SCRATCH_SIZE: usize = 128 * 1024;

#[derive(Clone, Copy)]
pub struct NoScratch;

impl <'a> Scratch<'a> for NoScratch {
    type Allocator = message::HeapAllocator;

    fn get_allocators(&'a mut self) -> (Self::Allocator, Self::Allocator) {
        (message::HeapAllocator::new(), message::HeapAllocator::new())
    }
}

pub struct UseScratch {
    buffer1: Vec<capnp::Word>,
    buffer2: Vec<capnp::Word>,
}

impl UseScratch {
    pub fn new() -> UseScratch {
        UseScratch {
            buffer1: capnp::Word::allocate_zeroed_vec(SCRATCH_SIZE),
            buffer2: capnp::Word::allocate_zeroed_vec(SCRATCH_SIZE),
        }
    }
}

impl <'a> Scratch<'a> for UseScratch {
    type Allocator = message::ScratchSpaceHeapAllocator<'a>;

    fn get_allocators(&'a mut self) -> (Self::Allocator, Self::Allocator) {
        let UseScratch {ref mut buffer1, ref mut buffer2 } = self;
        (message::ScratchSpaceHeapAllocator::new(capnp::Word::words_to_bytes_mut(buffer1)),
         message::ScratchSpaceHeapAllocator::new(capnp::Word::words_to_bytes_mut(buffer2)))
    }
}

fn pass_by_object<S, T>(testcase: T, mut reuse: S, iters: u64) -> ::capnp::Result<()>
    where S: for<'a> Scratch<'a>, T: TestCase,
{
    let mut rng = common::FastRand::new();
    let (mut allocator_req, mut allocator_res) = reuse.get_allocators();
    for _ in 0..iters {
        let mut message_req = message::Builder::new(&mut allocator_req);
        let mut message_res = message::Builder::new(&mut allocator_res);

        let expected = testcase.setup_request(
            &mut rng,
            message_req.init_root());

        testcase.handle_request(
            message_req.get_root_as_reader()?,
            message_res.init_root())?;

        testcase.check_response(
            message_res.get_root_as_reader()?,
            expected)?;
    }
    Ok(())
}

fn pass_by_bytes<C, S, T>(testcase: T, mut reuse: S, compression: C, iters: u64) -> ::capnp::Result<()>
    where C: Serialize, S: for<'a> Scratch<'a>, T: TestCase,
{
    let mut request_bytes = vec![0u8; SCRATCH_SIZE * 8];
    let mut response_bytes = vec![0u8; SCRATCH_SIZE * 8];
    let mut rng = common::FastRand::new();
    let (mut allocator_req, mut allocator_res) = reuse.get_allocators();
    for _ in 0..iters {
        let mut message_req = message::Builder::new(&mut allocator_req);
        let mut message_res = message::Builder::new(&mut allocator_res);

        let expected = {
            let request = message_req.init_root();
            testcase.setup_request(&mut rng, request)
        };

        {
            let response = message_res.init_root();

            {
                let mut writer: &mut [u8] = &mut request_bytes;
                compression.write_message(&mut writer, &mut message_req)?;
            }

            let mut request_bytes1: &[u8] = &request_bytes;
            let message_reader = compression.read_message(
                &mut request_bytes1,
                Default::default())?;

            let request_reader = message_reader.get_root()?;
            testcase.handle_request(request_reader, response)?;
        }

        {
            let mut writer: &mut [u8] = &mut response_bytes;
            compression.write_message(&mut writer, &mut message_res)?;
        }

        let mut response_bytes1: &[u8] = &response_bytes;
        let message_reader = compression.read_message(
            &mut response_bytes1,
            Default::default())?;

        let response_reader = message_reader.get_root()?;
        testcase.check_response(response_reader, expected)?;
    }
    Ok(())
}

fn server<C, S, T, R, W>(testcase: T, mut reuse: S, compression: C, iters: u64, mut input: R, mut output: W)
                         -> ::capnp::Result<()>
    where C: Serialize, S: for<'a> Scratch<'a>, T: TestCase, R: io::Read, W: io::Write,
{
    let mut out_buffered = io::BufWriter::new(&mut output);
    let mut in_buffered = io::BufReader::new(&mut input);
    let (mut allocator_res, _) = reuse.get_allocators();
    for _ in 0..iters {
        use std::io::Write;
        let mut message_res = message::Builder::new(&mut allocator_res);

        {
            let response = message_res.init_root();
            let message_reader = compression.read_message(
                &mut in_buffered,
                capnp::message::DEFAULT_READER_OPTIONS)?;
            let request_reader = message_reader.get_root()?;
            testcase.handle_request(request_reader, response)?;
        }

        compression.write_message(&mut out_buffered, &mut message_res)?;
        out_buffered.flush()?;
    }
    Ok(())
}

fn sync_client<C, S, T>(testcase: T, mut reuse: S, compression: C, iters: u64)
                        -> ::capnp::Result<()>
    where C: Serialize, S: for<'a> Scratch<'a>, T: TestCase,
{
    let mut out_stream: ::std::fs::File = unsafe { ::std::os::unix::io::FromRawFd::from_raw_fd(1) };
    let mut in_stream: ::std::fs::File = unsafe { ::std::os::unix::io::FromRawFd::from_raw_fd(0) };
    let mut in_buffered = io::BufReader::new(&mut in_stream);
    let mut out_buffered = io::BufWriter::new(&mut out_stream);
    let mut rng = common::FastRand::new();
    let (mut allocator_req, _) = reuse.get_allocators();
    for _ in 0..iters {
        use std::io::Write;
        let mut message_req = message::Builder::new(&mut allocator_req);

        let expected = {
            let request = message_req.init_root();
            testcase.setup_request(&mut rng, request)
        };
        compression.write_message(&mut out_buffered, &mut message_req)?;
        out_buffered.flush()?;

        let message_reader = compression.read_message(
            &mut in_buffered,
            Default::default())?;
        let response_reader = message_reader.get_root()?;
        testcase.check_response(response_reader, expected)?;
    }
    Ok(())
}

fn pass_by_pipe<C, S, T>(testcase: T, reuse: S, compression: C, iters: u64) -> ::capnp::Result<()>
    where C: Serialize, S: for<'a> Scratch<'a>, T: TestCase,
{
    use std::{process, env};

    let mut args: Vec<String> = env::args().collect();
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
            server(testcase, reuse, compression, iters, child_std_out, child_std_in)?;
            println!("{}", p.wait().unwrap());
            Ok(())
        }
        Err(e) => {
            println!("could not start process: {}", e);
            Ok(())
        }
    }
}

pub enum Mode {
    Object,
    Bytes,
    Client,
    Server,
    Pipe,
}

impl Mode {
    pub fn parse(s: &str) -> ::capnp::Result<Mode> {
        match s {
            "object" => Ok(Mode::Object),
            "bytes" => Ok(Mode::Bytes),
            "client" => Ok(Mode::Client),
            "server" => Ok(Mode::Server),
            "pipe" => Ok(Mode::Pipe),
            s => Err(::capnp::Error::failed(format!("unrecognized mode: {}", s))),
        }
    }
}

fn do_testcase<C, S, T>(testcase: T, mode: Mode, reuse: S, compression: C, iters: u64) -> ::capnp::Result<()>
    where C: Serialize, S: for<'a> Scratch<'a>, T: TestCase,
{
    match mode {
        Mode::Object => pass_by_object(testcase, reuse, iters),
        Mode::Bytes => pass_by_bytes(testcase, reuse, compression, iters),
        Mode::Client => sync_client(testcase, reuse, compression, iters),
        Mode::Server => {
            let input: ::std::fs::File = unsafe { ::std::os::unix::io::FromRawFd::from_raw_fd(1) };
            let output: ::std::fs::File = unsafe { ::std::os::unix::io::FromRawFd::from_raw_fd(0) };
            server(testcase, reuse, compression, iters, input, output)
        }
        Mode::Pipe => pass_by_pipe(testcase, reuse, compression, iters),
    }
}

fn do_testcase1<C, S>(case: &str, mode: Mode, scratch: S, compression: C, iters: u64) -> ::capnp::Result<()>
    where C: Serialize, S: for<'a> Scratch<'a>,
{
    match case {
        "carsales" => do_testcase(carsales::CarSales, mode, scratch, compression, iters),
        "catrank" => do_testcase(catrank::CatRank, mode, scratch, compression, iters),
        "eval" => do_testcase(eval::Eval, mode, scratch, compression, iters),
        s => Err(::capnp::Error::failed(format!("unrecognized test case: {}", s))),
    }
}

fn do_testcase2<C>(case: &str, mode: Mode, scratch: &str, compression: C, iters: u64) -> ::capnp::Result<()>
    where C: Serialize,
{
    match scratch {
        "no-reuse" => do_testcase1(case, mode, NoScratch, compression, iters),
        "reuse" => do_testcase1(case, mode, UseScratch::new(), compression, iters),
        s => Err(::capnp::Error::failed(format!("unrecognized reuse option: {}", s))),
    }
}

fn try_main() -> ::capnp::Result<()> {
    let args: Vec<String> = ::std::env::args().collect();

    assert!(args.len() == 6,
            "USAGE: {} CASE MODE REUSE COMPRESSION ITERATION_COUNT",
            args[0]);

    let iters = match args[5].parse::<u64>() {
        Ok(n) => n,
        Err(_) =>
            return Err(::capnp::Error::failed(format!("Could not parse a u64 from: {}", args[5]))),
    };

    let mode = Mode::parse(&*args[2])?;

    match &*args[4] {
        "none" => do_testcase2(&*args[1], mode, &*args[3], NoCompression, iters),
        "packed" => do_testcase2(&*args[1], mode, &*args[3], Packed, iters),
        s => Err(::capnp::Error::failed(format!("unrecognized compression: {}", s))),
    }
}

pub fn main() {
    match try_main() {
        Ok(()) => (),
        Err(e) => {
            panic!("error: {:?}", e);
        }
    }
}
