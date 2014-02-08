/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

// Things from capability.c++

use std;

use capnp::any::{AnyPointer};
use capnp::common::{MessageSize};
use capnp::capability::{ClientHook, Request, Server};

pub struct LocalClient {
    server : ~Server,
}

impl ClientHook for LocalClient {
    fn copy(&self) -> ~ClientHook {
        fail!()
    }
    fn new_call(&self,
                interface_id : u64,
                method_id : u16,
                size_hint : Option<MessageSize>)
                -> Request<AnyPointer::Builder, AnyPointer::Reader, AnyPointer::Pipeline> {
        fail!()
    }

    // HACK
    fn get_descriptor(&self) -> ~std::any::Any {
        fail!()
    }

}
