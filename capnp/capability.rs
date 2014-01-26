/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use any::{AnyPointer};
use common::{MessageSize};

pub trait RequestHook {
    fn send(&self);
}

pub struct Request<Params, Results> {
    params : Params,
    hook : ~RequestHook
}

pub trait ClientHook {
    fn new_call(interface_id : u64,
                method_id : u16,
                size_hint : Option<MessageSize>)
                -> Request<AnyPointer::Builder, AnyPointer::Reader>;
}
