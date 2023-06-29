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

#![cfg(feature = "alloc")]
use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::any_pointer;
use crate::capability::{Params, Promise, RemotePromise, Request, Results};
use crate::MessageSize;

pub trait ResponseHook {
    fn get(&self) -> crate::Result<any_pointer::Reader<'_>>;
}

pub trait RequestHook {
    fn get(&mut self) -> any_pointer::Builder<'_>;
    fn get_brand(&self) -> usize;
    fn send(self: Box<Self>) -> RemotePromise<any_pointer::Owned>;
    fn tail_send(
        self: Box<Self>,
    ) -> Option<(
        u32,
        crate::capability::Promise<(), crate::Error>,
        Box<dyn PipelineHook>,
    )>;
}

pub trait ClientHook {
    fn add_ref(&self) -> Box<dyn ClientHook>;
    fn new_call(
        &self,
        interface_id: u64,
        method_id: u16,
        size_hint: Option<MessageSize>,
    ) -> Request<any_pointer::Owned, any_pointer::Owned>;

    fn call(
        &self,
        interface_id: u64,
        method_id: u16,
        params: Box<dyn ParamsHook>,
        results: Box<dyn ResultsHook>,
    ) -> crate::capability::Promise<(), crate::Error>;

    /// If this capability is associated with an rpc connection, then this method
    /// returns an identifier for that connection.
    fn get_brand(&self) -> usize;

    /// Returns a (locally) unique identifier for this capabilitiy.
    fn get_ptr(&self) -> usize;

    /// If this ClientHook is a promise that has already resolved, returns the inner, resolved version
    /// of the capability.  The caller may permanently replace this client with the resolved one if
    /// desired.  Returns null if the client isn't a promise or hasn't resolved yet -- use
    /// `whenMoreResolved()` to distinguish between them.
    fn get_resolved(&self) -> Option<Box<dyn ClientHook>>;

    /// If this client is a settled reference (not a promise), return nullptr.  Otherwise, return a
    /// promise that eventually resolves to a new client that is closer to being the final, settled
    /// client (i.e. the value eventually returned by `getResolved()`).  Calling this repeatedly
    /// should eventually produce a settled client.
    fn when_more_resolved(
        &self,
    ) -> Option<crate::capability::Promise<Box<dyn ClientHook>, crate::Error>>;

    /// Repeatedly calls whenMoreResolved() until it returns nullptr.
    fn when_resolved(&self) -> Promise<(), crate::Error>;
}

impl Clone for Box<dyn ClientHook> {
    fn clone(&self) -> Self {
        self.add_ref()
    }
}

pub trait ResultsHook {
    fn get(&mut self) -> crate::Result<any_pointer::Builder<'_>>;
    fn allow_cancellation(&self);
    fn tail_call(self: Box<Self>, request: Box<dyn RequestHook>) -> Promise<(), crate::Error>;
    fn direct_tail_call(
        self: Box<Self>,
        request: Box<dyn RequestHook>,
    ) -> (
        crate::capability::Promise<(), crate::Error>,
        Box<dyn PipelineHook>,
    );
}

pub trait ParamsHook {
    fn get(&self) -> crate::Result<crate::any_pointer::Reader<'_>>;
}

// Where should this live?
pub fn internal_get_typed_params<T>(typeless: Params<any_pointer::Owned>) -> Params<T> {
    Params {
        hook: typeless.hook,
        marker: ::core::marker::PhantomData,
    }
}

pub fn internal_get_typed_results<T>(typeless: Results<any_pointer::Owned>) -> Results<T> {
    Results {
        hook: typeless.hook,
        marker: ::core::marker::PhantomData,
    }
}

pub fn internal_get_untyped_results<T>(typeful: Results<T>) -> Results<any_pointer::Owned> {
    Results {
        hook: typeful.hook,
        marker: ::core::marker::PhantomData,
    }
}

pub trait PipelineHook {
    fn add_ref(&self) -> Box<dyn PipelineHook>;
    fn get_pipelined_cap(&self, ops: &[PipelineOp]) -> Box<dyn ClientHook>;

    /// Version of get_pipelined_cap() passing the array by move. May avoid a copy in some cases.
    /// Default implementation just calls the other version.
    fn get_pipelined_cap_move(&self, ops: Vec<PipelineOp>) -> Box<dyn ClientHook> {
        self.get_pipelined_cap(&ops)
    }
}

impl Clone for Box<dyn PipelineHook> {
    fn clone(&self) -> Self {
        self.add_ref()
    }
}

#[derive(Clone, Copy)]
pub enum PipelineOp {
    Noop,
    GetPointerField(u16),
}
