/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use any::{AnyPointer};
use common::{MessageSize};
use layout::{FromStructBuilder, HasStructSize};
use message::{MessageBuilder, MallocMessageBuilder};

pub trait RequestHook {
    fn message<'a>(&'a mut self) -> &'a mut MallocMessageBuilder;
    fn send(&self);
}

pub struct Request<Params, Results> {
    priv hook : ~RequestHook
}

impl <Params, Results> Request <Params, Results> {
    pub fn new(hook : ~RequestHook) -> Request <Params, Results> {
        Request { hook : hook }
    }
}

impl <'a, Params : FromStructBuilder<'a> + HasStructSize, Results> Request<Params, Results> {
    pub fn init_params(&'a mut self) -> Params {
        self.hook.message().init_root()
    }
}

pub trait ClientHook {
    fn new_call(&self,
                interface_id : u64,
                method_id : u16,
                size_hint : Option<MessageSize>)
                -> Request<AnyPointer::Builder, AnyPointer::Reader>;
}


pub struct Client {
    hook : ~ClientHook
}

impl Client {
    pub fn new(hook : ~ClientHook) -> Client {
        Client { hook : hook }
    }

    pub fn new_call<Params, Results>(&self,
                                     interface_id : u64,
                                     method_id : u16,
                                     size_hint : Option<MessageSize>)
                                     -> Request<Params, Results> {
        let typeless = self.hook.new_call(interface_id, method_id, size_hint);
        Request { hook : typeless.hook }
    }
}

