// Copyright (c) 2016 Sandstorm Development Group, Inc. and contributors
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

use std::future::Future;

use capnp::Error;
use futures_io::AsyncWrite;

pub use crate::io::Sender;
use crate::io::{futures_io::Compat, serialize::AsOutputSegments};

/// Creates a new write queue that wraps the given `AsyncWrite`.
///
/// Returns `(sender, task)`, where `sender` can be used to push writes onto the
/// queue, and `task` is a future that performs the work of the writes. The queue
/// will run as long as `task` is polled, until either `sender.terminate()` is
/// called or `sender` and all of its clones are dropped.
pub fn write_queue<W, M>(writer: W) -> (Sender<M>, impl Future<Output = Result<(), Error>>)
where
    W: AsyncWrite + Unpin,
    M: AsOutputSegments,
{
    crate::io::write_queue(Compat::new(writer))
}
