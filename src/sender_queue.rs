// Copyright (c) 2013-2016 Sandstorm Development Group, Inc. and contributors
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

use futures::{Future};
use futures::sync::oneshot;

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use capnp::capability::Promise;
use capnp::Error;

use std::collections::BTreeMap;

struct Inner<In, Out> {
    next_id: u64,
    map: BTreeMap<u64, (In, oneshot::Sender<Out>)>,
}

pub struct SenderQueue<In, Out> {
    inner: Rc<RefCell<Inner<In, Out>>>,
}

pub struct Remover<In, Out> {
    id: u64,
    inner: Weak<RefCell<Inner<In, Out>>>,
}

impl <In, Out> Drop for Remover<In, Out> {
    fn drop(&mut self) {
        // TODO
    }
}

impl <In, Out> SenderQueue<In, Out> {
    pub fn new() -> SenderQueue<In, Out> {
        SenderQueue {
            inner: Rc::new(RefCell::new(Inner {
                next_id: 0,
                map: BTreeMap::new(),
            })),
        }
    }

    pub fn push(&mut self, value: In) -> Promise<Out, Error> {
        let weak_inner = Rc::downgrade(&self.inner);
        let Inner { ref mut next_id, ref mut map, .. } = *self.inner.borrow_mut();
        let (tx, rx) = oneshot::channel();
        map.insert(*next_id, (value, tx));

        let remover = Remover {
            id: *next_id,
            inner: weak_inner,
        };

        *next_id += 1;

        unimplemented!()
    }
}
