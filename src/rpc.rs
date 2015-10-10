// Copyright (c) 2013-2015 Sandstorm Development Group, Inc. and contributors
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

use capnp::{any_pointer};
use capnp::capability;
use capnp::capability::{RemotePromise, Request};
use capnp::private::capability::{ClientHook, ParamsHook, PipelineHook, PipelineOp,
                                 RequestHook, ResponseHook, ResultsHook};
use capnp_gj::serialize;

use std::vec::Vec;
use std::collections::hash_map::HashMap;
use std::collections::binary_heap::BinaryHeap;
use std::cell::RefCell;
use std::rc::Rc;

use rpc_capnp::{message, return_, cap_descriptor, message_target, payload, promised_answer};

pub type QuestionId = u32;
pub type AnswerId = QuestionId;
pub type ExportId = u32;
pub type ImportId = ExportId;


pub struct ImportTable<T> {
    slots : HashMap<u32, T>,
}

impl <T> ImportTable<T> {
    pub fn new() -> ImportTable<T> {
        ImportTable { slots : HashMap::new() }
    }
}

#[derive(PartialEq, Eq)]
struct ReverseU32 { val : u32 }

impl ::std::cmp::Ord for ReverseU32 {
    fn cmp(&self, other : &ReverseU32) -> ::std::cmp::Ordering {
        if self.val > other.val { ::std::cmp::Ordering::Less }
        else if self.val < other.val { ::std::cmp::Ordering::Greater }
        else { ::std::cmp::Ordering::Equal }
    }
}

impl ::std::cmp::PartialOrd for ReverseU32 {
    fn partial_cmp(&self, other : &ReverseU32) -> Option<::std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}


pub struct ExportTable<T> {
    slots : Vec<Option<T>>,

    // prioritize lower values
    free_ids : BinaryHeap<ReverseU32>,
}

impl <T> ExportTable<T> {
    pub fn new() -> ExportTable<T> {
        ExportTable { slots : Vec::new(),
                      free_ids : BinaryHeap::new() }
    }

    pub fn erase(&mut self, id : u32) {
        self.slots[id as usize] = None;
        self.free_ids.push(ReverseU32 { val : id } );
    }

    pub fn push(&mut self, val : T) -> u32 {
        match self.free_ids.pop() {
            Some(ReverseU32 { val : id }) => {
                self.slots[id as usize] = Some(val);
                id
            }
            None => {
                self.slots.push(Some(val));
                self.slots.len() as u32 - 1
            }
        }
    }
}

pub struct Question {
    is_awaiting_return: bool,
}

pub struct Answer {
    active: bool,
}

pub struct Export {
    ref_count: usize
}

pub struct Import {
    import_client: (),
}


// How should this work? I make a two party rpc system by passing in an AsyncStream.
pub struct TwoPartyVatNetwork<T> where T: ::gj::io::AsyncWrite + ::gj::io::AsyncRead {
    stream: T,
    receive_options: ::capnp::message::ReaderOptions,
}

/*
pub struct RpcSystem {

}

impl RpcSystem {
    pub fn bootstrap() -> Client {

    }
}
*/

pub struct ConnectionErrorHandler {
    state: Rc<RefCell<ConnectionState>>,
}

impl ::gj::TaskReaper<(), ::capnp::Error> for ConnectionErrorHandler {
    fn task_failed(&mut self, error: ::capnp::Error) {
        // TODO: disconnect
    }
}

pub struct ConnectionState {
    exports: ExportTable<Export>,
    questions: ExportTable<Question>,
    answers: ImportTable<Answer>,
    imports: ImportTable<Import>,
    tasks: ::gj::TaskSet<(), ::capnp::Error>,
}

impl ConnectionState {
    pub fn new() -> Rc<RefCell<ConnectionState>> {
        unimplemented!()
    }
}
